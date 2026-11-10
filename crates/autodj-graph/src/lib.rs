//! Mix graph solver: order tracks for harmonic + tempo + energy-arc compatibility.

mod transition_model;

use autodj_core::{
    camelot_distance, stretch_clamp_excess, CamelotKey, TrackFeatures, TransitionClass,
};
use serde::{Deserialize, Serialize};

pub use transition_model::{
    feature_vector, predict_transition_class, predict_transition_class_edge,
    predict_transition_class_with_spectral, TransitionModel,
};

/// Genre policy knobs used by the solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenrePolicy {
    /// Genre id (`house`, `drum_and_bass`, …).
    pub id: String,
    /// Preferred BPM center.
    pub bpm_center: f32,
    /// Max absolute BPM delta before hard penalty.
    pub max_bpm_delta: f32,
    /// Default crossfade seconds.
    pub default_crossfade_secs: f32,
    /// Preferred transition class.
    pub preferred_transition: TransitionClass,
    /// Weight applied to the energy-arc term (higher = stronger open→peak→cool bias).
    #[serde(default = "default_energy_arc_weight")]
    pub energy_arc_weight: f32,
}

fn default_energy_arc_weight() -> f32 {
    1.0
}

/// Optional overrides from tracklist/chart aggregate stats (`policy_hints` JSON).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyHintOverride {
    /// Override preferred BPM center when set.
    #[serde(default)]
    pub bpm_center: Option<f32>,
    /// Override default crossfade seconds when set.
    #[serde(default)]
    pub default_crossfade_secs: Option<f32>,
}

impl GenrePolicy {
    /// Built-in defaults matching `Docs/genre-policy.md`.
    pub fn builtin(id: &str) -> Self {
        match id {
            "drum_and_bass" | "dnb" => Self {
                id: "drum_and_bass".into(),
                bpm_center: 174.0,
                max_bpm_delta: 6.0,
                default_crossfade_secs: 0.25,
                preferred_transition: TransitionClass::HardCut,
                energy_arc_weight: 1.0,
            },
            "hip_hop" | "rnb" => Self {
                id: "hip_hop".into(),
                bpm_center: 90.0,
                max_bpm_delta: 8.0,
                default_crossfade_secs: 1.0,
                preferred_transition: TransitionClass::EchoOut,
                energy_arc_weight: 1.0,
            },
            "trance" => Self {
                id: "trance".into(),
                bpm_center: 138.0,
                max_bpm_delta: 4.0,
                default_crossfade_secs: 16.0,
                preferred_transition: TransitionClass::FilterSweep,
                energy_arc_weight: 1.25,
            },
            "uk_garage" | "garage" => Self {
                id: "uk_garage".into(),
                bpm_center: 132.0,
                max_bpm_delta: 5.0,
                default_crossfade_secs: 8.0,
                preferred_transition: TransitionClass::EqSwap,
                energy_arc_weight: 1.0,
            },
            "open_format" => Self {
                id: "open_format".into(),
                bpm_center: 120.0,
                max_bpm_delta: 20.0,
                default_crossfade_secs: 4.0,
                preferred_transition: TransitionClass::Tremolo,
                energy_arc_weight: 0.5,
            },
            // house / techno / disco / default
            _ => Self {
                id: "house".into(),
                bpm_center: 124.0,
                max_bpm_delta: 6.0,
                default_crossfade_secs: 8.0,
                preferred_transition: TransitionClass::EqSwap,
                energy_arc_weight: 1.0,
            },
        }
    }

    /// Merge aggregate setcraft hints onto this policy (bpm_center / crossfade only).
    pub fn with_hints(&self, hint: &PolicyHintOverride) -> Self {
        let mut out = self.clone();
        if let Some(bpm) = hint.bpm_center {
            out.bpm_center = bpm;
        }
        if let Some(xfade) = hint.default_crossfade_secs {
            out.default_crossfade_secs = xfade;
        }
        out
    }
}

/// Load a genre→hints map from JSON (`{ "house": { "bpm_center": 125.0, … }, … }`).
pub fn load_policy_hints_map(
    raw: &str,
) -> Result<std::collections::HashMap<String, PolicyHintOverride>, serde_json::Error> {
    serde_json::from_str(raw)
}

