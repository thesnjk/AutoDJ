# Architecture

```
playlist / library
        │
        ▼
 autodj-io ──► autodj-core (analyze + sections) ──► SQLite feature cache
        │
        ▼
 autodj-graph (genre policy + Camelot/tempo/energy cost)
        │
        ▼
 autodj-core (cue: phrase snap) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA tempo blend + EQ xfade + R128) ──► WAV out
        │
        ▼
autodj_train (optional): synthetic pairs → transition classifier weights
```

## Crates

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction, Camelot helpers, phrase cue planner (`cue.rs`), 3-band EQ (`eq.rs`), WSOLA tempo blend (`stretch.rs`), offline mix renderer, R128 loudness. Pure Rust; primary target macOS Apple Silicon.
- **autodj-graph** — greedy nearest-neighbor solver (TSP upgrade later) + genre policy.
- **autodj-cli** — user-facing analyze/mix; golden smoke for CI.

## Train / runtime split

Python owns dataset synthesis and model training. Exported weights (future ONNX) are loaded by Rust inference; PyTorch never links into the CLI.
