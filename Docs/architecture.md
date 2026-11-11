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

- **autodj-io** — WAV/FLAC decode (`read_audio` / `claxon`) + WAV encode; PCM containers + `slice_secs`.
- **autodj-core** — feature extraction (`AnalysisQuality`, BPM octave correction, `beat_offset_secs`, FFT hop `spectral_centroid_hz` via `realfft`), Rekordbox XML ingest (`rekordbox.rs`; Type 1/2 → absolute `mix_in_secs`/`mix_out_secs`), cue planner (`estimate_bars_per_phrase` + `phrase_bars_lcm` join snap + measured `cue_edge_phrase_aligned`; `plan_segment_cues` with `drop_start` + `CueOverrides` mix-in/out), EQ / FX crossfades (EqSwap, FilterSweep, EchoOut, Tremolo, Riser, Dissolve; bass-overlap score), bidirectional WSOLA tempo blend (`tempo_blend_head` + `tempo_blend_tail`; `stretch_clamp_excess`), R128, spectral + join-local + beat-phase eval (`eval.rs`; join indices + bass overlap from renderer); `segment_start_indices` for mix-timeline UI. Pure Rust; primary target macOS Apple Silicon. Analysis is offline-only (not audio-thread safe).
- **autodj-graph** — greedy NN solver with energy-arc phases + genre policy weights; `pair_cost_edge` includes `key_pair_cost` (Camelot + `HARMONIC_ROUTE_DISCOUNT` when `suggest_next_keys`), stretch-clamp excess + resolved-class penalty; `rank_next_matches` / `build_remix_order` for Automix Matching; `crossfade_secs_for_class` scales edge duration by class; `preserve_order` for setlist fidelity; `PolicyHintOverride` / `with_hints`; versioned `TransitionModel` Softmax (`predict_transition_class_edge` for v3 stretch + spectral + phrase bars); `apply_transition_overrides` for playlist `transition_out` pins.
- **autodj-cli** — user-facing analyze/mix; JSON or Rekordbox `.xml` via `--playlist`; `--playlist-name`; `--preserve-order`; `--policy-hints`; Softmax default on ordinary mix (`--transition-model` / `--no-transition-model`); `--deterministic` (incl. prep quality gate on `AnalysisQuality`); MixReport sidecar `{out}.mixreport.json` (`--report-out` override) plus stdout JSON; CI goldens `--golden smoke` / `broadcast` (−23 LUFS) / `policy_hints` / `rekordbox` / `rekordbox_cue_overrides` / `cue_overrides` / `transitions` / `transition_model` (six-genre Softmax E2E).
- **autodj-app** — Tauri 2 desktop shell; primary **Automix folder** (`automix_folder`: scan wav/flac → temp playlist → optional analyze `--db` → mix); **Next matches** (`suggest_matches`) + **Queue & remix** (`remix_queue_next`) from now-playing; JSON/Rekordbox playlist remains advanced (Policy hints Browse + Load demo via `resolve_demo_policy_hints`); Mix & Listen HTML5 autoplay of `MixReport.out` via `convertFileSrc` + asset protocol (`/tmp/**`, `$HOME/**`); **Mix Now** seek via `join_timeline_secs` (button + `N` / `Shift+N` keyboard); cue-segment rail from `cues[].mix_timeline_*` plus join markers; playhead lights active/next join + Transitions row and a class listen hint within ±4 s of a join; now-playing highlight from cue windows; Set order (AutoDJ) from `MixReport.order` with cue Camelot/BPM/`bars_per_phrase` and prep trusted/weak pills from `prep_tracks`; Transitions show key route / ΔBPM / edge cost and click-to-seek joins; **Automix Editor** (`mix_editor_remix` / `export_editor_playlist`) seeds mix-in/out + `transition_out` + `bars_per_phrase` from MixReport cues/transitions then re-mixes with `--preserve-order` or saves CLI playlist JSON (wires Advanced `#playlist` + preserve-order); MixReport QA panel with gate→rubric tooltips, override/phrase flags, Reveal MixReport sidecar, + Listen while playing checklist; resolves committed Softmax when model path empty; forwards `--db` / `--preserve-order` / `--playlist-name` / `--deterministic` / `--lufs` (when Strict QA off) to CLI via `AUTODJ_BIN` / PATH / `cargo run -p autodj-cli`; emits `autodj-progress` events. Excluded from the Cargo workspace (`workspace.exclude`).

## Train / runtime split

Python owns dataset synthesis, offline tracklist/chart aggregates, and softmax training (numpy only in CI). Aggregate `policy_hints` and versioned `transition_model.json` feed Rust via CLI. Softmax **v3** (default) uses 9 numerics + 6 genre one-hots (15 dims): BPM pair, |ΔBPM|, Camelot distance, energy ΔdB, `stretch_clamp_excess`, `spectral_centroid_delta_hz`, `bars_per_phrase_a`, `|bars_a−bars_b|`. Artifact field `version: 3` with **eight** class logits including `dissolve`; v1/v2 JSON, `transition_model_pre_tremolo.json` (5-class), `transition_model_pre_riser.json` (6-class), and `transition_model_pre_dissolve.json` (7-class) still load via version/class dispatch. PyTorch never links into the CLI (optional extra only).
