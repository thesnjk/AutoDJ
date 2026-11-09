# Architecture

```
playlist / library ──► apps/autodj-app (Tauri) ──┐
        │                                         │
        ▼                                         ▼
 autodj-io ──► autodj-core (analyze + sections) ──► SQLite feature cache
        │
        ▼
 autodj-graph (Camelot/tempo + ArcPhase + policy hints + optional TransitionModel)
        │
        ▼
 autodj-core (cue: phrase/beat-grid snap) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA + EQ xfade + R128) ──► eval spectral + beat-phase/peak gates ──► WAV out
        │
        ▼
autodj_train: synth → SoftmaxLogReg JSON + tracklist aggregate → policy_hints
```

## Crates / app

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction (`AnalysisQuality`, BPM octave correction, `beat_offset_secs`), cue planner, EQ, WSOLA, R128, spectral + beat-phase eval (`eval.rs`). Pure Rust; primary target macOS Apple Silicon. Analysis is offline-only (not audio-thread safe).
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `PolicyHintOverride` / `with_hints`; optional `TransitionModel` per-edge class.
- **autodj-cli** — user-facing analyze/mix; `--policy-hints`; `--transition-model`; `--deterministic` (strict beat/peak gates + fixed LUFS); golden smoke for CI.
- **autodj-app** — Tauri 2 desktop shell; invokes CLI via `AUTODJ_BIN` / PATH / `cargo run -p autodj-cli`; emits `autodj-progress` events. Excluded from the Cargo workspace (`workspace.exclude`).

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and softmax training (numpy only in CI). Aggregate `policy_hints` and `transition_model.json` feed Rust via CLI. PyTorch never links into the CLI (optional extra only).