/// Position in a setcraft energy arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcPhase {
    /// First ~25% of the set — prefer rising energy.
    Opening,
    /// Next ~25% — continue building.
    Build,
    /// Next ~25% — prefer peak energy arrivals.
    Peak,
    /// Final ~25% — prefer cooling down.
    Cooldown,
}

/// Map a placement index into an [`ArcPhase`] for a set of `len` tracks.
pub fn arc_phase(index: usize, len: usize) -> ArcPhase {
    if len == 0 {
        return ArcPhase::Opening;
    }
    let t = index as f32 / len as f32;
    if t < 0.25 {
        ArcPhase::Opening
    } else if t < 0.5 {
        ArcPhase::Build
    } else if t < 0.75 {
        ArcPhase::Peak
    } else {
        ArcPhase::Cooldown
    }
}

/// Soft penalty for genre-forbidden style knobs (e.g. long blends in DnB).
pub fn soft_style_penalty(policy: &GenrePolicy, crossfade_secs: f32) -> f32 {
    if policy.id == "drum_and_bass" && crossfade_secs > 1.0 {
        5.0
    } else {
        0.0
    }
}

/// Soft cost when a candidate transition class mismatches the genre preference.
pub fn policy_transition_penalty(class: TransitionClass, policy: &GenrePolicy) -> f32 {
    if class == policy.preferred_transition {
        0.0
    } else {
        2.0
    }
}

/// Weight for [`stretch_clamp_excess`] in pairwise cost (outside ±15% WSOLA clamp).
const STRETCH_EXCESS_WEIGHT: f32 = 12.0;

/// Resolve the transition class the solver would use for this edge.
pub fn resolve_transition_class(
    a: &TrackFeatures,
    b: &TrackFeatures,
    policy: &GenrePolicy,
    model: Option<&TransitionModel>,
) -> TransitionClass {
    if let Some(m) = model {
        predict_transition_class_edge(
            m,
            a,
            b,
            camelot_distance(a.key, b.key),
            &policy.id,
        )
    } else {
        policy.preferred_transition
    }
}

/// Scale genre default crossfade length by transition class.
///
/// - [`TransitionClass::HardCut`] → `0` (true abut cut)
/// - [`TransitionClass::EchoOut`] → short tail (`min(policy, max(1s, 2 beats))`)
/// - blends / EQ / filter / tremolo → genre `default_crossfade_secs`
pub fn crossfade_secs_for_class(class: TransitionClass, policy: &GenrePolicy, bpm: f32) -> f32 {
    let policy_default = policy.default_crossfade_secs.max(0.0);
    match class {
        TransitionClass::HardCut => 0.0,
        TransitionClass::EchoOut => {
            let beat = 60.0 / bpm.max(1.0);
            let two_beats = 2.0 * beat;
            policy_default.min(1.0_f32.max(two_beats))
        }
        TransitionClass::EqSwap
        | TransitionClass::FilterSweep
        | TransitionClass::LinearCrossfade
        | TransitionClass::Tremolo => policy_default,
    }
}

/// Edge decision between two consecutive tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDecision {
    /// From track index in the solved order (index into input features after permute).
    pub from: usize,
    /// To track index.
    pub to: usize,
    /// Chosen transition class.
    pub class: TransitionClass,
    /// Crossfade duration seconds.
    pub crossfade_secs: f32,
    /// Pair cost used by the solver.
    pub cost: f32,
}

/// Solved mix order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixSolution {
    /// Permutation of input track indices.
    pub order: Vec<usize>,
    /// Transition decisions along the path.
    pub transitions: Vec<TransitionDecision>,
    /// Total path cost.
    pub total_cost: f32,
}

/// Pairwise transition cost without arc context (Opening-equivalent energy term).
pub fn pair_cost(a: &TrackFeatures, b: &TrackFeatures, policy: &GenrePolicy) -> f32 {
    pair_cost_at(a, b, policy, ArcPhase::Opening)
}

/// Pairwise transition cost (lower is better) with setcraft arc phase.
pub fn pair_cost_at(
    a: &TrackFeatures,
    b: &TrackFeatures,
    policy: &GenrePolicy,
    phase: ArcPhase,
) -> f32 {
    pair_cost_edge(a, b, policy, phase, None)
}

