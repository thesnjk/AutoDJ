//! AutoDJ CLI — analyze libraries and render offline mixes.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use autodj_core::{
    adjacent_flux_ratio, analyze_path, bars_per_phrase_for_genre, bass_conflict_pass,
    beat_phase_gate_pass, camelot_distance, cue_edge_phrase_aligned, estimate_bars_per_phrase,
    join_continuity_pass, max_join_flux_ratio, mean_beat_phase_offset_ms, measure_loudness,
    mix_buffers_with_meta, normalize_loudness, parse_rekordbox_xml, peak_gate_pass,
    plan_segment_cues, sections_drop_ordered, snap_to_beat_grid, spectral_continuity_pass,
    stretch_clamp_excess, tempo_stretch_gate_pass, within_lufs_tolerance, BeatPhaseCue,
    CamelotKey, CrossfadeParams, CueOverrides, MixPlan, MixSegment, TrackFeatures,
    TransitionClass, DEFAULT_BASS_OVERLAP_LIMIT,
    DEFAULT_BEAT_PHASE_LIMIT_MS, DEFAULT_JOIN_FLUX_RATIO, DEFAULT_MAX_FLUX_RATIO,
    DEFAULT_TARGET_LUFS, DEFAULT_TRUE_PEAK_LIMIT, LUFS_TOLERANCE_LU,
};
use autodj_graph::{
    apply_transition_overrides, build_remix_order, crossfade_secs_for_class, load_policy_hints_map,
    predict_transition_class_edge, preserve_order, rank_next_matches, solve_order,
    suggest_next_keys, GenrePolicy, TransitionModel,
};
use autodj_io::{read_wav, write_wav};
use clap::{Parser, Subcommand};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "autodj", version, about = "Automatic DJ mix engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze audio files into a SQLite feature cache.
    Analyze {
        /// Input file or directory.
        #[arg(long)]
        input: PathBuf,
        /// SQLite database path.
        #[arg(long, default_value = "features.sqlite")]
        db: PathBuf,
    },
    /// Mix a playlist JSON or Rekordbox XML (or golden smoke) to WAV.
    Mix {
        /// Playlist JSON or Rekordbox DJ_PLAYLISTS XML path.
        #[arg(long)]
        playlist: Option<PathBuf>,
        /// When XML has multiple playlists, select by NODE name.
        #[arg(long)]
        playlist_name: Option<String>,
        /// Keep playlist order (skip energy-arc / harmonic reorder). Useful for Rekordbox exports.
        #[arg(long, default_value_t = false)]
        preserve_order: bool,
        /// Output WAV path.
        #[arg(long, default_value = "mix.wav")]
        out: PathBuf,
        /// Genre policy id.
        #[arg(long, default_value = "house")]
        genre: String,
        /// Built-in golden suite: `smoke`, `rekordbox`, `transitions`, `transition_model`, `cue_overrides`, or `transition_overrides`.
        #[arg(long)]
        golden: Option<String>,
        /// Feature DB (optional; analyzes on the fly if missing).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Strict gates + fixed defaults (beat-phase / peak hard-fail; default LUFS path).
        #[arg(long, default_value_t = true)]
        deterministic: bool,
        /// Relax MixReport hard gates (overrides `--deterministic`).
        #[arg(long, default_value_t = false)]
        no_deterministic: bool,
        /// Target integrated loudness in LUFS (EBU R 128 / BS.1770). Use -23 for broadcast.
        #[arg(long, default_value_t = DEFAULT_TARGET_LUFS)]
        lufs: f64,
        /// Skip loudness normalization (peak-limit only).
        #[arg(long, default_value_t = false)]
        no_loudness_norm: bool,
        /// Optional genre policy hints JSON from tracklist aggregate (`autodj-train aggregate-tracklists --hints-out`).
        #[arg(long)]
        policy_hints: Option<PathBuf>,
        /// Softmax transition model JSON (default: committed artifact for ordinary mixes).
        #[arg(long)]
        transition_model: Option<PathBuf>,
        /// Skip Softmax; use genre `preferred_transition` per edge (goldens do this by default).
        #[arg(long, default_value_t = false)]
        no_transition_model: bool,
    },
    /// Rank next-track matches from a folder for Automix Matching.
    Suggest {
        /// Library folder (wav/flac).
        #[arg(long)]
        folder: PathBuf,
        /// Current / from track path.
        #[arg(long)]
        from: PathBuf,
        /// Max candidates to return.
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// Genre policy id.
        #[arg(long, default_value = "house")]
        genre: String,
        /// Feature DB (optional; analyzes on the fly if missing).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Softmax transition model JSON.
        #[arg(long)]
        transition_model: Option<PathBuf>,
        /// Skip Softmax; use genre preferred_transition.
        #[arg(long, default_value_t = false)]
        no_transition_model: bool,
        /// Optional genre policy hints JSON.
        #[arg(long)]
        policy_hints: Option<PathBuf>,
        /// Placement index of the from-track in the current set (energy-arc phase).
        #[arg(long, default_value_t = 0)]
        position: usize,
        /// Set length for energy-arc phase (defaults to folder track count).
        #[arg(long)]
        set_len: Option<usize>,
    },
    /// Remix a folder set: keep played prefix, pin next, solve remainder, preserve-order mix.
    Remix {
        /// Library folder (wav/flac) used to resolve remaining tracks.
        #[arg(long)]
        folder: PathBuf,
        /// Already-played paths including the current track (repeatable).
        #[arg(long = "prefix", required = true)]
        prefix: Vec<PathBuf>,
        /// Forced next track path.
        #[arg(long)]
        next: PathBuf,
        /// Output WAV path.
        #[arg(long, default_value = "mix.wav")]
        out: PathBuf,
        /// Genre policy id.
        #[arg(long, default_value = "house")]
        genre: String,
        /// Feature DB (optional).
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        deterministic: bool,
        #[arg(long, default_value_t = false)]
        no_deterministic: bool,
        #[arg(long, default_value_t = DEFAULT_TARGET_LUFS)]
        lufs: f64,
        #[arg(long, default_value_t = false)]
        no_loudness_norm: bool,
        #[arg(long)]
        policy_hints: Option<PathBuf>,
        #[arg(long)]
        transition_model: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        no_transition_model: bool,
    },
}

#[derive(Debug, Serialize)]
struct SuggestCandidateReport {
    path: String,
    cost: f32,
    key: String,
    bpm: f32,
    class: TransitionClass,
    on_route: bool,
}

#[derive(Debug, Serialize)]
struct SuggestReport {
    from: String,
    candidates: Vec<SuggestCandidateReport>,
}

