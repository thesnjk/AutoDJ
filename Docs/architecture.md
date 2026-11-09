# Architecture

```
playlist JSON / Rekordbox XML ──► apps/autodj-app (Tauri) ──┐
        │                                                    │
        ▼                                                    ▼
 autodj-io ──► autodj-core (analyze + rekordbox overrides) ──► SQLite feature cache
        │
        ▼
 autodj-graph (solve_order or preserve_order + TransitionModel)
        │
        ▼
 autodj-core (cue: phrase/beat-grid snap) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA + EQ xfade + R128) ──► eval spectral + join + bass + beat-phase/peak gates ──► WAV out
        │
        ▼
autodj_train: synth → SoftmaxLogReg JSON + tracklist aggregate → policy_hints
```

## Crates / app

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction (`AnalysisQuality`, BPM octave correction, `beat_offset_secs`), Rekordbox XML ingest (`rekordbox.rs`), cue planner, EQ (incl. bass-overlap score), WSOLA, R128, spectral + join-local + beat-phase eval (`eval.rs`; join indices + bass overlap from renderer). Pure Rust; primary target macOS Apple Silicon. Analysis is offline-only (not audio-thread safe).
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `preserve_order` for setlist fidelity; `PolicyHintOverride` / `with_hints`; optional `TransitionModel` per-edge class.
- **autodj-cli** — user-facing analyze/mix; JSON or Rekordbox `.xml` via `--playlist`; `--playlist-name`; `--preserve-order`; `--policy-hints`; `--transition-model`; `--deterministic`; CI goldens `--golden smoke` / `rekordbox` / `transitions` / `transition_model` (Softmax E2E).
- **autodj-app** — Tauri 2 desktop shell; dialog picker for JSON/Rekordbox XML; forwards `--preserve-order` / `--playlist-name` to CLI via `AUTODJ_BIN` / PATH / `cargo run -p autodj-cli`; emits `autodj-progress` events. Excluded from the Cargo workspace (`workspace.exclude`).

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and softmax training (numpy only in CI). Aggregate `policy_hints` and `transition_model.json` feed Rust via CLI. PyTorch never links into the CLI (optional extra only).