/// Pairwise cost including stretch-clamp excess and resolved transition-class penalty.
pub fn pair_cost_edge(
    a: &TrackFeatures,
    b: &TrackFeatures,
    policy: &GenrePolicy,
    phase: ArcPhase,
    model: Option<&TransitionModel>,
) -> f32 {
    let key_cost = camelot_distance(a.key, b.key) as f32;
    let bpm_delta = (a.bpm - b.bpm).abs();
    let bpm_cost = if bpm_delta > policy.max_bpm_delta {
        10.0 + bpm_delta
    } else {
        bpm_delta * 0.35
    };
    let center_pull = ((a.bpm + b.bpm) * 0.5 - policy.bpm_center).abs() * 0.02;
    let energy = energy_arc_cost(a.loudness_db, b.loudness_db, phase) * policy.energy_arc_weight;
    let class = resolve_transition_class(a, b, policy, model);
    let xfade = crossfade_secs_for_class(class, policy, a.bpm);
    let style = soft_style_penalty(policy, xfade) + policy_transition_penalty(class, policy);
    let stretch = STRETCH_EXCESS_WEIGHT * stretch_clamp_excess(a.bpm, b.bpm);
    key_cost + bpm_cost + energy + center_pull + style + stretch
}

fn energy_arc_cost(loud_a: f32, loud_b: f32, phase: ArcPhase) -> f32 {
    let delta = loud_b - loud_a;
    match phase {
        ArcPhase::Opening | ArcPhase::Build => {
            if delta >= -0.5 {
                // Rising or flat — small cost.
                delta.abs() * 0.02
            } else {
                // Dropping while building — penalize.
                0.8 + (-delta) * 0.15
            }
        }
        ArcPhase::Peak => {
            // Prefer louder arrivals; quiet arrivals cost more.
            if loud_b >= -12.0 {
                (loud_b + 8.0).abs() * 0.02
            } else {
                0.6 + (-12.0 - loud_b) * 0.1
            }
        }
        ArcPhase::Cooldown => {
            if delta <= 0.5 {
                delta.abs() * 0.02
            } else {
                0.8 + delta * 0.15
            }
        }
    }
}

