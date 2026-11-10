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
 autodj-core (cue: per-track phrase + LCM join snap + drop_start) ──► slice buffers
        │
        ▼
 autodj-core (WSOLA + EQ xfade + R128) ──► eval spectral + join + bass + stretch + beat-phase/peak gates ──► WAV out
        │
        ▼
autodj_train: synth → SoftmaxLogReg JSON (v3) + tracklist aggregate → policy_hints
```

## Crates / app

- **autodj-io** — PCM containers + `slice_secs`.
- **autodj-core** — feature extraction (`AnalysisQuality`, BPM octave correction, `beat_offset_secs`, `spectral_centroid_hz`), Rekordbox XML ingest (`rekordbox.rs`), cue planner (`estimate_bars_per_phrase` + `phrase_bars_lcm` join snap + measured `cue_edge_phrase_aligned`; `plan_segment_cues` with `drop_start`), EQ / FX crossfades (EqSwap, FilterSweep, EchoOut, Tremolo, Riser; bass-overlap score), WSOLA (`stretch_clamp_excess`), R128, spectral + join-local + beat-phase eval (`eval.rs`; join indices + bass overlap from renderer). Pure Rust; primary target macOS Apple Silicon. Analysis is offline-only (not audio-thread safe).
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `pair_cost_edge` includes stretch-clamp excess + resolved-class penalty; `crossfade_secs_for_class` scales edge duration by class; `preserve_order` for setlist fidelity; `PolicyHintOverride` / `with_hints`; versioned `TransitionModel` Softmax (`predict_transition_class_edge` for v3 stretch + spectral + phrase bars).
- **autodj-cli** — user-facing analyze/mix; JSON or Rekordbox `.xml` via `--playlist`; `--playlist-name`; `--preserve-order`; `--policy-hints`; Softmax default on ordinary mix (`--transition-model` / `--no-transition-model`); `--deterministic` (incl. prep quality gate on `AnalysisQuality`); CI goldens `--golden smoke` / `rekordbox` / `transitions` / `transition_model` (six-genre Softmax E2E).
- **autodj-app** — Tauri 2 desktop shell; dialog picker for JSON/Rekordbox XML; MixReport QA panel (gate strip incl. prep + transitions); resolves committed Softmax when model path empty; forwards `--preserve-order` / `--playlist-name` / `--deterministic` to CLI via `AUTODJ_BIN` / PATH / `cargo run -p autodj-cli`; emits `autodj-progress` events. Excluded from the Cargo workspace (`workspace.exclude`).

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and softmax training (numpy only in CI). Aggregate `policy_hints` and versioned `transition_model.json` feed Rust via CLI. Softmax **v3** (default) uses 9 numerics + 6 genre one-hots (15 dims): BPM pair, |ΔBPM|, Camelot distance, energy ΔdB, `stretch_clamp_excess`, `spectral_centroid_delta_hz`, `bars_per_phrase_a`, `|bars_a−bars_b|`. Artifact field `version: 3` with **seven** class logits including `riser`; v1/v2 JSON, `transition_model_pre_tremolo.json` (5-class), and `transition_model_pre_riser.json` (6-class) still load via version/class dispatch. PyTorch never links into the CLI (optional extra only).
