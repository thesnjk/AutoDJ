# Mix quality rubric

Automated gates (CI) plus listening checklist for v1+.

## Automated (must pass)

| Check | Pass criterion |
|-------|----------------|
| Golden smoke CLI | `autodj mix --golden smoke` exits 0 and writes WAV |
| Peak limit | True-peak proxy ≤ 0.99 full scale |
| Integrated loudness (R128 / BS.1770) | After normalize, `integrated_lufs` within ±2 LU of `--lufs` (golden default −14 LUFS; use `--lufs -23` for classic EBU R 128). Evidence: `loudness.rs`, MixReport `loudness_gate_pass` |
| Phrase / section cues | MixReport `cue_phrase_aligned: true`; cue windows from `plan_segment_cues` (evidence: `cue.rs`, unit snap tests). Crossfade durations snapped to whole beats. |
| Beat-phase (v1) | Mean downbeat offset across transitions < 50 ms on gold loops — improved by WSOLA tempo blend on multi-BPM joins; unit coverage via `align_crossfade_to_downbeat` / `snap_to_phrase` / `stretch.rs` |
| Tempo blend (multi-BPM) | Golden smoke (128→140) reports `tempo_blend_applied: true` (evidence: `stretch.rs`, `mix_buffers_with_meta`) |
| Spectral continuity (v1) | Adjacent hop-RMS ratio ≤ 4.0 (`DEFAULT_MAX_FLUX_RATIO`); MixReport `spectral_continuity_pass` (evidence: `eval.rs`). Buffers with fewer than 8 hops skip-with-pass |
| Solver unit tests | Harmonic neighbor preferred; energy-arc quiet opener + rise/fall (`autodj-graph` tests) |
| EQ / bass-swap DSP | `eq_swap_gains` + `apply_eq_swap_crossfade` unit tests; house golden uses `EqSwap` (evidence: `eq.rs`, `render.rs`) |
| Transition classifier (train gate) | `autodj-train eval` accuracy ≥ 0.90 on `eval/gold/synthetic_holdout.jsonl` (evidence: `SoftmaxLogReg`, CI python job). Golden WAV still uses genre `preferred_transition` unless `--transition-model` is passed |
| Gold analyze (BPM/key/sections) | `testdata/gold/*.wav` BPM within ±1 of `expected_analyze.json`; frozen key string; `intro_end < drop_start < outro_start`; not sparse (evidence: `analysis::tests::gold_analyze_matches_expected`) |
| macOS native CI | Rust job on `macos-latest` (Apple Silicon hosted) plus `ubuntu-latest`; local `aarch64-apple-darwin` verify required when hosted Mac runners are unavailable |

## Listening checklist (human)

- [ ] Transitions feel phrase-aware on real tracks (automated planner covers gold; listen for awkward early cuts)
- [ ] Bass conflict avoided during EQ-swap blends (automated bass ducking in `eq.rs`; listen for mud on real vinyl rips)
- [ ] Energy arc matches genre (house long blend vs DnB snap)
- [ ] No obvious clicks/pops at join points
- [ ] Subjective loudness even across the set (automated gate covers ±2 LU; listen for pumping / over-limiting)
- [ ] Large BPM gaps (±15%+) — WSOLA may sound warbly; prefer solver avoid or listen-test |

Fail any automated row → block merge. Listening fails → open follow-up issue, do not paper over with doc-only checkboxes.
