# AutoDJ

Automatic DJ mix engine and learning pipeline. Analyze a playlist, solve a genre-aware mix graph (harmonic + tempo + energy arc), and render a beat-aligned offline mix.

## Status

MVP scaffold: feature extraction, playlist solver, offline renderer, CLI golden smoke, synthetic transition training stub, tracklist stats ingest.

## Quick start

```bash
# Analyze + mix royalty-free gold loops
cargo run -p autodj-cli -- mix --golden smoke

# Analyze a folder of wav/flac into the feature cache
cargo run -p autodj-cli -- analyze --input ./testdata/gold --db ./features.sqlite

# Mix from a playlist JSON
cargo run -p autodj-cli -- mix --playlist ./testdata/party.json --out ./mix.wav --genre house
```

Python train package:

```bash
cd python
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
pytest autodj_train
```

## Layout

See [`Docs/developer-map.md`](Docs/developer-map.md).

## License

MIT — see `LICENSE`. Do not commit copyrighted commercial audio or scraped full DJ sets. See [`Docs/legal-data-policy.md`](Docs/legal-data-policy.md).
