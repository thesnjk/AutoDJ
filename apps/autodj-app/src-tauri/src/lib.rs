//! AutoDJ desktop shell — thin Tauri wrapper over `autodj-cli`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    line: String,
}

enum AutodjLaunch {
    Bin(PathBuf),
    Cargo { repo_root: PathBuf },
}

/// Locate committed Softmax artifact; omit from CLI args when missing (preferred_transition fallback).
fn resolve_committed_transition_model_path() -> Result<String, String> {
    let rel = Path::new("python/autodj_train/models/artifacts/transition_model.json");
    let mut candidates = Vec::new();
    if let Ok(root) = find_repo_root() {
        candidates.push(root.join(rel));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(rel));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join("../../..").join(rel));
    for c in candidates {
        if c.exists() {
            return Ok(c.canonicalize().unwrap_or(c).to_string_lossy().into_owned());
        }
    }
    Err(
        "committed transition_model.json not found (dev checkout or set AUTODJ_REPO_ROOT)".into(),
    )
}

/// Locate committed demo policy-hints JSON for Advanced → Load demo.
fn resolve_demo_policy_hints_path() -> Result<String, String> {
    let rel = Path::new("python/autodj_train/datasets/aggregates/demo_policy_hints.json");
    let mut candidates = Vec::new();
    if let Ok(root) = find_repo_root() {
        candidates.push(root.join(rel));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(rel));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join("../../..").join(rel));
    for c in candidates {
        if c.exists() {
            return Ok(c.canonicalize().unwrap_or(c).to_string_lossy().into_owned());
        }
    }
    Err(
        "demo_policy_hints.json not found (dev checkout or set AUTODJ_REPO_ROOT)".into(),
    )
}

