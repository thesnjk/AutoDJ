# AutoDJ

Automatic DJ mix engine and learning pipeline. Analyze a library or playlist, solve a genre-aware mix graph (harmonic + tempo + energy arc + Softmax transition classes), and render a beat-aligned offline mix to **WAV or FLAC** with a MixReport QA sidecar. Desktop shell: Folder Automix → listen → Next matches / Automix Editor.

## Status

**P0–P86 shipped** on `main` — ungated competitive matrix closed; full-auto North Star ladder complete (**Radio libraries**, continuous radio, join-timed horizon remix, **set-length + vibe**); Docs honesty (**P84**–**P86**). Softmax 8-class FX, Rekordbox + cues, FLAC I/O, MixReport gates, Tauri shell, thesis-grade writeup + [`Docs/data-contracts.md`](Docs/data-contracts.md). Narrative: [`Docs/project-writeup.md`](Docs/project-writeup.md). Matrix: [`Docs/competitive-automix-sweep.md`](Docs/competitive-automix-sweep.md). Ladder: [`Docs/full-auto-roadmap.md`](Docs/full-auto-roadmap.md). Later/gated: stems, live dual-deck, neural phrase ML, streaming OAuth, open-web scrape.

## Quick start

```bash
# Analyze + mix royalty-free gold loops (+ MixReport sidecar)
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav

# Analyze a folder of wav/flac into the feature cache
cargo run -p autodj-cli -- analyze --input ./testdata/gold --db ./features.sqlite

# Mix from a playlist JSON (Softmax on by default)
cargo run -p autodj-cli -- mix --playlist ./testdata/party.json --out ./mix.wav --genre house

# Optional FLAC export / matching CLI
# cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.flac
# cargo run -p autodj-cli -- suggest --folder ./testdata/gold --from ./testdata/gold/loop_a_128.wav
```

Python train package:

```bash
cd python
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
pytest autodj_train
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
```

Desktop (macOS): see [`Docs/getting-started.md`](Docs/getting-started.md) — `cd apps/autodj-app && npm install && npm run tauri dev`.

## Docs

| Doc | Role |
|-----|------|
| [`Docs/project-writeup.md`](Docs/project-writeup.md) | Full professional writeup (Analysis→Design→Implementation→Testing→Evaluation) + capability ledger |
| [`Docs/full-auto-roadmap.md`](Docs/full-auto-roadmap.md) | North Star + P80–P83 phase ladder |
| [`Docs/developer-map.md`](Docs/developer-map.md) | Layout, tests, hard rules |
| [`Docs/getting-started.md`](Docs/getting-started.md) | First mix, CLI flags, Tauri |
| [`Docs/architecture.md`](Docs/architecture.md) | Pipeline + crate roles |
| [`Docs/mix-quality-rubric.md`](Docs/mix-quality-rubric.md) | Automated + listening gates |
| [`Docs/competitive-automix-sweep.md`](Docs/competitive-automix-sweep.md) | Competitive matrix + P-rows |
| [`Docs/genre-policy.md`](Docs/genre-policy.md) | Builtin genre defaults |
| [`Docs/legal-data-policy.md`](Docs/legal-data-policy.md) | Lawful ingest rules |

## License

MIT — see `LICENSE`. Do not commit copyrighted commercial audio or scraped full DJ sets. See [`Docs/legal-data-policy.md`](Docs/legal-data-policy.md).
