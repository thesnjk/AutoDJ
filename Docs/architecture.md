# Architecture

```
playlist JSON / Rekordbox XML ──► apps/autodj-app (Tauri) ──┐
        │                                                    │
        ▼                                                    ▼
 autodj-io ──► autodj-core (analyze + rekordbox overrides) ──► SQLite feature cache
        │
        ▼
 autodj-graph (solve_order or preserve_order + TransitionModel)
        │
        ▼
 autodj-core (cue: per-track phrase + beat-grid snap + drop_start) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA + EQ xfade + R128) ──► eval spectral + join + bass + stretch + beat-phase/peak gates ──► WAV out
        │
        ▼
autodj_train: synth → SoftmaxLogReg JSON (v2) + tracklist aggregate → policy_hints
```

## Crates / app

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction (`AnalysisQuality`, BPM octave correction, `beat_offset_secs`, `spectral_centroid_hz`), Rekordbox XML ingest (`rekordbox.rs`), cue planner (`estimate_bars_per_phrase` + `plan_segment_cues` with `drop_start`), EQ (incl. bass-overlap score), WSOLA (`stretch_clamp_excess`), R128, spectral + join-local + beat-phase eval (`eval.rs`; join indices + bass overlap from renderer). Pure Rust; primary target macOS Apple Silicon. Analysis is offline-only (not audio-thread safe).
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `pair_cost_edge` includes stretch-clamp excess + resolved-class penalty; `crossfade_secs_for_class` scales edge duration by class; `preserve_order` for setlist fidelity; `PolicyHintOverride` / `with_hints`; versioned `TransitionModel` Softmax (`predict_transition_class_edge` for v2 stretch + spectral delta).
- **autodj-cli** — user-facing analyze/mix; JSON or Rekordbox `.xml` via `--playlist`; `--playlist-name`; `--preserve-order`; `--policy-hints`; Softmax default on ordinary mix (`--transition-model` / `--no-transition-model`); `--deterministic` (incl. prep quality gate on `AnalysisQuality`); CI goldens `--golden smoke` / `rekordbox` / `transitions` / `transition_model` (six-genre Softmax E2E).
- **autodj-app** — Tauri 2 desktop shell; dialog picker for JSON/Rekordbox XML; MixReport QA panel (gate strip incl. prep + transitions); resolves committed Softmax when model path empty; forwards `--preserve-order` / `--playlist-name` / `--deterministic` to CLI via `AUTODJ_BIN` / PATH / `cargo run -p autodj-cli`; emits `autodj-progress` events. Excluded from the Cargo workspace (`workspace.exclude`).

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and softmax training (numpy only in CI). Aggregate `policy_hints` and versioned `transition_model.json` feed Rust via CLI. Softmax **v2** (default) uses 7 numerics + 6 genre one-hots (13 dims): BPM pair, |ΔBPM|, Camelot distance, energy ΔdB, `stretch_clamp_excess`, `spectral_centroid_delta_hz`. Artifact field `version: 2`; v1 JSON (11 dims) still predicts via version dispatch; `transition_model_v1.json` keeps the pre-v2 snapshot. PyTorch never links into the CLI (optional extra only).
