# Competitive automix sweep

Living gap matrix. Updated 2026-11-09 after Firecrawl research (raw notes in local `.firecrawl/`, not committed). Every Gap needs an evidence path.

## Surface matrix

| Surface | Competitor cue | AutoDJ status | Evidence |
|---------|----------------|---------------|----------|
| Offline automix render | djay Automix (playlist → continuous blend; Mix Now / cue points) | Parity-path MVP | `autodj-cli` `mix`; `crates/autodj-core/src/render.rs` |
| Feature extraction (BPM/key/sections) | Rekordbox / djay prep analysis | Parity-path | `analysis.rs` `AnalysisQuality` + octave fold; MixReport `prep_quality_gate_pass`; gold `expected_analyze.json` regression |
| Harmonic suggestions | Rekordbox / DJ.Studio Camelot ordering | Parity MVP | `camelot.rs`; `autodj-graph` `prefers_harmonic_neighbor` |
| Energy-arc setcraft | Club open→peak→cool programming | Parity-path | `ArcPhase` + `pair_cost_at` + quiet opener; MixReport order |
| Transition type palette | djay Filter / Echo / Tremolo / Riser / Neural Mix; SetFlow six-core (EQ, bass swap, cut, blend) | Parity-path | `eq.rs` Riser + Tremolo + Filter/Echo/EqSwap; Softmax 7-class incl. riser; `trance` preferred Riser / `open_format` Tremolo; CI `--golden transitions` / `transition_model`; Neural Mix / stems Later |
| Mix-in / mix-out cueing | VirtualDJ Smart automix (outro→intro ~4–8s); djay start/end points | Parity-path | `cue.rs` `plan_segment_cues` + `drop_start`; MixReport `cue_phrase_aligned` + `cue_drop_aware` |
| Library prep (Rekordbox XML) | Rekordbox playlist export → DJ.Studio | Parity-path | `rekordbox.rs` + `--playlist` `.xml`; BPM/key/grid/cue overrides; `--preserve-order`; CI `--golden rekordbox` |
| Stems-aware transitions | Serato Stems (vocal / melody / bass / drums pads) | Gap | no stem DSP; deferred v2 |
| Phrase sync | Rekordbox phrase analysis / djay BPM+Beats sync | Parity-path | per-track `bars_per_phrase` + LCM join snap + measured `cue_phrase_aligned` + Softmax v3 phrase bars/mismatch |
| Learning from setcraft | Mixonset / tracklist research tools | Parity-path | SoftmaxLogReg v3 15-dim + **7 classes** (incl. riser); `autodj-train eval` ≥ 0.90; CI `--golden transition_model`; v1/v2 JSON still load |
| Live / controller | VirtualDJ / Serato dual-deck | Parity-path (desktop shell) | Tauri 2 MixReport QA panel (gate strip + transitions) + deterministic checkbox; live dual-deck still Later |
| Loudness / broadcast | EBU R 128 (−23 LUFS integrated; true-peak limits); radio-style continuous sets | Parity-path (measure+norm shipped) | `loudness.rs` + CLI `--lufs`; golden gate ±2 LU (`Docs/mix-quality-rubric.md`) |
| Tempo blend | djay Sync + Tempo Blend | Parity-path | `stretch.rs` WSOLA `tempo_blend_head` + `tempo_blend_tail` (bidirectional); MixReport `tempo_blend_applied` + `tempo_stretch_gate_pass`; solver `stretch_clamp_excess` cost |
| Spectral continuity eval | Mix-quality rubric adjacent flux | Parity-path | `eval.rs` hop-RMS ratio; MixReport `spectral_continuity_pass` + `join_continuity_pass` |

## Citations (paraphrase + link only)

