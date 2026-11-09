//! AutoDJ desktop shell — thin Tauri wrapper over `autodj-cli`.

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
    /// Keep playlist order (Rekordbox setlist fidelity).
    preserve_order: Option<bool>,
    /// Rekordbox PLAYLISTS NODE name when the XML has multiple playlists.
    playlist_name: Option<String>,
    /// Strict MixReport hard gates (CLI default).
    #[serde(default = "default_deterministic")]
    deterministic: bool,
}

#[tauri::command]
fn mix(app: AppHandle, args: MixArgs) -> Result<serde_json::Value, String> {
    let mut cli_args: Vec<&str> = vec![
        "mix",
        "--playlist",
        &args.playlist,
        "--out",
        &args.out,
        "--genre",
        &args.genre,
    ];
    let hints = args.policy_hints.filter(|s| !s.trim().is_empty());
    let model = args.transition_model.filter(|s| !s.trim().is_empty());
    let playlist_name = args
        .playlist_name
        .filter(|s| !s.trim().is_empty());
    if args.preserve_order.unwrap_or(false) {
        cli_args.push("--preserve-order");
    }
    if let Some(ref name) = playlist_name {
        cli_args.push("--playlist-name");
        cli_args.push(name.as_str());
    }
    if let Some(ref h) = hints {
        cli_args.push("--policy-hints");
        cli_args.push(h.as_str());
    }
    if let Some(ref m) = model {
        cli_args.push("--transition-model");
        cli_args.push(m.as_str());
    }
    if args.deterministic {
        cli_args.push("--deterministic");
    } else {
        cli_args.push("--no-deterministic");
    }

    let (ok, stdout, stderr) = run_autodj(&app, &cli_args)?;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            resolve_autodj_bin,
            autodj_version,
            analyze,
            mix,
            path_exists
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
