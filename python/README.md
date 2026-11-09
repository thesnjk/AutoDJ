# autodj_train

Synthetic transition generation, tracklist statistics ingest, and transition-class training stubs.

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
pytest autodj_train
autodj-train synth --out ./autodj_train/datasets/synthetic.jsonl --n 200
autodj-train aggregate-tracklists \
  --fixture autodj_train/datasets/fixtures/sample_tracklists.json \
  --fixture autodj_train/datasets/fixtures/curated_setcraft.json \
  --csv autodj_train/datasets/fixtures/sample_charts.csv \
  --out autodj_train/datasets/aggregates/demo_stats.json \
  --hints-out autodj_train/datasets/aggregates/demo_policy_hints.json
```

Feed hints into the mix CLI: `autodj mix --playlist party.json --policy-hints …/demo_policy_hints.json`.

Never place copyrighted commercial audio under this package. Offline JSON/CSV only — no live tracklist scrapes in CI.
