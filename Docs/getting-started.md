# Getting started

## Prerequisites

- **Primary platform:** macOS Apple Silicon (`aarch64-apple-darwin`). Offline DSP is pure Rust (EQ, WSOLA tempo blend, R128 — no Linux-only audio backends).
- Rust stable (1.75+)
- Python 3.12+ (train package)
- Optional desktop: Node 20+ for `apps/autodj-app` (Tauri 2)
- CI also runs the Rust suite on `macos-latest` and `ubuntu-latest` (hosted Mac runners need Actions minutes on private repos).

## First mix

```bash
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav
# Optional: classic EBU R 128 programme loudness
# cargo run -p autodj-cli -- mix --golden smoke --lufs -23 --out /tmp/autodj-smoke.wav
afplay /tmp/autodj-smoke.wav   # macOS
```
Default export target is **−14 LUFS** (streaming-style). Pass `--lufs -23` for broadcast R 128.

## Your playlist

Create `party.json`:

```json
{
  "genre": "house",
  "tracks": [
    {"path": "/absolute/path/a.wav", "key": "8A", "bpm": 124},
    {"path": "/absolute/path/b.wav", "key": "9A", "bpm": 125}
  ]
}
```

```bash
cargo run -p autodj-cli -- analyze --input /absolute/path --db features.sqlite
cargo run -p autodj-cli -- mix --playlist party.json --db features.sqlite --out mix.wav --genre house

# Optional setcraft / classifier overrides
cargo run -p autodj-cli -- mix --playlist party.json --out mix.wav \
  --policy-hints python/autodj_train/datasets/aggregates/demo_policy_hints.json
cargo run -p autodj-cli -- mix --playlist party.json --out mix.wav \
  --transition-model python/autodj_train/models/artifacts/transition_model.json
```

Keys/BPM in the playlist override analyzer estimates when present.

Train gate (CI):

```bash
cd python && pip install -e ".[dev]"
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
```

## Desktop shell (Tauri)

```bash
cargo build -p autodj-cli
cd apps/autodj-app
npm install
npm run tauri dev
```

Point the UI at a playlist JSON and output WAV. CLI resolution: `AUTODJ_BIN` → `autodj` on `PATH` → `cargo run -p autodj-cli` from the repo root.