- [djay Automix user manual](https://help.algoriddim.com/user-manual/djay-pro-windows/mixing-basics/using-automix) — transition FX palette, tempo adjust, Mix Now.
- [VirtualDJ Automix](https://virtualdj.com/manuals/virtualdj/interface/browser/sideview/automix.html) — Smart mix-in/out, fade/cut types, Automix Editor.
- [Serato Using Stems](https://support.serato.com/hc/en-us/articles/5700921209615-Using-Stems) — stem pads for vocal/bass/drums isolation.
- [Rekordbox × DJ.Studio prep](https://rekordbox.com/en/connect/djstudio/dj-set-preparation/) — Camelot ordering + phrase mastery messaging.
- [SetFlow transition technique](https://www.setflow.app/blog/dj-transitions-how-to-mix-between-songs) — six core transitions (EQ / bass / filter / echo / cut / blend).
- [EBU R 128](https://en.wikipedia.org/wiki/EBU_R_128) — programme loudness normalization (BS.1770 family).

## Gates / skips

- Pacemaker product marketing pages: thin public technical signal; use EBU R 128 / BS.1770 as the broadcast loudness target instead of product ToS scrape.
- Mixonset: no stable public API docs scraped this pass; keep train path synthetic-only until a lawful ingest path is documented.
- YouTube full-set scrapes: **forbidden** per `Docs/legal-data-policy.md`.
- Live 1001Tracklists / Beatport HTTP clients: **gated** (ToS / anti-bot / paid API); use offline JSON/CSV fixtures + curated citations only.
- Tauri CI on ubuntu WebKit: **deferred** this pass — Mac-local `npm run tauri dev` / `cargo check -p` via `apps/autodj-app/src-tauri`; existing Rust/python CI unchanged.

## Persona re-rank (mix_impact × gap)

Formula note: `(competitor best − AutoDJ) × persona weight × mix_impact`. Bedroom DJ / content creator and radio/podcast continuous mix weighted highest this session.

| Priority | Gap | Why |
|----------|-----|-----|
| **P0** | EBU R128 normalize + CI loudness gate | **Shipped** — keep regression via golden `loudness_gate_pass` |
| **P1** | Phrase / section-aware cue points | **Shipped** — `cue.rs` + energy section marks; keep `cue_phrase_aligned` in golden JSON |
| **P2** | 3-band EQ / bass-swap transition classes | **Shipped** — `eq.rs` + renderer wiring; `macos-latest` CI + local Apple Silicon verify |
| **P3** | Tempo blend / stretch | **Shipped** — WSOLA tempo blend on xfade heads; keep `tempo_blend_applied` on golden |
| **P4** | Solver energy arc + spectral continuity gate | **Shipped** — `ArcPhase` solver + `eval.rs` golden gate |
| **P5** | Tracklist ingest → policy hints | **Shipped** — offline JSON/CSV aggregate + `--policy-hints` |
| **P6** | Train stub / transition classifier | **Shipped** — SoftmaxLogReg + gold holdout + `--transition-model` |
| **P7** | Tauri desktop shell | **Shipped** — `apps/autodj-app` playlist → progress → export over CLI |
| **P8** | Feature extraction hardening | **Shipped** — BPM octave fold + `AnalysisQuality` + gold analyze gate |
| **P9** | Beat grid + beat-phase / peak CI gates | **Shipped** — `beat_offset_secs` + MixReport `beat_phase_gate_pass` / `peak_gate_pass` |
| **P10** | Rekordbox XML ingest | **Shipped** — `rekordbox.rs` + `--preserve-order` + prep metadata overrides |
| **P11** | Tauri XML file picker polish | **Shipped** — dialog plugin + preserve-order / playlist-name controls |
| **P12** | Join-local continuity gate | **Shipped** — `join_sample_indices` + MixReport `join_continuity_pass` |
| **P13** | Rekordbox full-mix CI golden | **Shipped** — `--golden rekordbox` + preserve-order integrity + CI step |
| **P14** | EqSwap bass-overlap gate | **Shipped** — `max_bass_overlap` + MixReport `bass_conflict_gate_pass` |
| **P15** | Multi-genre transition CI golden | **Shipped** — `--golden transitions` + MixReport `transitions` |
| **P16** | Transition-model E2E mix CI golden | **Shipped** — `--golden transition_model` + Softmax class pin |
| **P17** | Warble-aware stretch costs + gate | **Shipped** — `stretch_clamp_excess` in solver + MixReport gate |
| **P18** | uk_garage / open_format transition golden | **Shipped** — `--golden transitions` covers all six builtins |
| **P19** | Tauri MixReport / QA panel | **Shipped** — gate strip + transitions + `--deterministic` bridge |
| **P20** | Default Softmax per-edge mixes | **Shipped** — ordinary mix auto-loads committed Softmax; `--no-transition-model`; six-genre `--golden transition_model` |
| **P21** | Drop/section-aware cue planning | **Shipped** — `plan_segment_cues` uses `drop_start`; MixReport `cue_drop_aware` |
| **P22** | Class-aware crossfade durations | **Shipped** — `crossfade_secs_for_class`; HardCut 0; EchoOut short; golden policy asserts |
| **P23** | Prep quality AnalysisQuality mix gates | **Shipped** — MixReport `prep_quality_gate_pass` + deterministic bail; Tauri prep pill |
| **P24** | Per-track phrase length heuristic | **Shipped** — `estimate_bars_per_phrase`; MixReport `per_track_phrase_applied` + `cues[].bars_per_phrase` |
| **P25** | Softmax v2 stretch + spectral features | **Shipped** — 13-dim vectorize; artifact `version: 2`; `predict_transition_class_edge`; `transition_model_v1.json` snapshot |
| **P26** | Cross-track phrase join + honest gates | **Shipped** — `phrase_bars_lcm` join snap; measured `cue_phrase_aligned` (short loops skip-with-pass) |
| **P27** | Softmax v3 phrase bars features | **Shipped** — 15-dim vectorize; artifact `version: 3`; `transition_model_v2.json` snapshot; v1/v2 loaders |
| **P28** | Tremolo transition FX | **Shipped** — `apply_tremolo_out_crossfade`; `open_format` preferred; Softmax stays 5-class |
| **P29** | Softmax 6-class Tremolo | **Shipped** — `tremolo` in Softmax `classes`; holdout regen; ordinary mix can select Tremolo |
| **P30** | Riser transition FX | **Shipped** — `apply_riser_out_crossfade`; `trance` preferred; Softmax stays 6-class |
| **P31** | Softmax 7-class Riser | **Shipped** — `riser` in Softmax `classes`; holdout regen; ordinary mix can select Riser |
| **P32** | Bidirectional tempo blend | **Shipped** — `tempo_blend_tail` + head; outgoing morphs toward incoming BPM |
| Next | Stems / live dual-deck | gated (rights-cleared stems path required) |
| Later | Mix-in/out cue overrides; neural phrase ML; listening-rubric polish | user-owned audio / playlist schema |

## Re-rank bias

Prefer agent-doable product code that closes remaining mix-quality gaps before brochure fluff. P0–P32 are in place; stems remain gated.
