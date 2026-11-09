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
 autodj-core (cue: phrase snap) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA + EQ xfade + R128) ──► eval spectral gate ──► WAV out
        │
        ▼
autodj_train: synth → SoftmaxLogReg JSON + tracklist aggregate → policy_hints
```

## Crates / app

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction, cue planner, EQ, WSOLA, R128, spectral continuity eval (`eval.rs`). Pure Rust; primary target macOS Apple Silicon.
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `PolicyHintOverride` / `with_hints`; optional `TransitionModel` per-edge class.
- **autodj-cli** — user-facing analyze/mix; `--policy-hints`; `--transition-model`; golden smoke for CI.
- **autodj-app** — Tauri 2 desktop shell; invokes CLI via `AUTODJ_BIN` / PATH / `cargo run -p autodj-cli`; emits `autodj-progress` events. Excluded from the Cargo workspace (`workspace.exclude`).

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and softmax training (numpy only in CI). Aggregate `policy_hints` and `transition_model.json` feed Rust via CLI. PyTorch never links into the CLI (optional extra only).