fn find_repo_root() -> Result<PathBuf, String> {
    if let Ok(env) = std::env::var("AUTODJ_REPO_ROOT") {
        let p = PathBuf::from(env);
        if p.join("crates/autodj-cli").exists() {
            return Ok(p);
        }
    }
    let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
    for _ in 0..8 {
        if dir.join("crates/autodj-cli").exists() && dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    // Dev: apps/autodj-app/src-tauri → repo root is ../../..
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir
        .join("../../..")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if candidate.join("crates/autodj-cli").exists() {
        return Ok(candidate);
    }
    Err("could not locate AutoDJ repo root (set AUTODJ_REPO_ROOT)".into())
}

fn which_autodj() -> Option<PathBuf> {
    let output = Command::new("which").arg("autodj").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn resolve_autodj_launch() -> Result<AutodjLaunch, String> {
    if let Ok(bin) = std::env::var("AUTODJ_BIN") {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Ok(AutodjLaunch::Bin(p));
        }
        return Err(format!("AUTODJ_BIN not found: {}", p.display()));
    }
    if let Some(bin) = which_autodj() {
        return Ok(AutodjLaunch::Bin(bin));
    }
    let root = find_repo_root()?;
    Ok(AutodjLaunch::Cargo { repo_root: root })
}

/// Report how the shell will invoke autodj (for UI status).
#[tauri::command]
fn resolve_autodj_bin() -> Result<String, String> {
    match resolve_autodj_launch()? {
        AutodjLaunch::Bin(p) => Ok(format!("bin:{}", p.display())),
        AutodjLaunch::Cargo { repo_root } => {
            Ok(format!("cargo:{} (cargo run -p autodj-cli)", repo_root.display()))
        }
    }
}

#[tauri::command]
fn autodj_version(app: AppHandle) -> Result<String, String> {
    let (status, stdout, stderr) = run_autodj(&app, &["--version"])?;
    if !status {
        return Err(stderr.if_empty(stdout));
    }
    Ok(stdout.trim().to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeArgs {
    input: String,
    db: String,
}

#[tauri::command]
fn analyze(app: AppHandle, args: AnalyzeArgs) -> Result<String, String> {
    let (ok, stdout, stderr) = run_autodj(
        &app,
        &["analyze", "--input", &args.input, "--db", &args.db],
    )?;
    if !ok {
        return Err(stderr.if_empty(stdout));
    }
    Ok(stdout)
}

fn default_deterministic() -> bool {
    true
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MixArgs {
    playlist: String,
    out: String,
    genre: String,
    policy_hints: Option<String>,
    transition_model: Option<String>,
    /// Skip Softmax; use genre `preferred_transition` (CLI `--no-transition-model`).
    #[serde(default)]
    no_transition_model: bool,
    /// Feature DB path (optional; analyzes on the fly when omitted).
    db: Option<String>,
    /// Keep playlist order (Rekordbox setlist fidelity).
    preserve_order: Option<bool>,
    /// Rekordbox PLAYLISTS NODE name when the XML has multiple playlists.
    playlist_name: Option<String>,
    /// Strict MixReport hard gates (CLI default).
    #[serde(default = "default_deterministic")]
    deterministic: bool,
    /// Target LUFS when non-deterministic (CLI `--lufs`); ignored under Strict QA.
    lufs: Option<f64>,
    /// MixReport JSON path (CLI `--report-out`); default `{out}.mixreport.json`.
    report_out: Option<String>,
    /// Peak-limit only (CLI `--no-loudness-norm`).
    #[serde(default)]
    no_loudness_norm: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutomixFolderArgs {
    /// Primary library folder (used when `folders` is empty / omitted).
    folder: String,
    /// Optional multi-root list (Radio libraries). When non-empty, union of wav/flac across roots.
    #[serde(default)]
    folders: Option<Vec<String>>,
    out: String,
    genre: String,
    policy_hints: Option<String>,
    transition_model: Option<String>,
    /// Skip Softmax; use genre `preferred_transition` (CLI `--no-transition-model`).
    #[serde(default)]
    no_transition_model: bool,
    /// Feature DB; when set, analyze each root into DB before mix.
    db: Option<String>,
    preserve_order: Option<bool>,
    #[serde(default = "default_deterministic")]
    deterministic: bool,
    /// Target LUFS when non-deterministic (CLI `--lufs`); ignored under Strict QA.
    lufs: Option<f64>,
    /// MixReport JSON path (CLI `--report-out`); default `{out}.mixreport.json`.
    report_out: Option<String>,
    /// Peak-limit only (CLI `--no-loudness-norm`).
    #[serde(default)]
    no_loudness_norm: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeLibrariesArgs {
    /// Library root folders to analyze into `db`.
    folders: Vec<String>,
    db: String,
}

/// Collect wav/flac paths in a folder (sorted), mirroring CLI `collect_audio`.
fn collect_folder_audio(folder: &Path) -> Result<Vec<PathBuf>, String> {
    if !folder.is_dir() {
        return Err(format!("not a directory: {}", folder.display()));
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(folder).map_err(|e| format!("read_dir: {e}"))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("dir entry: {e}"))?.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_ascii_lowercase().as_str(), "wav" | "flac") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Resolve Automix roots: non-empty `folders`, else single `folder`.
fn resolve_library_roots(folder: &str, folders: Option<&[String]>) -> Result<Vec<PathBuf>, String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(list) = folders {
        for s in list {
            let t = s.trim();
            if !t.is_empty() {
                roots.push(PathBuf::from(t));
            }
        }
    }
    if roots.is_empty() {
        let t = folder.trim();
        if t.is_empty() {
            return Err("set a library folder or Radio libraries roots".into());
        }
        roots.push(PathBuf::from(t));
    }
    Ok(roots)
}

/// Union wav/flac across roots; dedupe by canonical path; sorted for determinism.
fn collect_roots_audio(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    use std::collections::BTreeSet;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<PathBuf> = Vec::new();
    for root in roots {
        let tracks = collect_folder_audio(root)?;
        for path in tracks {
            let key = path
                .canonicalize()
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
                .into_owned();
            if seen.insert(key) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn write_temp_playlist(genre: &str, tracks: &[PathBuf]) -> Result<PathBuf, String> {
    let mut paths = Vec::with_capacity(tracks.len());
    for t in tracks {
        let abs = t
            .canonicalize()
            .unwrap_or_else(|_| t.clone())
            .to_string_lossy()
            .into_owned();
        paths.push(serde_json::json!({ "path": abs }));
    }
    let json = serde_json::json!({
        "genre": genre,
        "tracks": paths,
    });
    let path = std::env::temp_dir().join(format!(
        "autodj-folder-playlist-{}.json",
        std::process::id()
    ));
    fs::write(&path, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?)
        .map_err(|e| format!("write temp playlist: {e}"))?;
    Ok(path)
}

/// One track in an Automix Editor re-mix (paths + optional cue/transition pins).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorTrack {
    path: String,
    mix_in_secs: Option<f64>,
    mix_out_secs: Option<f64>,
    /// Snake_case `TransitionClass` (e.g. `hard_cut`); omitted = auto.
    transition_out: Option<String>,
    key: Option<String>,
    bpm: Option<f64>,
    /// Phrase length pin (`4` / `8` / `16`); omitted = heuristic.
    bars_per_phrase: Option<u32>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MixEditorRemixArgs {
    out: String,
    genre: String,
    tracks: Vec<EditorTrack>,
    policy_hints: Option<String>,
    transition_model: Option<String>,
    /// Skip Softmax; use genre `preferred_transition` (CLI `--no-transition-model`).
    #[serde(default)]
    no_transition_model: bool,
    db: Option<String>,
    #[serde(default = "default_deterministic")]
    deterministic: bool,
    lufs: Option<f64>,
    /// MixReport JSON path (CLI `--report-out`); default `{out}.mixreport.json`.
    report_out: Option<String>,
    /// Peak-limit only (CLI `--no-loudness-norm`).
    #[serde(default)]
    no_loudness_norm: bool,
}

/// Build CLI-compatible playlist JSON from Automix Editor tracks (pretty string).
fn build_editor_playlist_json(genre: &str, tracks: &[EditorTrack]) -> Result<String, String> {
    if tracks.is_empty() {
        return Err("Automix Editor: need at least one track".into());
    }
    let mut objs = Vec::with_capacity(tracks.len());
    for t in tracks {
        let path = Path::new(&t.path);
        let abs = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned();
        let mut map = serde_json::Map::new();
        map.insert("path".into(), serde_json::Value::String(abs));
        if let Some(v) = t.mix_in_secs {
            map.insert("mix_in_secs".into(), serde_json::json!(v));
        }
        if let Some(v) = t.mix_out_secs {
            map.insert("mix_out_secs".into(), serde_json::json!(v));
        }
        if let Some(ref cls) = t.transition_out {
            let trimmed = cls.trim();
            if !trimmed.is_empty() {
                map.insert(
                    "transition_out".into(),
                    serde_json::Value::String(trimmed.to_string()),
                );
            }
        }
        if let Some(ref key) = t.key {
            let trimmed = key.trim();
            if !trimmed.is_empty() {
                map.insert("key".into(), serde_json::Value::String(trimmed.to_string()));
            }
        }
        if let Some(bpm) = t.bpm {
            map.insert("bpm".into(), serde_json::json!(bpm));
        }
        if let Some(bars) = t.bars_per_phrase {
            if matches!(bars, 4 | 8 | 16) {
                map.insert("bars_per_phrase".into(), serde_json::json!(bars));
            }
        }
        objs.push(serde_json::Value::Object(map));
    }
    let json = serde_json::json!({
        "genre": genre,
        "tracks": objs,
    });
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

/// Write a temp playlist JSON with optional mix-in/out and `transition_out` overrides.
fn write_temp_playlist_overrides(
    genre: &str,
    tracks: &[EditorTrack],
) -> Result<PathBuf, String> {
    let body = build_editor_playlist_json(genre, tracks)?;
    let path = std::env::temp_dir().join(format!(
        "autodj-editor-playlist-{}.json",
        std::process::id()
    ));
    fs::write(&path, body).map_err(|e| format!("write editor playlist: {e}"))?;
    Ok(path)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportEditorPlaylistArgs {
    dest: String,
    genre: String,
    tracks: Vec<EditorTrack>,
}

/// Persist Automix Editor pins as CLI-compatible playlist JSON at `dest`.
#[tauri::command]
fn export_editor_playlist(args: ExportEditorPlaylistArgs) -> Result<(), String> {
    let dest = args.dest.trim();
    if dest.is_empty() {
        return Err("export playlist: empty destination path".into());
    }
    let body = build_editor_playlist_json(&args.genre, &args.tracks)?;
    fs::write(dest, body).map_err(|e| format!("write playlist {}: {e}", dest))?;
    Ok(())
}

/// Re-mix with locked order and optional cue/transition overrides (Automix Editor).
#[tauri::command]
fn mix_editor_remix(app: AppHandle, args: MixEditorRemixArgs) -> Result<serde_json::Value, String> {
    let playlist = write_temp_playlist_overrides(&args.genre, &args.tracks)?;
    let hints = args.policy_hints.filter(|s| !s.trim().is_empty());
    let model = args.transition_model.filter(|s| !s.trim().is_empty());
    let db = args.db.filter(|s| !s.trim().is_empty());
    let report_out = args.report_out.filter(|s| !s.trim().is_empty());
    run_mix_cli(
        &app,
        playlist.to_string_lossy().as_ref(),
        &args.out,
        &args.genre,
        hints.as_deref(),
        model.as_deref(),
        args.no_transition_model,
        db.as_deref(),
        true, // lock MixReport.order
        None,
        args.deterministic,
        args.lufs,
        report_out.as_deref(),
        args.no_loudness_norm,
    )
}

/// Append Softmax path, committed default, or `--no-transition-model`.
fn push_transition_model_args(
    owned: &mut Vec<String>,
    no_transition_model: bool,
    transition_model: Option<&str>,
) {
    if no_transition_model {
        owned.push("--no-transition-model".into());
        return;
    }
    let resolved_default;
    if let Some(m) = transition_model.filter(|s| !s.is_empty()) {
        owned.push("--transition-model".into());
        owned.push(m.into());
    } else {
        resolved_default = resolve_committed_transition_model_path().ok();
        if let Some(ref path) = resolved_default {
            owned.push("--transition-model".into());
            owned.push(path.clone());
        }
    }
}

fn push_report_and_loudness_args(
    owned: &mut Vec<String>,
    report_out: Option<&str>,
    no_loudness_norm: bool,
    deterministic: bool,
    lufs: Option<f64>,
) {
    if let Some(path) = report_out.filter(|s| !s.is_empty()) {
        owned.push("--report-out".into());
        owned.push(path.into());
    }
    if no_loudness_norm {
        owned.push("--no-loudness-norm".into());
    }
    if deterministic {
        owned.push("--deterministic".into());
    } else {
        owned.push("--no-deterministic".into());
        // CLI ignores --lufs under --deterministic; peak-limit skips normalize targets.
        // Use `--lufs=-23` form so clap does not treat a leading '-' as a flag.
        if !no_loudness_norm {
            if let Some(target) = lufs {
                owned.push(format!("--lufs={target}"));
            }
        }
    }
}

fn run_mix_cli(
    app: &AppHandle,
    playlist: &str,
    out: &str,
    genre: &str,
    policy_hints: Option<&str>,
    transition_model: Option<&str>,
    no_transition_model: bool,
    db: Option<&str>,
    preserve_order: bool,
    playlist_name: Option<&str>,
    deterministic: bool,
    lufs: Option<f64>,
    report_out: Option<&str>,
    no_loudness_norm: bool,
) -> Result<serde_json::Value, String> {
    let mut owned: Vec<String> = vec![
        "mix".into(),
        "--playlist".into(),
        playlist.into(),
        "--out".into(),
        out.into(),
        "--genre".into(),
        genre.into(),
    ];
    if preserve_order {
        owned.push("--preserve-order".into());
    }
    if let Some(name) = playlist_name.filter(|s| !s.is_empty()) {
        owned.push("--playlist-name".into());
        owned.push(name.into());
    }
    if let Some(h) = policy_hints.filter(|s| !s.is_empty()) {
        owned.push("--policy-hints".into());
        owned.push(h.into());
    }
    if let Some(d) = db.filter(|s| !s.is_empty()) {
        owned.push("--db".into());
        owned.push(d.into());
    }
    push_transition_model_args(&mut owned, no_transition_model, transition_model);
    push_report_and_loudness_args(
        &mut owned,
        report_out,
        no_loudness_norm,
        deterministic,
        lufs,
    );

    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let (ok, stdout, stderr) = run_autodj(app, &refs)?;
    if !ok {
        return Err(stderr.if_empty(stdout));
    }
    extract_json(&stdout).ok_or_else(|| {
        format!(
            "mix succeeded but MixReport JSON not found in stdout:\n{}",
            stdout.chars().take(800).collect::<String>()
        )
    })
}

#[tauri::command]
fn mix(app: AppHandle, args: MixArgs) -> Result<serde_json::Value, String> {
    let hints = args.policy_hints.filter(|s| !s.trim().is_empty());
    let model = args.transition_model.filter(|s| !s.trim().is_empty());
    let db = args.db.filter(|s| !s.trim().is_empty());
    let playlist_name = args.playlist_name.filter(|s| !s.trim().is_empty());
    let report_out = args.report_out.filter(|s| !s.trim().is_empty());
    run_mix_cli(
        &app,
        &args.playlist,
        &args.out,
        &args.genre,
        hints.as_deref(),
        model.as_deref(),
        args.no_transition_model,
        db.as_deref(),
        args.preserve_order.unwrap_or(false),
        playlist_name.as_deref(),
        args.deterministic,
        args.lufs,
        report_out.as_deref(),
        args.no_loudness_norm,
    )
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuggestMatchesArgs {
    folder: String,
    from: String,
    #[serde(default = "default_top_n")]
    top: usize,
    genre: String,
    policy_hints: Option<String>,
    transition_model: Option<String>,
    /// Skip Softmax; use genre `preferred_transition` (CLI `--no-transition-model`).
    #[serde(default)]
    no_transition_model: bool,
    db: Option<String>,
    #[serde(default)]
    position: usize,
    set_len: Option<usize>,
}

fn default_top_n() -> usize {
    5
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemixQueueArgs {
    folder: String,
    /// Already-played paths including the current track.
    prefix: Vec<String>,
    next: String,
    out: String,
    genre: String,
    policy_hints: Option<String>,
    transition_model: Option<String>,
    /// Skip Softmax; use genre `preferred_transition` (CLI `--no-transition-model`).
    #[serde(default)]
    no_transition_model: bool,
    db: Option<String>,
    #[serde(default = "default_deterministic")]
    deterministic: bool,
    /// Target LUFS when non-deterministic (CLI `--lufs`); ignored under Strict QA.
    lufs: Option<f64>,
    /// MixReport JSON path (CLI `--report-out`); default `{out}.mixreport.json`.
    report_out: Option<String>,
    /// Peak-limit only (CLI `--no-loudness-norm`).
    #[serde(default)]
    no_loudness_norm: bool,
}

/// Rank next-track matches for the now-playing track (CLI `suggest`).
#[tauri::command]
fn suggest_matches(app: AppHandle, args: SuggestMatchesArgs) -> Result<serde_json::Value, String> {
    let mut owned: Vec<String> = vec![
        "suggest".into(),
        "--folder".into(),
        args.folder,
        "--from".into(),
        args.from,
        "--top".into(),
        args.top.to_string(),
        "--genre".into(),
        args.genre,
        "--position".into(),
        args.position.to_string(),
    ];
    if let Some(n) = args.set_len {
        owned.push("--set-len".into());
        owned.push(n.to_string());
    }
    if let Some(h) = args.policy_hints.filter(|s| !s.is_empty()) {
        owned.push("--policy-hints".into());
        owned.push(h);
    }
    if let Some(d) = args.db.filter(|s| !s.is_empty()) {
        owned.push("--db".into());
        owned.push(d);
    }
    let model = args.transition_model.filter(|s| !s.is_empty());
    push_transition_model_args(&mut owned, args.no_transition_model, model.as_deref());
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let (ok, stdout, stderr) = run_autodj(&app, &refs)?;
    if !ok {
        return Err(stderr.if_empty(stdout));
    }
    extract_json(&stdout).ok_or_else(|| {
        format!(
            "suggest succeeded but JSON not found in stdout:\n{}",
            stdout.chars().take(800).collect::<String>()
        )
    })
}

/// Pin next track after played prefix, solve remainder, preserve-order mix (CLI `remix`).
#[tauri::command]
fn remix_queue_next(app: AppHandle, args: RemixQueueArgs) -> Result<serde_json::Value, String> {
    if args.prefix.is_empty() {
        return Err("remix requires a non-empty prefix (including current track)".into());
    }
    emit_progress(
        &app,
        &format!(
            "queue remix: prefix={} next={}",
            args.prefix.len(),
            Path::new(&args.next)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&args.next)
        ),
    );
    let mut owned: Vec<String> = vec![
        "remix".into(),
        "--folder".into(),
        args.folder,
        "--next".into(),
        args.next,
        "--out".into(),
        args.out,
        "--genre".into(),
        args.genre,
    ];
    for p in &args.prefix {
        owned.push("--prefix".into());
        owned.push(p.clone());
    }
    if let Some(h) = args.policy_hints.filter(|s| !s.is_empty()) {
        owned.push("--policy-hints".into());
        owned.push(h);
    }
    if let Some(d) = args.db.filter(|s| !s.is_empty()) {
        owned.push("--db".into());
        owned.push(d);
    }
    let model = args.transition_model.filter(|s| !s.is_empty());
    push_transition_model_args(&mut owned, args.no_transition_model, model.as_deref());
    let report_out = args.report_out.filter(|s| !s.is_empty());
    push_report_and_loudness_args(
        &mut owned,
        report_out.as_deref(),
        args.no_loudness_norm,
        args.deterministic,
        args.lufs,
    );
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let (ok, stdout, stderr) = run_autodj(&app, &refs)?;
    if !ok {
        return Err(stderr.if_empty(stdout));
    }
    extract_json(&stdout).ok_or_else(|| {
        format!(
            "remix succeeded but MixReport JSON not found in stdout:\n{}",
            stdout.chars().take(800).collect::<String>()
        )
    })
}

/// Analyze each library root into the Feature DB (Radio libraries index).
#[tauri::command]
fn analyze_libraries(app: AppHandle, args: AnalyzeLibrariesArgs) -> Result<String, String> {
    let db = args.db.trim();
    if db.is_empty() {
        return Err("set Feature DB path".into());
    }
    let roots = resolve_library_roots("", Some(&args.folders))?;
    let mut summaries = Vec::new();
    for root in &roots {
        let root_s = root.to_string_lossy().into_owned();
        emit_progress(&app, &format!("analyze library root → {root_s}"));
        let (ok, stdout, stderr) =
            run_autodj(&app, &["analyze", "--input", &root_s, "--db", db])?;
        if !ok {
            return Err(stderr.if_empty(stdout));
        }
        summaries.push(format!("ok: {root_s}"));
    }
    Ok(summaries.join("\n"))
}

/// Scan one or more library roots of wav/flac, build a temp playlist, optionally analyze into `--db`, then mix.
#[tauri::command]
fn automix_folder(app: AppHandle, args: AutomixFolderArgs) -> Result<serde_json::Value, String> {
    let roots = resolve_library_roots(&args.folder, args.folders.as_deref())?;
    let tracks = collect_roots_audio(&roots)?;
    if tracks.len() < 2 {
        let roots_s = roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "need ≥2 wav/flac files across library roots [{roots_s}]; found {}",
            tracks.len()
        ));
    }
    emit_progress(
        &app,
        &format!(
            "library automix: {} tracks across {} root(s)",
            tracks.len(),
            roots.len()
        ),
    );

    let playlist_path = write_temp_playlist(&args.genre, &tracks)?;
    emit_progress(
        &app,
        &format!("wrote temp playlist {}", playlist_path.display()),
    );

    let db = args.db.filter(|s| !s.trim().is_empty());
    if let Some(ref db_path) = db {
        for root in &roots {
            let root_s = root.to_string_lossy().into_owned();
            let (ok, stdout, stderr) =
                run_autodj(&app, &["analyze", "--input", &root_s, "--db", db_path])?;
            if !ok {
                return Err(stderr.if_empty(stdout));
            }
            emit_progress(&app, &format!("analyze {root_s} → {db_path}"));
        }
    }

    let hints = args.policy_hints.filter(|s| !s.trim().is_empty());
    let model = args.transition_model.filter(|s| !s.trim().is_empty());
    let report_out = args.report_out.filter(|s| !s.trim().is_empty());
    let playlist_s = playlist_path.to_string_lossy().into_owned();
    run_mix_cli(
        &app,
        &playlist_s,
        &args.out,
        &args.genre,
        hints.as_deref(),
        model.as_deref(),
        args.no_transition_model,
        db.as_deref(),
        args.preserve_order.unwrap_or(false),
        None,
        args.deterministic,
        args.lufs,
        report_out.as_deref(),
        args.no_loudness_norm,
    )
}

trait IfEmpty {
    fn if_empty(self, other: String) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, other: String) -> String {
        if self.trim().is_empty() {
            other
        } else {
            self
        }
    }
}

fn extract_json(stdout: &str) -> Option<serde_json::Value> {
    let mut last: Option<serde_json::Value> = None;
    for (i, _) in stdout.match_indices('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stdout[i..]) {
            last = Some(v);
        }
    }
    last
}

fn emit_progress(app: &AppHandle, line: &str) {
    let _ = app.emit(
        "autodj-progress",
        ProgressEvent {
            line: line.to_string(),
        },
    );
}

fn run_autodj(app: &AppHandle, args: &[&str]) -> Result<(bool, String, String), String> {
    let launch = resolve_autodj_launch()?;
    let mut cmd = match &launch {
        AutodjLaunch::Bin(bin) => {
            emit_progress(app, &format!("$ {} {}", bin.display(), args.join(" ")));
            let mut c = Command::new(bin);
            c.args(args);
            c
        }
        AutodjLaunch::Cargo { repo_root } => {
            emit_progress(
                app,
                &format!(
                    "$ cargo run -q -p autodj-cli -- {}  (cwd={})",
                    args.join(" "),
                    repo_root.display()
                ),
            );
            let mut c = Command::new("cargo");
            c.current_dir(repo_root);
            c.args(["run", "-q", "-p", "autodj-cli", "--"]);
            c.args(args);
            c
        }
    };

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("spawn autodj: {e}"))?;

    let stdout = child.stdout.take().ok_or("missing stdout")?;
    let stderr = child.stderr.take().ok_or("missing stderr")?;

    let app_out = app.clone();
    let out_handle = thread::spawn(move || {
        let mut buf = String::new();
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            emit_progress(&app_out, &line);
            buf.push_str(&line);
            buf.push('\n');
        }
        buf
    });

    let app_err = app.clone();
    let err_handle = thread::spawn(move || {
        let mut buf = String::new();
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            emit_progress(&app_err, &format!("[err] {line}"));
            buf.push_str(&line);
            buf.push('\n');
        }
        buf
    });

    let status = child.wait().map_err(|e| format!("wait autodj: {e}"))?;
    let out_buf = out_handle.join().map_err(|_| "stdout thread panicked".to_string())?;
    let err_buf = err_handle.join().map_err(|_| "stderr thread panicked".to_string())?;

    Ok((status.success(), out_buf, err_buf))
}

/// Ensure a path exists for UX validation (optional helper).
#[tauri::command]
fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

/// Absolute path to committed `demo_policy_hints.json` (dev checkout).
#[tauri::command]
fn resolve_demo_policy_hints() -> Result<String, String> {
    resolve_demo_policy_hints_path()
}

/// Absolute path to committed Softmax `transition_model.json` (dev checkout).
#[tauri::command]
fn resolve_committed_transition_model() -> Result<String, String> {
    resolve_committed_transition_model_path()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            resolve_autodj_bin,
            autodj_version,
            analyze,
            analyze_libraries,
            mix,
            automix_folder,
            suggest_matches,
            remix_queue_next,
            mix_editor_remix,
            export_editor_playlist,
            path_exists,
            resolve_demo_policy_hints,
            resolve_committed_transition_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn resolve_roots_prefers_folders_list() {
        let roots = resolve_library_roots(
            "/ignored",
            Some(&["/a".into(), "  ".into(), "/b".into()]),
        )
        .unwrap();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0], PathBuf::from("/a"));
        assert_eq!(roots[1], PathBuf::from("/b"));
    }

    #[test]
    fn resolve_roots_falls_back_to_folder() {
        let roots = resolve_library_roots("/solo", Some(&[])).unwrap();
        assert_eq!(roots, vec![PathBuf::from("/solo")]);
    }

    #[test]
    fn collect_roots_dedupes_and_unions() {
        let dir = std::env::temp_dir().join(format!(
            "autodj-p80-roots-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        File::create(a.join("x.wav")).unwrap();
        File::create(a.join("y.flac")).unwrap();
        // Same basename in second root is a distinct path.
        File::create(b.join("x.wav")).unwrap();
        File::create(b.join("z.txt")).unwrap(); // ignored
        let tracks = collect_roots_audio(&[a, b]).unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(tracks.len(), 3);
    }
}
