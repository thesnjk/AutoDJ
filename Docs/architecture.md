# Architecture

```
playlist / library
        │
        ▼
 autodj-io ──► autodj-core (analyze + sections) ──► SQLite feature cache
        │
        ▼
 autodj-graph (Camelot/tempo + ArcPhase + optional policy hints)
        │
        ▼
 autodj-core (cue: phrase snap) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA + EQ xfade + R128) ──► eval spectral gate ──► WAV out
        │
        ▼
autodj_train: synthetic pairs + tracklist aggregate → policy_hints JSON
```

## Crates

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction, cue planner, EQ, WSOLA, R128, spectral continuity eval (`eval.rs`). Pure Rust; primary target macOS Apple Silicon.
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `PolicyHintOverride` / `with_hints`.
- **autodj-cli** — user-facing analyze/mix; `--policy-hints`; golden smoke for CI.

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and model training. Aggregate `policy_hints` feed Rust `GenrePolicy` via CLI. Exported weights (future ONNX) are loaded by Rust inference; PyTorch never links into the CLI.