#[derive(Debug, Deserialize)]
struct PlaylistFile {
    tracks: Vec<PlaylistTrack>,
    #[serde(default)]
    genre: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistTrack {
    path: PathBuf,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    bpm: Option<f32>,
    #[serde(default)]
    beat_offset_secs: Option<f64>,
    #[serde(default)]
    intro_end: Option<f64>,
    #[serde(default)]
    outro_start: Option<f64>,
    /// Absolute mix-in within the track (seconds). Overrides section heuristics.
    #[serde(default)]
    mix_in_secs: Option<f64>,
    /// Absolute mix-out within the track (seconds). Overrides section heuristics.
    #[serde(default)]
    mix_out_secs: Option<f64>,
    /// Transition class for the edge from this track to the next (Softmax/policy override).
    #[serde(default)]
    transition_out: Option<TransitionClass>,
    /// Optional phrase length override (`4` / `8` / `16`).
    #[serde(default)]
    bars_per_phrase: Option<u32>,
}

/// Minimum analyzed BPM confidence for prep-trusted tracks without a BPM override.
const DEFAULT_MIN_BPM_CONFIDENCE: f32 = 0.25;

#[derive(Debug, Serialize)]
struct MixReport {
    order: Vec<String>,
    total_cost: f32,
    genre: String,
    out: String,
    /// Post-render integrated loudness (LUFS), when measured.
    integrated_lufs: Option<f64>,
    /// Target LUFS used for normalization, if any.
    target_lufs: Option<f64>,
    /// Whether measured loudness is within ±[`LUFS_TOLERANCE_LU`] of target.
    loudness_gate_pass: Option<bool>,
    /// Per-track phrase/section cue windows used for the render.
    cues: Vec<CueReport>,
    /// True when cue planner ran (phrase-aligned mix-in/out).
    cue_phrase_aligned: bool,
    /// True when any playlist track supplied `mix_in_secs` / `mix_out_secs`.
    cue_mix_override_applied: bool,
    /// True when any playlist track supplied `transition_out`.
    transition_override_applied: bool,
    /// True when every edge used ordered section marks (`intro ≤ drop ≤ outro`).
    cue_drop_aware: bool,
    /// True when every cue used a resolved phrase length in `{4,8,16}`.
    per_track_phrase_applied: bool,
    /// Per-track analysis prep summary (confidence + override flags).
    prep_tracks: Vec<PrepTrackReport>,
    /// Whether every track is prep-trusted (analysis or BPM override).
    prep_quality_gate_pass: bool,
    /// True when any adjacent pair used WSOLA tempo blend (|ΔBPM| ≥ 0.5).
    tempo_blend_applied: bool,
    /// Adjacent spectral-flux ratio (max/median); lower is smoother.
    spectral_flux_ratio: f32,
    /// Whether spectral continuity is within genre default bounds.
    spectral_continuity_pass: bool,
    /// Worst join-local hop-RMS ratio across crossfade neighborhoods.
    max_join_flux_ratio: f32,
    /// Whether join-local flux is under [`DEFAULT_JOIN_FLUX_RATIO`].
    join_continuity_pass: bool,
    /// Mean absolute downbeat join offset across same-tempo edges (ms).
    beat_phase_mean_ms: f32,
    /// Whether [`beat_phase_mean_ms`] is under the 50 ms rubric limit.
    beat_phase_gate_pass: bool,
    /// Post-render true-peak (linear full-scale), when measured.
    true_peak: Option<f64>,
    /// Whether true-peak ≤ 0.99.
    peak_gate_pass: Option<bool>,
    /// Whether playlist order was preserved (Rekordbox / `--preserve-order`).
    preserve_order: bool,
    /// Worst EqSwap concurrent low-band ownership product (0 when no EqSwap joins).
    max_bass_overlap: f32,
    /// Whether [`max_bass_overlap`] is under [`DEFAULT_BASS_OVERLAP_LIMIT`].
    bass_conflict_gate_pass: bool,
    /// Per-edge transition decisions (empty for single-track mixes).
    transitions: Vec<TransitionEdgeReport>,
    /// Worst WSOLA stretch-factor clamp excess across adjacent edges (0 = no clamp).
    max_stretch_clamp_excess: f32,
    /// Whether [`max_stretch_clamp_excess`] is zero (no silent ±15% clip).
    tempo_stretch_gate_pass: bool,
    /// Mix-timeline seconds of each transition join (incl. hard cuts), for Mix Now seek.
    join_timeline_secs: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct PrepTrackReport {
    track: String,
    sparse_audio: bool,
    bpm_confidence: f32,
    key_confidence: f32,
    bpm_octave_corrected: bool,
    bpm_overridden: bool,
    key_overridden: bool,
    beat_offset_overridden: bool,
    /// True when analysis is strong enough or BPM was overridden.
    trusted: bool,
}

#[derive(Debug, Serialize)]
struct TransitionEdgeReport {
    from: usize,
    to: usize,
    class: TransitionClass,
    /// Policy / solver crossfade seconds before cue snap.
    policy_crossfade_secs: f32,
    /// Cue-planned crossfade seconds used for render.
    crossfade_secs: f32,
    /// Camelot key of the outgoing track.
    key_from: String,
    /// Camelot key of the incoming track.
    key_to: String,
    /// Harmonic distance on the Camelot wheel.
    camelot_distance: u8,
    /// Absolute BPM delta between the edge endpoints.
    bpm_delta: f32,
    /// True when `key_to` is in [`suggest_next_keys`] for `key_from`.
    key_on_route: bool,
    /// Solver pairwise edge cost (lower is better).
    edge_cost: f32,
}

#[derive(Debug, Serialize)]
struct CueReport {
    track: String,
    play_start_secs: f64,
    play_end_secs: f64,
    crossfade_secs: f32,
    bpm: f32,
    /// Camelot key used for this cue (analysis / playlist override).
    key: String,
    /// BPM of the next track when a transition follows; else null.
    bpm_to: Option<f32>,
    /// Resolved phrase length in bars used for cue snap.
    bars_per_phrase: u32,
    /// Seconds into the rendered mix WAV where this track begins (solved order).
    mix_timeline_start_secs: f64,
    /// Seconds into the rendered mix WAV where this track ends (next join or mix end).
    mix_timeline_end_secs: f64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Analyze { input, db } => cmd_analyze(&input, &db),
        Commands::Suggest {
            folder,
            from,
            top,
            genre,
            db,
            transition_model,
            no_transition_model,
            policy_hints,
            position,
            set_len,
        } => {
            let policy = resolve_policy(&genre, policy_hints.as_deref())?;
            let mut model = load_transition_model(transition_model.as_deref())?;
            if model.is_none() && !no_transition_model {
                let committed = resolve_committed_transition_model()?;
                model = load_transition_model(Some(&committed))?;
            }
            let report = cmd_suggest(
                &folder,
                &from,
                top,
                &policy,
                db.as_deref(),
                model.as_ref(),
                position,
                set_len,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Commands::Remix {
            folder,
            prefix,
            next,
            out,
            genre,
            db,
            deterministic: deterministic_flag,
            no_deterministic,
            lufs,
            no_loudness_norm,
            policy_hints,
            transition_model,
            no_transition_model,
        } => {
            let deterministic = deterministic_flag && !no_deterministic;
            let target = if no_loudness_norm {
                None
            } else if deterministic {
                Some(DEFAULT_TARGET_LUFS)
            } else {
                Some(lufs)
            };
            let policy = resolve_policy(&genre, policy_hints.as_deref())?;
            let mut model = load_transition_model(transition_model.as_deref())?;
            if model.is_none() && !no_transition_model {
                let committed = resolve_committed_transition_model()?;
                model = load_transition_model(Some(&committed))?;
            }
            let report = cmd_remix(
                &folder,
                &prefix,
                &next,
                &out,
                policy,
                db.as_deref(),
                target,
                model.as_ref(),
                deterministic,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Commands::Mix {
            playlist,
            playlist_name,
            preserve_order: keep_order,
            out,
            genre,
            golden,
            db,
            deterministic: deterministic_flag,
            no_deterministic,
            lufs,
            no_loudness_norm,
            policy_hints,
            transition_model,
            no_transition_model,
        } => {
            let deterministic = deterministic_flag && !no_deterministic;
            // Deterministic: fixed default LUFS (ignore --lufs) + hard beat/peak gates.
            let target = if no_loudness_norm {
                None
            } else if deterministic {
                Some(DEFAULT_TARGET_LUFS)
            } else {
                Some(lufs)
            };
            let policy = resolve_policy(&genre, policy_hints.as_deref())?;
            if let Some(name) = golden {
                // Golden suites use fixed LUFS unless --no-loudness-norm.
                // smoke / rekordbox / transitions omit the Softmax model for preferred_transition
                // bit-compat; only `transition_model` loads the committed artifact.
                let golden_target = if no_loudness_norm {
                    None
                } else {
                    Some(DEFAULT_TARGET_LUFS)
                };
                return match name.as_str() {
                    "smoke" => cmd_golden_smoke(
                        &out,
                        policy,
                        golden_target,
                        None,
                        deterministic,
                    ),
                    "rekordbox" => cmd_golden_rekordbox(
                        &out,
                        policy,
                        golden_target,
                        None,
                        deterministic,
                    ),
                    "transitions" => cmd_golden_transitions(
                        &out,
                        golden_target,
                        None,
                        deterministic,
                    ),
                    "transition_model" => {
                        cmd_golden_transition_model(&out, golden_target, deterministic)
                    }
                    "cue_overrides" => {
                        cmd_golden_cue_overrides(&out, golden_target, deterministic)
                    }
                    "transition_overrides" => {
                        cmd_golden_transition_overrides(&out, golden_target, deterministic)
                    }
                    other => bail!("unknown golden suite: {other}"),
                };
            }
            let mut model = load_transition_model(transition_model.as_deref())?;
            if model.is_none() && !no_transition_model {
                let committed = resolve_committed_transition_model()?;
                model = load_transition_model(Some(&committed))?;
            }
            let playlist = playlist.context("--playlist is required unless --golden is set")?;
            let report = cmd_mix(
                &playlist,
                playlist_name.as_deref(),
                &out,
                policy,
                db.as_deref(),
                target,
                model.as_ref(),
                deterministic,
                keep_order,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
    }
}

fn load_transition_model(path: Option<&Path>) -> Result<Option<TransitionModel>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read transition model {}", path.display()))?;
    let model = TransitionModel::from_json(&raw).context("parse transition model JSON")?;
    Ok(Some(model))
}

fn resolve_policy(genre: &str, hints_path: Option<&Path>) -> Result<GenrePolicy> {
    let base = GenrePolicy::builtin(genre);
    let Some(path) = hints_path else {
        return Ok(base);
    };
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read policy hints {}", path.display()))?;
    let map = load_policy_hints_map(&raw).context("parse policy hints JSON")?;
    if let Some(hint) = map.get(&base.id).or_else(|| map.get(genre)) {
        Ok(base.with_hints(hint))
    } else {
        Ok(base)
    }
}

fn path_key(p: &Path) -> String {
    p.canonicalize()
        .unwrap_or_else(|_| p.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    path_key(a) == path_key(b)
}

/// Load wav/flac features from a folder (DB hit or on-the-fly analyze).
fn load_folder_features(
    folder: &Path,
    db: Option<&Path>,
    genre_id: &str,
) -> Result<Vec<TrackFeatures>> {
    let paths = collect_audio(folder)?;
    if paths.is_empty() {
        bail!("no wav/flac in {}", folder.display());
    }
    let conn = db.map(open_db).transpose()?;
    let mut features = Vec::with_capacity(paths.len());
    for path in paths {
        let mut feats = if let Some(conn) = conn.as_ref() {
            load_features(conn, &path)?.unwrap_or_else(|| {
                analyze_path(&path).expect("analyze fallback")
            })
        } else {
            analyze_path(&path)?
        };
        resolve_track_phrase(&mut feats, genre_id);
        feats.path.replace(path);
        features.push(feats);
    }
    Ok(features)
}

fn cmd_suggest(
    folder: &Path,
    from: &Path,
    top: usize,
    policy: &GenrePolicy,
    db: Option<&Path>,
    model: Option<&TransitionModel>,
    position: usize,
    set_len: Option<usize>,
) -> Result<SuggestReport> {
    let features = load_folder_features(folder, db, &policy.id)?;
    let from_idx = features
        .iter()
        .position(|f| {
            f.path
                .as_ref()
                .map(|p| paths_equal(p, from))
                .unwrap_or(false)
        })
        .with_context(|| format!("--from not found in folder: {}", from.display()))?;
    let set_len = set_len.unwrap_or(features.len());
    let ranked = rank_next_matches(
        from_idx,
        &features,
        policy,
        model,
        top,
        position,
        set_len,
    );
    let candidates = ranked
        .into_iter()
        .map(|c| {
            let feats = &features[c.index];
            SuggestCandidateReport {
                path: feats
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                cost: c.cost,
                key: feats.key.as_str(),
                bpm: feats.bpm,
                class: c.transition,
                on_route: c.key_on_route,
            }
        })
        .collect();
    Ok(SuggestReport {
        from: path_key(from),
        candidates,
    })
}

fn cmd_remix(
    folder: &Path,
    prefix: &[PathBuf],
    next: &Path,
    out: &Path,
    policy: GenrePolicy,
    db: Option<&Path>,
    target_lufs: Option<f64>,
    model: Option<&TransitionModel>,
    deterministic: bool,
) -> Result<MixReport> {
    if prefix.is_empty() {
        bail!("remix requires at least one --prefix path (including current)");
    }
    let features = load_folder_features(folder, db, &policy.id)?;
    let index_of = |want: &Path| -> Result<usize> {
        features
            .iter()
            .position(|f| {
                f.path
                    .as_ref()
                    .map(|p| paths_equal(p, want))
                    .unwrap_or(false)
            })
            .with_context(|| format!("path not in folder: {}", want.display()))
    };
    let mut prefix_idxs = Vec::with_capacity(prefix.len());
    for p in prefix {
        prefix_idxs.push(index_of(p)?);
    }
    let next_idx = index_of(next)?;
    let order = build_remix_order(&features, &prefix_idxs, next_idx, &policy, model);
    if order.is_empty() {
        bail!("remix order empty (check prefix/next overlap)");
    }
    let mut paths = Vec::with_capacity(order.len());
    for &i in &order {
        let p = features[i]
            .path
            .clone()
            .context("track missing path")?;
        paths.push(serde_json::json!({ "path": path_key(&p) }));
    }
    let playlist_json = serde_json::json!({
        "genre": policy.id,
        "tracks": paths,
    });
    let playlist_path = std::env::temp_dir().join(format!(
        "autodj-remix-playlist-{}.json",
        std::process::id()
    ));
    fs::write(
        &playlist_path,
        serde_json::to_string_pretty(&playlist_json)?,
    )?;
    cmd_mix(
        &playlist_path,
        None,
        out,
        policy,
        db,
        target_lufs,
        model,
        deterministic,
        true, // preserve_order
    )
}

fn cmd_analyze(input: &Path, db: &Path) -> Result<()> {
    let conn = open_db(db)?;
    let paths = collect_audio(input)?;
    let mut n = 0usize;
    for path in paths {
        let feats = analyze_path(&path)
            .with_context(|| format!("analyze {}", path.display()))?;
        upsert_features(&conn, &feats)?;
        n += 1;
        println!(
            "analyzed {}  bpm={:.1} key={} loud={:.1}dB  bpm_conf={:.2} key_conf={:.2} sparse={} octave_corr={}",
            path.display(),
            feats.bpm,
            feats.key.as_str(),
            feats.loudness_db,
            feats.quality.bpm_confidence,
            feats.quality.key_confidence,
            feats.quality.sparse_audio,
            feats.quality.bpm_octave_corrected
        );
    }
    println!("wrote {n} tracks → {}", db.display());
    Ok(())
}

fn load_playlist_file(
    playlist: &Path,
    playlist_name: Option<&str>,
) -> Result<PlaylistFile> {
    let ext = playlist
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "xml" {
        let rb = parse_rekordbox_xml(playlist, playlist_name)
            .with_context(|| format!("parse Rekordbox XML {}", playlist.display()))?;
        Ok(PlaylistFile {
            genre: None,
            tracks: rb
                .tracks
                .into_iter()
                .map(|t| PlaylistTrack {
                    path: t.path,
                    key: t.key,
                    bpm: t.bpm,
                    beat_offset_secs: t.beat_offset_secs,
                    intro_end: t.intro_end,
                    outro_start: t.outro_start,
                    mix_in_secs: None,
                    mix_out_secs: None,
                    transition_out: None,
                    bars_per_phrase: None,
                })
                .collect(),
        })
    } else {
        let raw = fs::read_to_string(playlist)?;
        serde_json::from_str(&raw).with_context(|| format!("parse playlist JSON {}", playlist.display()))
    }
}

fn apply_track_overrides(feats: &mut TrackFeatures, t: &PlaylistTrack) {
    if let Some(k) = t.key.as_deref() {
        if let Some(ck) = CamelotKey::parse_flexible(k) {
            feats.key = ck;
        }
    }
    if let Some(bpm) = t.bpm {
        feats.bpm = bpm;
    }
    if let Some(off) = t.beat_offset_secs {
        feats.beat_offset_secs = off;
    }
    if let Some(intro) = t.intro_end {
        feats.sections.intro_end = intro;
    }
    if let Some(outro) = t.outro_start {
        feats.sections.outro_start = outro;
    }
    if let Some(bars) = t.bars_per_phrase {
        if matches!(bars, 4 | 8 | 16) {
            feats.bars_per_phrase = bars;
        }
    }
    // Keep drop between intro and outro when marks rearrange sections.
    if feats.sections.drop_start < feats.sections.intro_end
        || feats.sections.drop_start > feats.sections.outro_start
    {
        feats.sections.drop_start =
            (feats.sections.intro_end + feats.sections.outro_start) * 0.5;
    }
}

fn resolve_track_phrase(feats: &mut TrackFeatures, genre_id: &str) {
    if matches!(feats.bars_per_phrase, 4 | 8 | 16) {
        return;
    }
    let fallback = bars_per_phrase_for_genre(genre_id);
    feats.bars_per_phrase = estimate_bars_per_phrase(feats, fallback);
}

#[allow(clippy::too_many_arguments)]
fn cmd_mix(
    playlist: &Path,
    playlist_name: Option<&str>,
    out: &Path,
    mut policy: GenrePolicy,
    db: Option<&Path>,
    target_lufs: Option<f64>,
    model: Option<&TransitionModel>,
    deterministic: bool,
    keep_order: bool,
) -> Result<MixReport> {
    let pl = load_playlist_file(playlist, playlist_name)?;
    if let Some(g) = pl.genre.as_deref() {
        // Playlist genre wins identity; keep any bpm/xfade overrides already applied
        // only when the playlist genre matches the resolved policy id.
        if g != policy.id {
            policy = GenrePolicy::builtin(g);
        }
    }
    let conn = db.map(open_db).transpose()?;

    let playlist_dir = playlist.parent().unwrap_or_else(|| Path::new("."));
    let mut features = Vec::new();
    let mut prep_tracks = Vec::new();
    for t in &pl.tracks {
        let track_path = resolve_track_path(playlist_dir, &t.path);
        let mut feats = if let Some(conn) = conn.as_ref() {
            load_features(conn, &track_path)?.unwrap_or_else(|| {
                analyze_path(&track_path).expect("analyze fallback")
            })
        } else {
            analyze_path(&track_path)?
        };
        apply_track_overrides(&mut feats, t);
        resolve_track_phrase(&mut feats, &policy.id);
        let bpm_overridden = t.bpm.is_some();
        let key_overridden = t.key.is_some();
        let beat_offset_overridden = t.beat_offset_secs.is_some();
        let analysis_ok = !feats.quality.sparse_audio
            && feats.quality.bpm_confidence >= DEFAULT_MIN_BPM_CONFIDENCE;
        let trusted = analysis_ok || bpm_overridden;
        prep_tracks.push(PrepTrackReport {
            track: track_path.display().to_string(),
            sparse_audio: feats.quality.sparse_audio,
            bpm_confidence: feats.quality.bpm_confidence,
            key_confidence: feats.quality.key_confidence,
            bpm_octave_corrected: feats.quality.bpm_octave_corrected,
            bpm_overridden,
            key_overridden,
            beat_offset_overridden,
            trusted,
        });
        feats.path.replace(track_path);
        features.push(feats);
    }
    let prep_quality_gate_pass = !prep_tracks.is_empty() && prep_tracks.iter().all(|p| p.trusted);
    if deterministic && !prep_quality_gate_pass {
        let offenders: Vec<String> = prep_tracks
            .iter()
            .filter(|p| !p.trusted)
            .map(|p| {
                format!(
                    "{} (sparse={}, bpm_conf={:.2}, bpm_overridden={})",
                    p.track, p.sparse_audio, p.bpm_confidence, p.bpm_overridden
                )
            })
            .collect();
        bail!(
            "prep quality gate failed (need !sparse && bpm_conf>={:.2} or BPM override): {}",
            DEFAULT_MIN_BPM_CONFIDENCE,
            offenders.join("; ")
        );
    }

    let mut solution = if keep_order {
        let sol = preserve_order(&features, &policy, model);
        let expected: Vec<usize> = (0..features.len()).collect();
        if sol.order != expected {
            bail!(
                "preserve-order integrity failed: got {:?} expected {:?}",
                sol.order,
                expected
            );
        }
        sol
    } else {
        // Free start → quiet opener within BPM band (energy-arc setcraft).
        solve_order(&features, &policy, None, model)
    };
    let transition_overrides: Vec<Option<TransitionClass>> = pl
        .tracks
        .iter()
        .map(|t| t.transition_out)
        .collect();
    let transition_override_applied = apply_transition_overrides(
        &mut solution,
        &transition_overrides,
        &features,
        &policy,
    );
    let mut segments = Vec::new();
    let mut cue_reports = Vec::new();
    let mut cue_drop_aware = true;
    let mut saw_edge = false;
    let mut cue_mix_override_applied = false;
    let n = solution.order.len();
    for (i, &idx) in solution.order.iter().enumerate() {
        let feats = &features[idx];
        let phrase_bars = feats.bars_per_phrase;
        let next_feats = solution
            .order
            .get(i + 1)
            .map(|&ni| &features[ni]);
        let path = feats
            .path
            .clone()
            .unwrap_or_else(|| resolve_track_path(playlist_dir, &pl.tracks[idx].path));
        let full = read_wav(&path)?;
        let policy_xfade = solution
            .transitions
            .get(i)
            .map(|t| t.crossfade_secs)
            .unwrap_or(policy.default_crossfade_secs);
        if let Some(next) = next_feats {
            saw_edge = true;
            if !sections_drop_ordered(&feats.sections) || !sections_drop_ordered(&next.sections) {
                cue_drop_aware = false;
            }
        }
        let track = &pl.tracks[idx];
        let overrides = CueOverrides {
            mix_in_secs: track.mix_in_secs,
            mix_out_secs: track.mix_out_secs,
        };
        if overrides.any() {
            cue_mix_override_applied = true;
        }
        let cues = plan_segment_cues(
            feats,
            next_feats,
            policy_xfade,
            phrase_bars,
            i == 0,
            i + 1 == n,
            overrides,
        );
        let buffer = full.slice_secs(cues.play_start_secs, cues.play_end_secs);
        let transition = solution
            .transitions
            .get(i)
            .map(|t| t.class)
            .unwrap_or(TransitionClass::LinearCrossfade);
        let bpm_to = next_feats.map(|f| f.bpm);
        cue_reports.push(CueReport {
            track: path.display().to_string(),
            play_start_secs: cues.play_start_secs,
            play_end_secs: cues.play_end_secs,
            crossfade_secs: cues.crossfade_secs,
            bpm: feats.bpm,
            key: feats.key.as_str(),
            bpm_to,
            bars_per_phrase: phrase_bars,
            // Filled after render from segment_start_indices.
            mix_timeline_start_secs: 0.0,
            mix_timeline_end_secs: 0.0,
        });
        segments.push(MixSegment {
            buffer,
            bpm: feats.bpm,
            crossfade_secs: cues.crossfade_secs,
            transition,
        });
    }

    // Measured phrase alignment across edges (short loops skip-with-pass inside helper).
    let cue_phrase_aligned = if cue_reports.len() < 2 {
        true
    } else {
        (0..cue_reports.len() - 1).all(|i| {
            let out_i = solution.order[i];
            let in_i = solution.order[i + 1];
            cue_edge_phrase_aligned(
                &features[out_i],
                &features[in_i],
                cue_reports[i].play_end_secs,
                cue_reports[i + 1].play_start_secs,
            )
        })
    };

    let sr = segments
        .first()
        .map(|s| s.buffer.sample_rate)
        .unwrap_or(44100);
    let plan = MixPlan {
        segments,
        sample_rate: sr,
    };
    let rendered = mix_buffers_with_meta(&plan, CrossfadeParams::default());
    let mut mixed = rendered.buffer;
    let tempo_blend_applied = rendered.tempo_blend_applied;
    let join_sample_indices = rendered.join_sample_indices;
    let segment_start_indices = rendered.segment_start_indices;
    let max_bass_overlap = rendered.max_bass_overlap;
    let mix_duration_secs = if mixed.sample_rate > 0 {
        mixed.frames() as f64 / mixed.sample_rate as f64
    } else {
        0.0
    };
    let sr_f = sr as f64;
    let join_timeline_secs: Vec<f64> = segment_start_indices
        .iter()
        .skip(1)
        .map(|&i| i as f64 / sr_f)
        .collect();
    for (i, cue) in cue_reports.iter_mut().enumerate() {
        let start = segment_start_indices
            .get(i)
            .copied()
            .unwrap_or(0) as f64
            / sr_f;
        let end = segment_start_indices
            .get(i + 1)
            .map(|&j| j as f64 / sr_f)
            .unwrap_or(mix_duration_secs);
        cue.mix_timeline_start_secs = start;
        cue.mix_timeline_end_secs = end;
    }

    let (integrated_lufs, loudness_gate_pass, true_peak) = if let Some(target) = target_lufs {
        let report = normalize_loudness(&mut mixed, target)
            .with_context(|| format!("normalize to {target} LUFS"))?;
        let pass = within_lufs_tolerance(report.integrated_lufs, target);
        if !pass {
            bail!(
                "loudness gate failed: integrated {:.2} LUFS vs target {:.2} (±{:.1} LU)",
                report.integrated_lufs,
                target,
                LUFS_TOLERANCE_LU
            );
        }
        (
            Some(report.integrated_lufs),
            Some(pass),
            Some(report.true_peak),
        )
    } else {
        match measure_loudness(&mixed) {
            Ok(report) => (Some(report.integrated_lufs), None, Some(report.true_peak)),
            Err(_) => (None, None, None),
        }
    };

    let spectral_flux_ratio = adjacent_flux_ratio(&mixed);
    let spectral_ok = spectral_continuity_pass(spectral_flux_ratio, DEFAULT_MAX_FLUX_RATIO);
    if !spectral_ok {
        bail!(
            "spectral continuity gate failed: flux ratio {:.2} > max {:.2}",
            spectral_flux_ratio,
            DEFAULT_MAX_FLUX_RATIO
        );
    }

    let max_join_flux = max_join_flux_ratio(&mixed, &join_sample_indices);
    let join_ok = join_continuity_pass(max_join_flux, DEFAULT_JOIN_FLUX_RATIO);
    if deterministic && !join_ok {
        bail!(
            "join continuity gate failed: max join flux {:.2} > max {:.2}",
            max_join_flux,
            DEFAULT_JOIN_FLUX_RATIO
        );
    }

    let bass_ok = bass_conflict_pass(max_bass_overlap, DEFAULT_BASS_OVERLAP_LIMIT);
    if deterministic && !bass_ok {
        bail!(
            "bass conflict gate failed: max bass overlap {:.3} > limit {:.3}",
            max_bass_overlap,
            DEFAULT_BASS_OVERLAP_LIMIT
        );
    }

    let mut max_stretch_clamp_excess = 0.0f32;
    for w in solution.order.windows(2) {
        let excess = stretch_clamp_excess(features[w[0]].bpm, features[w[1]].bpm);
        if excess > max_stretch_clamp_excess {
            max_stretch_clamp_excess = excess;
        }
    }
    let stretch_ok = tempo_stretch_gate_pass(max_stretch_clamp_excess);
    if deterministic && !stretch_ok {
        bail!(
            "tempo stretch gate failed: max clamp excess {:.4} > 0 (WSOLA ±15%)",
            max_stretch_clamp_excess
        );
    }

    let phase_cues: Vec<BeatPhaseCue> = cue_reports
        .iter()
        .zip(solution.order.iter())
        .map(|(c, &idx)| BeatPhaseCue {
            play_start_secs: c.play_start_secs,
            play_end_secs: c.play_end_secs,
            crossfade_secs: c.crossfade_secs,
            bpm: c.bpm,
            beat_offset_secs: features[idx].beat_offset_secs,
        })
        .collect();
    let beat_phase_mean_ms = mean_beat_phase_offset_ms(&phase_cues);
    let beat_ok = beat_phase_gate_pass(beat_phase_mean_ms, DEFAULT_BEAT_PHASE_LIMIT_MS);
    if deterministic && !beat_ok {
        bail!(
            "beat-phase gate failed: mean {:.1} ms ≥ limit {:.1} ms",
            beat_phase_mean_ms,
            DEFAULT_BEAT_PHASE_LIMIT_MS
        );
    }

    let peak_ok = true_peak.map(|tp| peak_gate_pass(tp, DEFAULT_TRUE_PEAK_LIMIT));
    if deterministic {
        if let Some(false) = peak_ok {
            bail!(
                "peak gate failed: true_peak {:.4} > {:.2}",
                true_peak.unwrap_or(0.0),
                DEFAULT_TRUE_PEAK_LIMIT
            );
        }
    }

    write_wav(out, &mixed)?;

    let transitions: Vec<TransitionEdgeReport> = solution
        .transitions
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let a = &features[t.from];
            let b = &features[t.to];
            TransitionEdgeReport {
                from: t.from,
                to: t.to,
                class: t.class,
                policy_crossfade_secs: t.crossfade_secs,
                crossfade_secs: cue_reports
                    .get(i)
                    .map(|c| c.crossfade_secs)
                    .unwrap_or(t.crossfade_secs),
                key_from: a.key.as_str(),
                key_to: b.key.as_str(),
                camelot_distance: camelot_distance(a.key, b.key),
                bpm_delta: (a.bpm - b.bpm).abs(),
                key_on_route: suggest_next_keys(a.key).contains(&b.key),
                edge_cost: t.cost,
            }
        })
        .collect();

    Ok(MixReport {
        order: solution
            .order
            .iter()
            .map(|&i| {
                features[i]
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| format!("track-{i}"))
            })
            .collect(),
        total_cost: solution.total_cost,
        genre: policy.id,
        out: out.display().to_string(),
        integrated_lufs,
        target_lufs,
        loudness_gate_pass,
        per_track_phrase_applied: !cue_reports.is_empty()
            && cue_reports
                .iter()
                .all(|c| matches!(c.bars_per_phrase, 4 | 8 | 16)),
        cues: cue_reports,
        cue_phrase_aligned,
        cue_mix_override_applied,
        transition_override_applied,
        cue_drop_aware: cue_drop_aware && saw_edge,
        prep_tracks,
        prep_quality_gate_pass,
        tempo_blend_applied,
        spectral_flux_ratio,
        spectral_continuity_pass: spectral_ok,
        max_join_flux_ratio: max_join_flux,
        join_continuity_pass: join_ok,
        beat_phase_mean_ms,
        beat_phase_gate_pass: beat_ok,
        true_peak,
        peak_gate_pass: peak_ok,
        preserve_order: keep_order,
        max_bass_overlap,
        bass_conflict_gate_pass: bass_ok,
        transitions,
        max_stretch_clamp_excess,
        tempo_stretch_gate_pass: stretch_ok,
        join_timeline_secs,
    })
}

fn cmd_golden_smoke(
    out: &Path,
    policy: GenrePolicy,
    target_lufs: Option<f64>,
    model: Option<&TransitionModel>,
    deterministic: bool,
) -> Result<()> {
    let root = workspace_testdata()?;
    let playlist = root.join("party.json");
    if !playlist.exists() {
        // Ensure playlist exists next to gold wavs.
        let json = serde_json::json!({
            "genre": "house",
            "tracks": [
                {"path": root.join("gold/loop_a_128.wav"), "key": "8A", "bpm": 128.0},
                {"path": root.join("gold/loop_b_128.wav"), "key": "9A", "bpm": 128.0},
                {"path": root.join("gold/loop_c_140.wav"), "key": "10A", "bpm": 140.0}
            ]
        });
        fs::write(&playlist, serde_json::to_string_pretty(&json)?)?;
    }
    let report = cmd_mix(
        &playlist,
        None,
        out,
        policy,
        None,
        target_lufs,
        model,
        deterministic,
        false,
    )?;
    if !report.cue_phrase_aligned {
        bail!("smoke golden: expected cue_phrase_aligned");
    }
    if !report.cue_drop_aware {
        bail!("smoke golden: expected cue_drop_aware");
    }
    if !report.prep_quality_gate_pass {
        bail!("smoke golden: expected prep_quality_gate_pass");
    }
    if !report.per_track_phrase_applied {
        bail!("smoke golden: expected per_track_phrase_applied");
    }
    for cue in &report.cues {
        if !matches!(cue.bars_per_phrase, 4 | 8 | 16) {
            bail!(
                "smoke golden: cue bars_per_phrase {} not in {{4,8,16}}",
                cue.bars_per_phrase
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn cmd_golden_rekordbox(
    out: &Path,
    policy: GenrePolicy,
    target_lufs: Option<f64>,
    model: Option<&TransitionModel>,
    deterministic: bool,
) -> Result<()> {
    let root = workspace_testdata()?;
    let playlist = root.join("rekordbox/gold_playlist.xml");
    if !playlist.exists() {
        bail!("missing Rekordbox golden fixture: {}", playlist.display());
    }
    let report = cmd_mix(
        &playlist,
        Some("Gold Set"),
        out,
        policy,
        None,
        target_lufs,
        model,
        deterministic,
        true,
    )?;
    if !report.preserve_order {
        bail!("rekordbox golden: expected preserve_order");
    }
    if !report.cue_drop_aware {
        bail!("rekordbox golden: expected cue_drop_aware");
    }
    if !report.prep_quality_gate_pass {
        bail!("rekordbox golden: expected prep_quality_gate_pass");
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Multi-genre golden: assert each edge class matches genre `preferred_transition`.
fn cmd_golden_transitions(
    out: &Path,
    target_lufs: Option<f64>,
    model: Option<&TransitionModel>,
    deterministic: bool,
) -> Result<()> {
    let root = workspace_testdata()?;
    // Genre-less fixture so playlist genre cannot override CLI policy (unlike party.json).
    let playlist = root.join("transitions_playlist.json");
    if !playlist.exists() {
        let json = serde_json::json!({
            "tracks": [
                {"path": "gold/loop_a_128.wav", "key": "8A", "bpm": 128.0},
                {"path": "gold/loop_b_128.wav", "key": "9A", "bpm": 128.0},
                {"path": "gold/loop_c_140.wav", "key": "10A", "bpm": 140.0}
            ]
        });
        fs::write(&playlist, serde_json::to_string_pretty(&json)?)?;
    }

    const GENRES: &[&str] = &[
        "house",
        "trance",
        "hip_hop",
        "drum_and_bass",
        "uk_garage",
        "open_format",
    ];
    let mut summaries = Vec::new();
    for &genre_id in GENRES {
        let policy = GenrePolicy::builtin(genre_id);
        let expected = policy.preferred_transition;
        let report = cmd_mix(
            &playlist,
            None,
            out,
            policy.clone(),
            None,
            target_lufs,
            model,
            deterministic,
            true,
        )?;
        if report.genre != genre_id {
            bail!(
                "transitions golden: expected genre {genre_id}, got {}",
                report.genre
            );
        }
        if report.transitions.is_empty() {
            bail!("transitions golden ({genre_id}): no edges in MixReport");
        }
        if !report.cue_drop_aware {
            bail!("transitions golden ({genre_id}): expected cue_drop_aware");
        }
        for edge in &report.transitions {
            if edge.class != expected {
                bail!(
                    "transitions golden ({genre_id}): edge {}→{} class {:?} != preferred {:?}",
                    edge.from,
                    edge.to,
                    edge.class,
                    expected
                );
            }
            let bpm = report
                .cues
                .get(edge.from)
                .map(|c| c.bpm)
                .unwrap_or(128.0);
            let expected_xf = crossfade_secs_for_class(expected, &policy, bpm);
            if (edge.policy_crossfade_secs - expected_xf).abs() > 1e-3 {
                bail!(
                    "transitions golden ({genre_id}): edge {}→{} policy_crossfade_secs {:.4} != class-aware {:.4}",
                    edge.from,
                    edge.to,
                    edge.policy_crossfade_secs,
                    expected_xf
                );
            }
        }
        summaries.push(serde_json::json!({
            "genre": report.genre,
            "preferred_transition": expected,
            "transitions": report.transitions,
            "preserve_order": report.preserve_order,
            "cue_drop_aware": report.cue_drop_aware,
            "bass_conflict_gate_pass": report.bass_conflict_gate_pass,
            "join_continuity_pass": report.join_continuity_pass,
            "loudness_gate_pass": report.loudness_gate_pass,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&summaries)?);
    Ok(())
}

/// Softmax E2E golden: per-genre mix with committed model; classes must match predictions.
fn cmd_golden_transition_model(
    out: &Path,
    target_lufs: Option<f64>,
    deterministic: bool,
) -> Result<()> {
    let model_path = resolve_committed_transition_model()?;
    let raw = fs::read_to_string(&model_path)
        .with_context(|| format!("read {}", model_path.display()))?;
    let model = TransitionModel::from_json(&raw).context("parse committed transition model")?;

    let root = workspace_testdata()?;
    let playlist = root.join("transitions_playlist.json");
    if !playlist.exists() {
        bail!(
            "missing transitions playlist fixture: {}",
            playlist.display()
        );
    }

    const GENRES: &[&str] = &[
        "house",
        "trance",
        "hip_hop",
        "drum_and_bass",
        "uk_garage",
        "open_format",
    ];
    let features = load_playlist_features(&playlist, None)?;
    let expected_edges = features.len().saturating_sub(1);
    let mut summaries = Vec::new();

    for &genre_id in GENRES {
        let policy = GenrePolicy::builtin(genre_id);
        let report = cmd_mix(
            &playlist,
            None,
            out,
            policy,
            None,
            target_lufs,
            Some(&model),
            deterministic,
            true,
        )?;
        if report.genre != genre_id {
            bail!(
                "transition_model golden: expected genre {genre_id}, got {}",
                report.genre
            );
        }
        if report.transitions.len() != expected_edges {
            bail!(
                "transition_model golden ({genre_id}): expected {expected_edges} edges, got {}",
                report.transitions.len()
            );
        }
        for edge in &report.transitions {
            let a = &features[edge.from];
            let b = &features[edge.to];
            let predicted = predict_transition_class_edge(
                &model,
                a,
                b,
                camelot_distance(a.key, b.key),
                genre_id,
            );
            if edge.class != predicted {
                bail!(
                    "transition_model golden ({genre_id}): edge {}→{} MixReport {:?} != predict {:?}",
                    edge.from,
                    edge.to,
                    edge.class,
                    predicted
                );
            }
        }
        summaries.push(serde_json::json!({
            "genre": report.genre,
            "transitions": report.transitions,
            "preserve_order": report.preserve_order,
            "bass_conflict_gate_pass": report.bass_conflict_gate_pass,
            "join_continuity_pass": report.join_continuity_pass,
            "loudness_gate_pass": report.loudness_gate_pass,
        }));
    }

    println!("{}", serde_json::to_string_pretty(&summaries)?);
    Ok(())
}

/// Golden: playlist `transition_out` pins MixReport edge classes.
fn cmd_golden_transition_overrides(
    out: &Path,
    target_lufs: Option<f64>,
    deterministic: bool,
) -> Result<()> {
    let root = workspace_testdata()?;
    let playlist_path = root.join("transition_overrides_playlist.json");
    if !playlist_path.exists() {
        bail!(
            "missing transition_overrides golden fixture: {}",
            playlist_path.display()
        );
    }
    let pl = load_playlist_file(&playlist_path, None)?;
    let policy = GenrePolicy::builtin("house");
    // Softmax on so overrides must beat the committed model, not just preferred_transition.
    let model_path = resolve_committed_transition_model()?;
    let model = load_transition_model(Some(&model_path))?;
    let report = cmd_mix(
        &playlist_path,
        None,
        out,
        policy.clone(),
        None,
        target_lufs,
        model.as_ref(),
        deterministic,
        true, // preserve order for stable edge indices
    )?;
    if !report.transition_override_applied {
        bail!("transition_overrides golden: expected transition_override_applied");
    }
    if report.transitions.len() != pl.tracks.len().saturating_sub(1) {
        bail!(
            "transition_overrides golden: expected {} edges, got {}",
            pl.tracks.len().saturating_sub(1),
            report.transitions.len()
        );
    }
    for (i, track) in pl.tracks.iter().enumerate() {
        let Some(expected) = track.transition_out else {
            continue;
        };
        let Some(edge) = report.transitions.get(i) else {
            bail!("transition_overrides golden: missing edge {i}");
        };
        if edge.class != expected {
            bail!(
                "transition_overrides golden: edge {i} class {:?} != override {:?}",
                edge.class,
                expected
            );
        }
        if expected == TransitionClass::HardCut && edge.policy_crossfade_secs != 0.0 {
            bail!(
                "transition_overrides golden: HardCut edge {i} policy_crossfade_secs {:.4} != 0",
                edge.policy_crossfade_secs
            );
        }
        let expected_xf = crossfade_secs_for_class(expected, &policy, track.bpm.unwrap_or(128.0));
        if (edge.policy_crossfade_secs - expected_xf).abs() > 1e-4 {
            bail!(
                "transition_overrides golden: edge {i} policy_crossfade_secs {:.4} != class-aware {:.4}",
                edge.policy_crossfade_secs,
                expected_xf
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Golden: playlist absolute mix-in/out overrides pin CueReport play windows.
fn cmd_golden_cue_overrides(
    out: &Path,
    target_lufs: Option<f64>,
    deterministic: bool,
) -> Result<()> {
    let root = workspace_testdata()?;
    let playlist_path = root.join("cue_overrides_playlist.json");
    if !playlist_path.exists() {
        bail!(
            "missing cue_overrides golden fixture: {}",
            playlist_path.display()
        );
    }
    let pl = load_playlist_file(&playlist_path, None)?;
    let policy = GenrePolicy::builtin("house");
    let report = cmd_mix(
        &playlist_path,
        None,
        out,
        policy,
        None,
        target_lufs,
        None,
        deterministic,
        true, // preserve order for stable cue indices
    )?;
    if !report.cue_mix_override_applied {
        bail!("cue_overrides golden: expected cue_mix_override_applied");
    }
    if report.cues.len() != pl.tracks.len() {
        bail!(
            "cue_overrides golden: expected {} cues, got {}",
            pl.tracks.len(),
            report.cues.len()
        );
    }
    // Beat-snap can move by ≤½ beat; assert within that tolerance of the requested override.
    for (i, track) in pl.tracks.iter().enumerate() {
        let cue = &report.cues[i];
        let bpm = cue.bpm.max(1.0);
        let half_beat = 30.0 / bpm as f64;
        if let Some(mix_in) = track.mix_in_secs {
            let expected = snap_to_beat_grid(mix_in, bpm, 0.0);
            let delta = (cue.play_start_secs - expected).abs();
            if delta > half_beat + 1e-3 {
                bail!(
                    "cue_overrides golden: track {i} play_start {:.4} != override snap {:.4} (Δ={delta:.4})",
                    cue.play_start_secs,
                    expected
                );
            }
        }
        if let Some(mix_out) = track.mix_out_secs {
            let expected = snap_to_beat_grid(mix_out, bpm, 0.0);
            let delta = (cue.play_end_secs - expected).abs();
            if delta > half_beat + 1e-3 {
                bail!(
                    "cue_overrides golden: track {i} play_end {:.4} != override snap {:.4} (Δ={delta:.4})",
                    cue.play_end_secs,
                    expected
                );
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn resolve_committed_transition_model() -> Result<PathBuf> {
    let root = workspace_testdata()?;
    let rel = Path::new("python/autodj_train/models/artifacts/transition_model.json");
    let candidates = [
        root.parent().unwrap_or(Path::new(".")).join(rel),
        PathBuf::from(rel),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../").join(rel),
    ];
    for c in candidates {
        if c.exists() {
            return Ok(c.canonicalize().unwrap_or(c));
        }
    }
    bail!("could not locate committed transition_model.json — run from repo root")
}

/// Analyze + apply playlist overrides (same path as `cmd_mix`, no SQLite).
fn load_playlist_features(
    playlist: &Path,
    playlist_name: Option<&str>,
) -> Result<Vec<TrackFeatures>> {
    let pl = load_playlist_file(playlist, playlist_name)?;
    let playlist_dir = playlist.parent().unwrap_or_else(|| Path::new("."));
    let mut features = Vec::new();
    for t in &pl.tracks {
        let track_path = resolve_track_path(playlist_dir, &t.path);
        let mut feats = analyze_path(&track_path)?;
        apply_track_overrides(&mut feats, t);
        feats.path = Some(track_path);
        features.push(feats);
    }
    Ok(features)
}

fn resolve_track_path(playlist_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    // Prefer path relative to cwd (repo-root playlists), else beside the playlist file.
    if path.exists() {
        return path.to_path_buf();
    }
    let beside = playlist_dir.join(path);
    if beside.exists() {
        return beside;
    }
    // party.json under testdata/ often references testdata/gold/... from repo root.
    if let Some(name) = path.file_name() {
        let gold = playlist_dir.join("gold").join(name);
        if gold.exists() {
            return gold;
        }
    }
    path.to_path_buf()
}

fn workspace_testdata() -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("testdata"),
        PathBuf::from("../testdata"),
        PathBuf::from("../../testdata"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata"),
    ];
    for c in candidates {
        if c.join("gold/loop_a_128.wav").exists() {
            return Ok(c.canonicalize().unwrap_or(c));
        }
    }
    bail!("could not locate testdata/gold — run from repo root")
}

fn collect_audio(input: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if input.is_file() {
        out.push(input.to_path_buf());
        return Ok(out);
    }
    for entry in fs::read_dir(input)? {
        let path = entry?.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_ascii_lowercase().as_str(), "wav" | "flac") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tracks (
            path TEXT PRIMARY KEY,
            bpm REAL NOT NULL,
            key TEXT NOT NULL,
            loudness_db REAL NOT NULL,
            duration_secs REAL NOT NULL,
            sample_rate INTEGER NOT NULL,
            intro_end REAL NOT NULL,
            drop_start REAL NOT NULL,
            outro_start REAL NOT NULL,
            spectral_centroid_hz REAL NOT NULL,
            json TEXT NOT NULL
        );",
    )?;
    Ok(conn)
}

fn upsert_features(conn: &Connection, feats: &TrackFeatures) -> Result<()> {
    let path = feats
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let json = serde_json::to_string(feats)?;
    conn.execute(
        "INSERT INTO tracks (
            path, bpm, key, loudness_db, duration_secs, sample_rate,
            intro_end, drop_start, outro_start, spectral_centroid_hz, json
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(path) DO UPDATE SET
            bpm=excluded.bpm, key=excluded.key, loudness_db=excluded.loudness_db,
            duration_secs=excluded.duration_secs, sample_rate=excluded.sample_rate,
            intro_end=excluded.intro_end, drop_start=excluded.drop_start,
            outro_start=excluded.outro_start, spectral_centroid_hz=excluded.spectral_centroid_hz,
            json=excluded.json",
        params![
            path,
            feats.bpm,
            feats.key.as_str(),
            feats.loudness_db,
            feats.duration_secs,
            feats.sample_rate,
            feats.sections.intro_end,
            feats.sections.drop_start,
            feats.sections.outro_start,
            feats.spectral_centroid_hz,
            json,
        ],
    )?;
    Ok(())
}

fn load_features(conn: &Connection, path: &Path) -> Result<Option<TrackFeatures>> {
    let key = path.display().to_string();
    let mut stmt = conn.prepare("SELECT json FROM tracks WHERE path = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        let feats: TrackFeatures = serde_json::from_str(&json)?;
        Ok(Some(feats))
    } else {
        Ok(None)
    }
}
