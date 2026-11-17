# Developer map (AutoDJ)

Thin index for humans and agents. Prefer linking one Doc at a time.

For direction and Next action, read [`project-charter.md`](project-charter.md) (**NARROW**) then take the first unfinished row in [`../product/backlog.md`](../product/backlog.md).

## Layout

| Path | Role |
|------|------|
| `crates/autodj-core/` | Analysis + offline DSP |
| `crates/autodj-graph/` | Mix order / transition solver |
| `crates/autodj-io/` | WAV/FLAC decode (`read_audio`) + WAV/FLAC encode (`write_audio`) |
| `crates/autodj-cli/` | `autodj` binary |
| `python/autodj_train/` | Synthetic transitions, Softmax train/eval |
| `apps/autodj-app/` | Tauri 2 shell — Radio libraries, continuous radio (horizon prerender), set-length + vibe; see app README + getting-started |
| `Docs/full-auto-roadmap.md` | North Star + P80–P83 phase ladder |
| `Docs/project-charter.md` | Current commercial direction (NARROW); source of truth for posture |
| `Docs/founder-os-audit.md` | Founder OS upgrade audit (preserve / gaps / contradictions) |
| `product/backlog.md` | Ordered self-service backlog (agent Next) |
| `research/` | Portfolio, scorecard, risk register, discovery/pilot templates |
| `testdata/gold/` | Royalty-free synthetic WAV + `loop_a_128.flac` fixtures |
| `Docs/` | Specs, legal, rubrics |
| `Docs/project-writeup.md` | Thesis-grade writeup + living capability ledger |
| `Docs/data-contracts.md` | Normative playlist / MixReport / Softmax / SQLite / hints / suggest+remix CLI shapes |
| `LICENSE` / `NOTICE` | MIT; sole copyright Jacob Hackett |
| ~~`Docs/founder-master-prompt.md`~~ | **Removed** — do not restore; operating rules are local `Docs/autoDJ-master-prompt.md` only |

## Platform

Primary runtime: **macOS Apple Silicon**. Offline path uses pure Rust (`eq.rs`, `loudness.rs`, `render.rs`) — no Core Audio / cpal required for export. GitHub Actions runs Rust on `macos-latest` and `ubuntu-latest`, plus `cargo check --manifest-path apps/autodj-app/src-tauri/Cargo.toml` on `macos-latest` only. Full `tauri dev` / WebKit remain Mac-local (Ubuntu WebKit CI deferred).

## Tests

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav
# → /tmp/autodj-smoke.wav.mixreport.json
# cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.flac
cargo run -p autodj-cli -- analyze --input testdata/gold/loop_a_128.flac --db /tmp/flac.sqlite
cargo run -p autodj-cli -- mix --golden broadcast --out /tmp/autodj-broadcast.wav
cargo run -p autodj-cli -- mix --golden policy_hints --out /tmp/autodj-policy-hints.wav
cargo run -p autodj-cli -- mix --golden rekordbox_cue_overrides --out /tmp/autodj-rb-cues.wav
afplay /tmp/autodj-smoke.wav   # macOS listen check
cd python && pip install -e ".[dev]" && pytest autodj_train
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
# Desktop (Mac):
cd apps/autodj-app && npm install && cargo check --manifest-path src-tauri/Cargo.toml
# npm run tauri dev
```

## Hard rules

- No heap alloc / blocking I/O on future real-time audio paths.
- Never commit copyrighted commercial audio or scraped full DJ sets.
- Master prompt `Docs/autoDJ-master-prompt.md` is local-only (`.git/info/exclude`).
- Prefer issue → branch → PR.
- Keep offline DSP portable (Mac-first, Linux CI); do not introduce Linux-only audio backends.
- `apps/autodj-app/src-tauri` is **excluded** from the workspace Cargo members.

## Docs pointers

- Project charter (direction): `Docs/project-charter.md`
- Backlog (agent Next): `product/backlog.md`
- Validation scorecard: `research/validation-scorecard.md`
- Customer discovery programme: `research/customer-discovery-programme.md`
- Opportunity portfolio: `research/opportunity-portfolio.md`
- Risk register: `research/risk-register.md`
- Project writeup (Analysis→Evaluation + ledger): `Docs/project-writeup.md`
- Data contracts: `Docs/data-contracts.md`
- Architecture: `Docs/architecture.md`
- Legal ingest: `Docs/legal-data-policy.md`
- Mix quality: `Docs/mix-quality-rubric.md`
- Genre defaults: `Docs/genre-policy.md`
- Competitive sweep: `Docs/competitive-automix-sweep.md`
- Getting started: `Docs/getting-started.md`
- Founder OS audit: `Docs/founder-os-audit.md`
