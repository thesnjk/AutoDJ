# Architecture

```
playlist / library
        │
        ▼
 autodj-io ──► autodj-core (analyze) ──► SQLite feature cache
        │
        ▼
 autodj-graph (genre policy + Camelot/tempo/energy cost)
        │
        ▼
 autodj-core (render: crossfade + limiter) ──► WAV out
        │
        ▼
autodj_train (optional): synthetic pairs → transition classifier weights
```

## Crates

- **autodj-io** — PCM containers only.
- **autodj-core** — feature extraction, Camelot helpers, offline mix renderer.
- **autodj-graph** — greedy nearest-neighbor solver (TSP upgrade later) + genre policy.
- **autodj-cli** — user-facing analyze/mix; golden smoke for CI.

## Train / runtime split

Python owns dataset synthesis and model training. Exported weights (future ONNX) are loaded by Rust inference; PyTorch never links into the CLI.