/// Choose a quiet opener within the genre BPM band (fallback: global quietest).
pub fn select_opener(tracks: &[TrackFeatures], policy: &GenrePolicy) -> usize {
    if tracks.is_empty() {
        return 0;
    }
    let in_band: Vec<usize> = tracks
        .iter()
        .enumerate()
        .filter(|(_, t)| (t.bpm - policy.bpm_center).abs() <= policy.max_bpm_delta)
        .map(|(i, _)| i)
        .collect();
    let pool = if in_band.is_empty() {
        (0..tracks.len()).collect::<Vec<_>>()
    } else {
        in_band
    };
    pool.into_iter()
        .min_by(|&i, &j| {
            tracks[i]
                .loudness_db
                .partial_cmp(&tracks[j].loudness_db)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0)
}

/// Keep playlist input order (no energy-arc / harmonic reordering).
///
/// Used for Rekordbox exports and `--preserve-order`. Transitions use the genre
/// preferred class (or `model` when provided).
pub fn preserve_order(
    tracks: &[TrackFeatures],
    policy: &GenrePolicy,
    model: Option<&TransitionModel>,
) -> MixSolution {
    let n = tracks.len();
    if n == 0 {
        return MixSolution {
            order: vec![],
            transitions: vec![],
            total_cost: 0.0,
        };
    }
    let order: Vec<usize> = (0..n).collect();
    let mut transitions = Vec::new();
    let mut total = 0.0f32;
    for w in order.windows(2) {
        let (from, to) = (w[0], w[1]);
        let class = resolve_transition_class(&tracks[from], &tracks[to], policy, model);
        let cost = pair_cost_edge(
            &tracks[from],
            &tracks[to],
            policy,
            ArcPhase::Opening,
            model,
        );
        total += cost;
        transitions.push(TransitionDecision {
            from,
            to,
            class,
            crossfade_secs: crossfade_secs_for_class(class, policy, tracks[from].bpm),
            cost,
        });
    }
    MixSolution {
        order,
        transitions,
        total_cost: total,
    }
}

/// Greedy nearest-neighbor solver with optional fixed start index.
///
/// When `start` is `None`, picks a quiet opener via [`select_opener`].
/// When `model` is `Some`, each edge class comes from [`predict_transition_class`];
/// otherwise [`GenrePolicy::preferred_transition`] is used.
pub fn solve_order(
    tracks: &[TrackFeatures],
    policy: &GenrePolicy,
    start: Option<usize>,
    model: Option<&TransitionModel>,
) -> MixSolution {
    let n = tracks.len();
    if n == 0 {
        return MixSolution {
            order: vec![],
            transitions: vec![],
            total_cost: 0.0,
        };
    }
    let mut remaining: Vec<usize> = (0..n).collect();
    let mut order = Vec::with_capacity(n);
    let mut transitions = Vec::new();
    let mut total = 0.0f32;

    let first = start.unwrap_or_else(|| select_opener(tracks, policy)).min(n - 1);
    order.push(first);
    remaining.retain(|&i| i != first);

    while !remaining.is_empty() {
        let last = *order.last().unwrap();
        let phase = arc_phase(order.len(), n);
        let mut best_i = 0usize;
        let mut best_cost = f32::MAX;
        for (ri, &cand) in remaining.iter().enumerate() {
            let c = pair_cost_edge(&tracks[last], &tracks[cand], policy, phase, model);
            if c < best_cost {
                best_cost = c;
                best_i = ri;
            }
        }
        let next = remaining.remove(best_i);
        let class = resolve_transition_class(&tracks[last], &tracks[next], policy, model);
        transitions.push(TransitionDecision {
            from: last,
            to: next,
            class,
            crossfade_secs: crossfade_secs_for_class(class, policy, tracks[last].bpm),
            cost: best_cost,
        });
        total += best_cost;
        order.push(next);
    }

    MixSolution {
        order,
        transitions,
        total_cost: total,
    }
}

/// Suggest a Camelot-compatible next key from `current`.
pub fn suggest_next_keys(current: CamelotKey) -> Vec<CamelotKey> {
    let mut out = vec![current];
    out.push(CamelotKey {
        number: current.number,
        minor: !current.minor,
    });
    let up = if current.number == 12 {
        1
    } else {
        current.number + 1
    };
    let down = if current.number == 1 {
        12
    } else {
        current.number - 1
    };
    out.push(CamelotKey {
        number: up,
        minor: current.minor,
    });
    out.push(CamelotKey {
        number: down,
        minor: current.minor,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use autodj_core::{AnalysisQuality, CamelotKey, SectionMarks, TrackFeatures};

    fn feat(bpm: f32, key: &str, loud: f32) -> TrackFeatures {
        TrackFeatures {
            path: None,
            bpm,
            key: CamelotKey::parse(key).unwrap(),
            loudness_db: loud,
            duration_secs: 8.0,
            sample_rate: 44100,
            sections: SectionMarks {
                intro_end: 1.0,
                drop_start: 3.0,
                outro_start: 6.0,
            },
            spectral_centroid_hz: 1000.0,
            beat_offset_secs: 0.0,
            quality: AnalysisQuality::default(),
            bars_per_phrase: 0,
        }
    }

    #[test]
    fn prefers_harmonic_neighbor() {
        let tracks = vec![
            feat(124.0, "8A", -10.0),
            feat(174.0, "1A", -9.0), // far
            feat(125.0, "9A", -9.5),
        ];
        let policy = GenrePolicy::builtin("house");
        let sol = solve_order(&tracks, &policy, Some(0), None);
        assert_eq!(sol.order[0], 0);
        assert_eq!(sol.order[1], 2, "order={:?}", sol.order);
    }

    #[test]
    fn preserve_order_keeps_input_indices() {
        let tracks = vec![
            feat(124.0, "8A", -10.0),
            feat(174.0, "1A", -9.0),
            feat(125.0, "9A", -9.5),
        ];
        let policy = GenrePolicy::builtin("house");
        let sol = preserve_order(&tracks, &policy, None);
        assert_eq!(sol.order, vec![0, 1, 2]);
        assert_eq!(sol.transitions.len(), 2);
        assert_eq!(sol.transitions[0].class, policy.preferred_transition);
    }

    #[test]
    fn arc_phases_cover_set() {
        assert_eq!(arc_phase(0, 8), ArcPhase::Opening);
        assert_eq!(arc_phase(2, 8), ArcPhase::Build);
        assert_eq!(arc_phase(4, 8), ArcPhase::Peak);
        assert_eq!(arc_phase(7, 8), ArcPhase::Cooldown);
    }

    #[test]
    fn energy_arc_order_rises_then_falls() {
        // Same key/BPM so loudness arc dominates; free start picks quietest.
        let tracks = vec![
            feat(124.0, "8A", -18.0), // quiet opener
            feat(124.0, "8A", -14.0),
            feat(124.0, "8A", -8.0),  // peak
            feat(124.0, "8A", -16.0), // cool
        ];
        let policy = GenrePolicy::builtin("house");
        let sol = solve_order(&tracks, &policy, None, None);
        assert_eq!(sol.order[0], 0, "quiet opener, got {:?}", sol.order);
        // Second should be louder than first (build).
        assert!(
            tracks[sol.order[1]].loudness_db >= tracks[sol.order[0]].loudness_db - 0.5,
            "order={:?}",
            sol.order
        );
        // Peak track (index 2, loudest) should appear before the final cooldown placement.
        let peak_pos = sol.order.iter().position(|&i| i == 2).unwrap();
        assert!(peak_pos < 3, "peak should not be last, order={:?}", sol.order);
    }

    #[test]
    fn genre_policy_weights() {
        let trance = GenrePolicy::builtin("trance");
        let open = GenrePolicy::builtin("open_format");
        let dnb = GenrePolicy::builtin("drum_and_bass");
        assert!((trance.energy_arc_weight - 1.25).abs() < 1e-5);
        assert!((open.energy_arc_weight - 0.5).abs() < 1e-5);
        assert_eq!(dnb.preferred_transition, TransitionClass::HardCut);
        assert_eq!(open.preferred_transition, TransitionClass::Tremolo);
        assert!(soft_style_penalty(&dnb, 8.0) >= 5.0);
        assert_eq!(soft_style_penalty(&dnb, 0.25), 0.0);
    }

    #[test]
    fn select_opener_picks_quietest_in_band() {
        let tracks = vec![
            feat(124.0, "8A", -8.0),
            feat(124.0, "8A", -20.0),
            feat(180.0, "8A", -30.0), // out of house band
        ];
        let policy = GenrePolicy::builtin("house");
        assert_eq!(select_opener(&tracks, &policy), 1);
    }

    #[test]
    fn with_hints_overrides_bpm_and_crossfade() {
        let base = GenrePolicy::builtin("house");
        let hinted = base.with_hints(&PolicyHintOverride {
            bpm_center: Some(126.5),
            default_crossfade_secs: Some(12.0),
        });
        assert!((hinted.bpm_center - 126.5).abs() < 1e-5);
        assert!((hinted.default_crossfade_secs - 12.0).abs() < 1e-5);
        assert_eq!(hinted.preferred_transition, base.preferred_transition);
        assert!((hinted.energy_arc_weight - base.energy_arc_weight).abs() < 1e-5);
    }

    #[test]
    fn load_policy_hints_map_parses_demo_shape() {
        let raw = r#"{"house":{"bpm_center":124.5,"default_crossfade_secs":12.0}}"#;
        let map = load_policy_hints_map(raw).unwrap();
        let h = map.get("house").unwrap();
        assert!((h.bpm_center.unwrap() - 124.5).abs() < 1e-5);
        assert!((h.default_crossfade_secs.unwrap() - 12.0).abs() < 1e-5);
    }

    #[test]
    fn committed_transition_model_predicts_sensible_classes() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../python/autodj_train/models/artifacts/transition_model.json"
        );
        let raw = std::fs::read_to_string(path).expect("committed transition_model.json");
        let model = TransitionModel::from_json(&raw).unwrap();
        assert!(
            model.version >= 3,
            "expected Softmax v3 artifact, got version {}",
            model.version
        );
        assert_eq!(model.mean.len(), 9 + model.genres.len());
        assert!(
            model.classes.iter().any(|c| c == "tremolo"),
            "expected tremolo in Softmax classes {:?}",
            model.classes
        );
        let dnb = predict_transition_class(&model, 174.0, 175.0, 1, 0.0, "drum_and_bass");
        assert_eq!(dnb, TransitionClass::HardCut);
        let house = predict_transition_class_with_spectral(
            &model,
            124.0,
            125.0,
            1,
            0.5,
            "house",
            100.0,
            8,
            8,
        );
        assert!(
            matches!(
                house,
                TransitionClass::EqSwap
                    | TransitionClass::FilterSweep
                    | TransitionClass::LinearCrossfade
            ),
            "unexpected house class {house:?}"
        );
        // With model, solve_order should not panic and may differ from preferred_transition.
        let tracks = vec![
            feat(124.0, "8A", -12.0),
            feat(125.0, "9A", -11.0),
        ];
        let policy = GenrePolicy::builtin("house");
        let sol = solve_order(&tracks, &policy, Some(0), Some(&model));
        assert_eq!(sol.transitions.len(), 1);
        assert_ne!(sol.transitions[0].class, TransitionClass::HardCut);
    }

    #[test]
    fn non_preferred_class_raises_policy_penalty() {
        let house = GenrePolicy::builtin("house");
        assert_eq!(
            policy_transition_penalty(TransitionClass::EqSwap, &house),
            0.0
        );
        assert_eq!(
            policy_transition_penalty(TransitionClass::HardCut, &house),
            2.0
        );
    }

    #[test]
    fn crossfade_secs_scales_by_class() {
        let house = GenrePolicy::builtin("house");
        let dnb = GenrePolicy::builtin("drum_and_bass");
        let hip = GenrePolicy::builtin("hip_hop");
        let trance = GenrePolicy::builtin("trance");
        assert_eq!(
            crossfade_secs_for_class(TransitionClass::HardCut, &dnb, 174.0),
            0.0
        );
        assert!(
            (crossfade_secs_for_class(TransitionClass::EqSwap, &house, 128.0) - 8.0).abs() < 1e-5
        );
        assert!(
            (crossfade_secs_for_class(TransitionClass::FilterSweep, &trance, 138.0) - 16.0).abs()
                < 1e-5
        );
        // hip_hop EchoOut policy 1.0 stays ~1s at 90 BPM (2 beats ≈ 1.33 → max(1,1.33)=1.33 → min(1,1.33)=1).
        let hip_xf = crossfade_secs_for_class(TransitionClass::EchoOut, &hip, 90.0);
        assert!((hip_xf - 1.0).abs() < 1e-5, "hip_hop echo got {hip_xf}");
        // Softmax echo on house: short tail (~1s), not 8s blend.
        let house_echo = crossfade_secs_for_class(TransitionClass::EchoOut, &house, 128.0);
        assert!((house_echo - 1.0).abs() < 1e-5, "house echo got {house_echo}");
        let tracks = vec![feat(174.0, "8A", -12.0), feat(175.0, "8A", -11.0)];
        let sol = preserve_order(&tracks, &dnb, None);
        assert_eq!(sol.transitions[0].crossfade_secs, 0.0);
    }

    #[test]
    fn stretch_excess_raises_pair_cost() {
        let policy = GenrePolicy::builtin("open_format");
        let a = feat(128.0, "8A", -12.0);
        let in_range = feat(140.0, "8A", -11.0);
        let clamped = feat(155.0, "8A", -11.0);
        let c_near = pair_cost_edge(&a, &in_range, &policy, ArcPhase::Opening, None);
        let c_far = pair_cost_edge(&a, &clamped, &policy, ArcPhase::Opening, None);
        assert!(
            c_far > c_near,
            "clamp edge should cost more: near={c_near} far={c_far}"
        );
        assert!(stretch_clamp_excess(128.0, 155.0) > stretch_clamp_excess(128.0, 140.0));
    }

    #[test]
    fn solve_avoids_immediate_clamp_neighbor() {
        let policy = GenrePolicy::builtin("open_format");
        let tracks = vec![
            feat(128.0, "8A", -14.0), // 0 start
            feat(155.0, "8A", -10.0), // 1 clamp
            feat(130.0, "8A", -12.0), // 2 safe
        ];
        let sol = solve_order(&tracks, &policy, Some(0), None);
        assert_eq!(sol.order[0], 0);
        assert_eq!(
            sol.order[1], 2,
            "should pick safe 130 BPM before 155 clamp, order={:?}",
            sol.order
        );
    }
}
