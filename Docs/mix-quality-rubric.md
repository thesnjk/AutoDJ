# Mix quality rubric

Automated gates (CI) plus listening checklist for v1+.

## Automated (must pass)

| Check | Pass criterion |
|-------|----------------|
| Golden smoke CLI | `autodj mix --golden smoke` exits 0 and writes WAV + `{out}.mixreport.json` sidecar (CI asserts file exists) |
| Golden broadcast CLI | `autodj mix --golden broadcast` exits 0; MixReport `target_lufs` −23 (`BROADCAST_TARGET_LUFS`), `loudness_gate_pass`, `integrated_lufs` within ±2 LU |
| Golden rekordbox CLI | `autodj mix --golden rekordbox` exits 0; MixReport `preserve_order: true`; setlist order a→b→c from `testdata/rekordbox/gold_playlist.xml` (hard-fail if `preserve_order` returns non-sequential indices) |
| Golden transitions CLI | `autodj mix --golden transitions` exits 0; all six builtins (house/trance/hip_hop/drum_and_bass/uk_garage/open_format) edges match `preferred_transition` and class-aware `policy_crossfade_secs` via `crossfade_secs_for_class` (trance `riser` / open_format `tremolo` / uk_garage `dissolve` / DnB 0 / hip_hop ~1 / house 8 / …) |
| Golden transition_model CLI | `autodj mix --golden transition_model` exits 0; all six builtins preserve-order edges match Softmax `predict_transition_class_edge` on committed v3 `transition_model.json` (stretch + spectral + phrase bars; not `preferred_transition`) |
| Golden cue_overrides CLI | `autodj mix --golden cue_overrides` exits 0; MixReport `cue_mix_override_applied`; each cue `play_start_secs`/`play_end_secs` matches playlist `mix_in_secs`/`mix_out_secs` within ≤½ beat |
| Golden rekordbox_cue_overrides CLI | `autodj mix --golden rekordbox_cue_overrides` exits 0; preserve-order MixReport `cue_mix_override_applied`; Type 1/2 from `testdata/rekordbox/cue_overrides_playlist.xml` pin play windows within ≤½ beat |
| Golden transition_overrides CLI | `autodj mix --golden transition_overrides` exits 0; MixReport `transition_override_applied`; overridden edges match playlist `transition_out` + class-aware `policy_crossfade_secs` (HardCut 0) |
| Golden policy_hints CLI | `autodj mix --golden policy_hints` exits 0; house MixReport edges keep `eq_swap` with hinted `policy_crossfade_secs` ≈ 12s (builtin without hints remains 8s via `--golden transitions`) |
| Peak limit | True-peak ≤ 0.99 full scale; MixReport `true_peak` + `peak_gate_pass` (hard-fail when `--deterministic`, default). Evidence: `loudness.rs`, golden smoke |
| Integrated loudness (R128 / BS.1770) | After normalize, `integrated_lufs` within ±2 LU of `--lufs` (most goldens lock −14 LUFS; `--golden broadcast` locks −23). Evidence: `loudness.rs`, MixReport `loudness_gate_pass`; Tauri Streaming/Broadcast presets when Strict QA off |
| Phrase / section cues | MixReport `cue_phrase_aligned` (measured residual via `cue_edge_phrase_aligned`; short loops skip-with-pass) + `cue_drop_aware` + `per_track_phrase_applied` + optional `cue_mix_override_applied`; per-track `cues[].bars_per_phrase ∈ {4,8,16}`; outgoing joins use `phrase_bars_lcm` when both tracks phrase-snap (skipped when absolute overrides set); drop-aware mix-in/out (evidence: `cue.rs`, golden smoke / `cue_overrides`). Crossfade durations snapped to whole beats. |
| Beat-phase (v1) | Mean same-tempo join offset &lt; 50 ms; MixReport `beat_phase_mean_ms` + `beat_phase_gate_pass` (hard-fail when `--deterministic`). Evidence: `TrackFeatures.beat_offset_secs`, `mean_beat_phase_offset_ms`, golden smoke |
| Tempo blend (multi-BPM) | Golden smoke (128→140) reports `tempo_blend_applied: true` via bidirectional WSOLA (`tempo_blend_head` + `tempo_blend_tail`; evidence: `stretch.rs`, `mix_buffers_with_meta`) |
| Tempo stretch clamp (warble) | Worst edge `stretch_clamp_excess` == 0 (ratio inside ±15%); MixReport `max_stretch_clamp_excess` + `tempo_stretch_gate_pass` (hard-fail when `--deterministic`). Solver costs clamp-excess edges. Evidence: `stretch.rs`, `pair_cost_edge` |
| Spectral continuity (v1) | Adjacent hop-RMS ratio ≤ 4.0 (`DEFAULT_MAX_FLUX_RATIO`); MixReport `spectral_continuity_pass` (evidence: `eval.rs`). Buffers with fewer than 8 hops skip-with-pass |
| Join continuity (clicks/pops) | Worst join-neighborhood hop-RMS ratio ≤ 6.0 (`DEFAULT_JOIN_FLUX_RATIO`); MixReport `max_join_flux_ratio` + `join_continuity_pass` (hard-fail when `--deterministic`). Hard cuts (`xfade=0`) skipped. Evidence: `eval.rs`, `MixRenderResult.join_sample_indices` |
| Bass conflict (EqSwap) | Worst EqSwap concurrent low-band ownership ≤ 0.40 (`DEFAULT_BASS_OVERLAP_LIMIT`); MixReport `max_bass_overlap` + `bass_conflict_gate_pass` (hard-fail when `--deterministic`). Non-EqSwap joins skipped. Evidence: `eq.rs` `max_eq_swap_bass_overlap`, renderer |
| Solver unit tests | Harmonic neighbor preferred; key-route discount (`key_pair_cost` / `suggest_next_keys`) beats closer-BPM off-route; energy-arc quiet opener + rise/fall (`autodj-graph` tests) |
| EQ / bass-swap DSP | `eq_swap_gains` + `apply_eq_swap_crossfade` unit tests; house golden uses `EqSwap` (evidence: `eq.rs`, `render.rs`) |
| Dissolve FX DSP | `apply_dissolve_out_crossfade` staggered band-kill unit test; uk_garage golden uses `Dissolve` (evidence: `eq.rs`, `render.rs`) |
| Transition classifier (train gate) | `autodj-train eval` accuracy ≥ 0.90 on `eval/gold/synthetic_holdout.jsonl` (evidence: SoftmaxLogReg v3 15-dim **8-class** incl. dissolve, CI python job). Ordinary `mix` defaults to committed Softmax; v1/v2 JSON still load; smoke/rekordbox/transitions goldens keep `preferred_transition` (`--no-transition-model`) |
| Gold analyze (BPM/key/sections) | `testdata/gold/*.wav` BPM within ±1 of `expected_analyze.json`; frozen key string; `intro_end < drop_start < outro_start`; not sparse (evidence: `analysis::tests::gold_analyze_matches_expected`) |
| Spectral centroid (FFT) | Hop-averaged magnitude centroid (Hann 2048 / hop 1024 via `realfft`); 440 Hz tone within ±8%; silence → 0 (evidence: `analysis::tests::spectral_centroid_*`) |
| Mix timeline (Mix Now) | MixReport `join_timeline_secs` + cue `mix_timeline_start_secs`/`end_secs` from renderer `segment_start_indices` (evidence: smoke JSON; Tauri Mix Now / `N`·`Shift+N` / set-order seek / join markers) |
| Automix Matching | `rank_next_matches` prefers key-route over closer-BPM off-route; `build_remix_order` pins forced next after prefix (evidence: `autodj-graph` tests; CLI `suggest` / `remix`) |
| Set-order harmonic readout | MixReport `cues[].key` + transition `key_from`/`key_to`/`camelot_distance`/`bpm_delta`/`key_on_route`/`edge_cost` (evidence: smoke JSON; Tauri Set order / Transitions) |
| Override / phrase UI parity | MixReport `cue_mix_override_applied` / `transition_override_applied` / `per_track_phrase_applied` + Set order `bars_per_phrase`; Automix Editor seeds mix-in/out + edge class from cues/transitions (evidence: cue_overrides / transition_overrides goldens; Tauri gate strip / `seedEditorDraft`) |
| Prep quality (AnalysisQuality) | MixReport `prep_quality_gate_pass` + `prep_tracks[]`; each track trusted when `!sparse && bpm_confidence ≥ 0.25` **or** playlist BPM override (JSON / Rekordbox). Hard-fail when `--deterministic`. Evidence: `cmd_mix`, golden smoke/rekordbox; Tauri Set order trusted/weak pills |
| macOS native CI | Rust job on `macos-latest` (Apple Silicon hosted) plus `ubuntu-latest`; local `aarch64-apple-darwin` verify required when hosted Mac runners are unavailable |

## Listening checklist (human)

- [ ] Transitions feel phrase-aware on real tracks (automated planner covers gold; listen for awkward early cuts)
- [ ] Bass conflict avoided during EQ-swap blends (automated bass-overlap gate covers gold; listen for mud on real vinyl rips)
- [ ] Energy arc matches genre (house long blend vs DnB snap)
- [ ] No obvious clicks/pops at join points (automated join-flux gate covers gold; listen for residual on real vinyl)
- [ ] Subjective loudness even across the set (automated gate covers ±2 LU; listen for pumping / over-limiting)
- [ ] Large BPM gaps (±15%+) — automated stretch-clamp gate covers gold; listen for residual warble on real sets |

Tauri MixReport panel: gate pills carry rubric tooltips; Strict QA fail callout lists failed hard gates; **Listen while playing** mirrors this checklist (evidence: `apps/autodj-app/src/main.ts`).

Fail any automated row → block merge. Listening fails → open follow-up issue, do not paper over with doc-only checkboxes.
