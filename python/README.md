# autodj_train

Synthetic transition generation, Softmax transition classifier, tracklist statistics ingest.

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
pytest autodj_train
ruff check .

# Synthetic pairs + train/eval (numpy SoftmaxLogReg — no torch required)
autodj-train synth --out ./autodj_train/datasets/synthetic.jsonl --n 200 --balance
autodj-train train --out autodj_train/models/artifacts/transition_model.json --n 2000 --seed 7
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
autodj-train predict --model autodj_train/models/artifacts/transition_model.json \
  --bpm-a 174 --bpm-b 175 --camelot-dist 1 --energy-delta-db 0 --genre drum_and_bass

# Tracklist aggregates → policy hints
autodj-train aggregate-tracklists \
  --fixture autodj_train/datasets/fixtures/sample_tracklists.json \
  --fixture autodj_train/datasets/fixtures/curated_setcraft.json \
  --csv autodj_train/datasets/fixtures/sample_charts.csv \
  --out autodj_train/datasets/aggregates/demo_stats.json \
  --hints-out autodj_train/datasets/aggregates/demo_policy_hints.json
```

Feed into the mix CLI:

```bash
autodj mix --playlist party.json --policy-hints …/demo_policy_hints.json
autodj mix --playlist party.json --transition-model …/transition_model.json
```

Never place copyrighted commercial audio under this package. Offline JSON/CSV only — no live tracklist scrapes in CI.
