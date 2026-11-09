# autodj_train

Synthetic transition generation, tracklist statistics ingest, and transition-class training stubs.

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
pytest autodj_train
autodj-train synth --out ./autodj_train/datasets/synthetic.jsonl --n 200
```

Never place copyrighted commercial audio under this package.
