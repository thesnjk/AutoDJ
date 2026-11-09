# Developer map (AutoDJ)

Thin index for humans and agents. Prefer linking one Doc at a time.

## Layout

| Path | Role |
|------|------|
| `crates/autodj-core/` | Analysis + offline DSP |
| `crates/autodj-graph/` | Mix order / transition solver |
| `crates/autodj-io/` | WAV decode/encode |
| `crates/autodj-cli/` | `autodj` binary |
| `python/autodj_train/` | Synthetic transitions, trainers, eval |
| `apps/autodj-app/` | Tauri shell (phase 2 stub) |
| `testdata/gold/` | Royalty-free synthetic WAV fixtures |
| `Docs/` | Specs, legal, rubrics |

## Platform

Primary runtime: **macOS Apple Silicon**. Offline path uses pure Rust (`eq.rs`, `loudness.rs`, `render.rs`) — no Core Audio / cpal required for export. GitHub Actions runs Rust on `macos-14` and `ubuntu-latest`.

## Tests

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav
afplay /tmp/autodj-smoke.wav   # macOS listen check
cd python && pip install -e ".[dev]" && pytest autodj_train
```

## Hard rules

- No heap alloc / blocking I/O on future real-time audio paths.
- Never commit copyrighted commercial audio or scraped full DJ sets.
- Master prompt `Docs/autoDJ-master-prompt.md` is local-only (`.git/info/exclude`).
- Prefer issue → branch → PR.
- Keep offline DSP portable (Mac-first, Linux CI); do not introduce Linux-only audio backends.

## Docs pointers

- Architecture: `Docs/architecture.md`
- Legal ingest: `Docs/legal-data-policy.md`
- Mix quality: `Docs/mix-quality-rubric.md`
- Genre defaults: `Docs/genre-policy.md`
- Competitive sweep: `Docs/competitive-automix-sweep.md`
- Getting started: `Docs/getting-started.md`
