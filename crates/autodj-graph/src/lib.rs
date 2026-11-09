//! Mix graph solver: order tracks for harmonic + tempo + energy-arc compatibility.

use autodj_core::{camelot_distance, CamelotKey, TrackFeatures, TransitionClass};
use serde::{Deserialize, Serialize};

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
            },
            "hip_hop" | "rnb" => Self {
                id: "hip_hop".into(),
                bpm_center: 90.0,
                max_bpm_delta: 8.0,
                default_crossfade_secs: 1.0,
                preferred_transition: TransitionClass::EchoOut,
            },
            "trance" => Self {
                id: "trance".into(),
                bpm_center: 138.0,
                max_bpm_delta: 4.0,
                default_crossfade_secs: 16.0,
                preferred_transition: TransitionClass::FilterSweep,
            },
            "uk_garage" | "garage" => Self {
                id: "uk_garage".into(),
                bpm_center: 132.0,
                max_bpm_delta: 5.0,
                default_crossfade_secs: 8.0,
                preferred_transition: TransitionClass::EqSwap,
            },
            "open_format" => Self {
                id: "open_format".into(),
                bpm_center: 120.0,
                max_bpm_delta: 20.0,
                default_crossfade_secs: 4.0,
                preferred_transition: TransitionClass::LinearCrossfade,
            },
            // house / techno / disco / default
            _ => Self {
                id: "house".into(),
                bpm_center: 124.0,
                max_bpm_delta: 6.0,
                default_crossfade_secs: 8.0,
                preferred_transition: TransitionClass::EqSwap,
            },
        }
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

/// Pairwise transition cost (lower is better).
pub fn pair_cost(a: &TrackFeatures, b: &TrackFeatures, policy: &GenrePolicy) -> f32 {
    let key_cost = camelot_distance(a.key, b.key) as f32;
    let bpm_delta = (a.bpm - b.bpm).abs();
    let bpm_cost = if bpm_delta > policy.max_bpm_delta {
        10.0 + bpm_delta
    } else {
        bpm_delta * 0.35
    };
    // Prefer rising then falling energy when possible (simple loudness proxy).
    let energy_cost = if b.loudness_db + 0.5 < a.loudness_db {
        0.15
    } else {
        (b.loudness_db - a.loudness_db).abs() * 0.05
    };
    let center_pull = ((a.bpm + b.bpm) * 0.5 - policy.bpm_center).abs() * 0.02;
    key_cost + bpm_cost + energy_cost + center_pull
}

/// Greedy nearest-neighbor solver with optional fixed start index.
pub fn solve_order(tracks: &[TrackFeatures], policy: &GenrePolicy, start: Option<usize>) -> MixSolution {
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

    let first = start.unwrap_or(0).min(n - 1);
    order.push(first);
    remaining.retain(|&i| i != first);

    while !remaining.is_empty() {
        let last = *order.last().unwrap();
        let mut best_i = 0usize;
        let mut best_cost = f32::MAX;
        for (ri, &cand) in remaining.iter().enumerate() {
            let c = pair_cost(&tracks[last], &tracks[cand], policy);
            if c < best_cost {
                best_cost = c;
                best_i = ri;
            }
        }
        let next = remaining.remove(best_i);
        transitions.push(TransitionDecision {
            from: last,
            to: next,
            class: policy.preferred_transition,
            crossfade_secs: policy.default_crossfade_secs,
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
    use autodj_core::{CamelotKey, SectionMarks, TrackFeatures};

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
        let sol = solve_order(&tracks, &policy, Some(0));
        assert_eq!(sol.order[0], 0);
        assert_eq!(sol.order[1], 2, "order={:?}", sol.order);
    }
}
