//! AutoDJ CLI — analyze libraries and render offline mixes.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use autodj_core::{
    adjacent_flux_ratio, analyze_path, bars_per_phrase_for_genre, bass_conflict_pass,
    beat_phase_gate_pass, join_continuity_pass, max_join_flux_ratio, mean_beat_phase_offset_ms,
    measure_loudness, mix_buffers_with_meta, normalize_loudness, parse_rekordbox_xml,
    peak_gate_pass, plan_segment_cues, spectral_continuity_pass, within_lufs_tolerance,
    BeatPhaseCue, CamelotKey, CrossfadeParams, MixPlan, MixSegment, TrackFeatures, TransitionClass,
    DEFAULT_BASS_OVERLAP_LIMIT, DEFAULT_BEAT_PHASE_LIMIT_MS, DEFAULT_JOIN_FLUX_RATIO,
    DEFAULT_MAX_FLUX_RATIO, DEFAULT_TARGET_LUFS, DEFAULT_TRUE_PEAK_LIMIT, LUFS_TOLERANCE_LU,
};
use autodj_graph::{
    load_policy_hints_map, preserve_order, solve_order, GenrePolicy, TransitionModel,
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
        /// Built-in golden suite: `smoke` (JSON) or `rekordbox` (XML preserve-order).
        #[arg(long)]
        golden: Option<String>,
        /// Feature DB (optional; analyzes on the fly if missing).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Strict gates + fixed defaults (beat-phase / peak hard-fail; default LUFS path).
        #[arg(long, default_value_t = true)]
        deterministic: bool,
        /// Target integrated loudness in LUFS (EBU R 128 / BS.1770). Use -23 for broadcast.
        #[arg(long, default_value_t = DEFAULT_TARGET_LUFS)]
        lufs: f64,
        /// Skip loudness normalization (peak-limit only).
        #[arg(long, default_value_t = false)]
        no_loudness_norm: bool,
        /// Optional genre policy hints JSON from tracklist aggregate (`autodj-train aggregate-tracklists --hints-out`).
        #[arg(long)]
        policy_hints: Option<PathBuf>,
        /// Optional Softmax transition model JSON from `autodj-train train` (per-edge class).
        #[arg(long)]
        transition_model: Option<PathBuf>,
    },
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
}

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
}

#[derive(Debug, Serialize)]
struct CueReport {
    track: String,
    play_start_secs: f64,
    play_end_secs: f64,
    crossfade_secs: f32,
    bpm: f32,
    /// BPM of the next track when a transition follows; else null.
    bpm_to: Option<f32>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Analyze { input, db } => cmd_analyze(&input, &db),
        Commands::Mix {
            playlist,
            playlist_name,
            preserve_order: keep_order,
            out,
            genre,
            golden,
            db,
            deterministic,
            lufs,
            no_loudness_norm,
            policy_hints,
            transition_model,
        } => {
            // Deterministic: fixed default LUFS (ignore --lufs) + hard beat/peak gates.
            let target = if no_loudness_norm {
                None
            } else if deterministic {
                Some(DEFAULT_TARGET_LUFS)
            } else {
                Some(lufs)
            };
            let policy = resolve_policy(&genre, policy_hints.as_deref())?;
            let model = load_transition_model(transition_model.as_deref())?;
            if let Some(name) = golden {
                // Golden always uses the default deterministic target unless --no-loudness-norm.
                // Omit --transition-model for bit-compatible preferred_transition behavior.
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
                        model.as_ref(),
                        deterministic,
                    ),
                    "rekordbox" => cmd_golden_rekordbox(
                        &out,
                        policy,
                        golden_target,
                        model.as_ref(),
                        deterministic,
                    ),
                    other => bail!("unknown golden suite: {other}"),
                };
            }
            let playlist = playlist.context("--playlist is required unless --golden is set")?;
            cmd_mix(
                &playlist,
                playlist_name.as_deref(),
                &out,
                policy,
                db.as_deref(),
                target,
                model.as_ref(),
                deterministic,
                keep_order,
            )
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
    // Keep drop between intro and outro when marks rearrange sections.
    if feats.sections.drop_start < feats.sections.intro_end
        || feats.sections.drop_start > feats.sections.outro_start
    {
        feats.sections.drop_start =
            (feats.sections.intro_end + feats.sections.outro_start) * 0.5;
    }
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
) -> Result<()> {
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
        feats.path = Some(track_path);
        features.push(feats);
    }

    let solution = if keep_order {
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
    let phrase_bars = bars_per_phrase_for_genre(&policy.id);
    let mut segments = Vec::new();
    let mut cue_reports = Vec::new();
    let n = solution.order.len();
    for (i, &idx) in solution.order.iter().enumerate() {
        let feats = &features[idx];
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
        let cues = plan_segment_cues(
            feats,
            next_feats,
            policy_xfade,
            phrase_bars,
            i == 0,
            i + 1 == n,
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
            bpm_to,
        });
        segments.push(MixSegment {
            buffer,
            bpm: feats.bpm,
            crossfade_secs: cues.crossfade_secs,
            transition,
        });
    }

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
    let max_bass_overlap = rendered.max_bass_overlap;

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

    let report = MixReport {
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
        cues: cue_reports,
        cue_phrase_aligned: true,
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
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
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
    cmd_mix(
        &playlist,
        None,
        out,
        policy,
        None,
        target_lufs,
        model,
        deterministic,
        false,
    )
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
    cmd_mix(
        &playlist,
        Some("Gold Set"),
        out,
        policy,
        None,
        target_lufs,
        model,
        deterministic,
        true,
    )
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
