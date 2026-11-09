# Getting started

## Prerequisites

- Rust stable (1.75+)
- Python 3.12+ (train package)
- Optional: Tauri deps when building `apps/autodj-app`

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
```

Keys/BPM in the playlist override analyzer estimates when present.
