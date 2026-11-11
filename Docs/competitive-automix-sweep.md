# Competitive automix sweep

Living gap matrix. Updated 2026-11-09 after Firecrawl research (raw notes in local `.firecrawl/`, not committed). Every Gap needs an evidence path.

## Surface matrix

| Surface | Competitor cue | AutoDJ status | Evidence |
|---------|----------------|---------------|----------|
| Offline automix render | djay Automix (playlist → continuous blend; Mix Now / cue points) | Parity-path MVP | `autodj-cli` `mix`; MixReport sidecar `{out}.mixreport.json`; `crates/autodj-core/src/render.rs` |
| Feature extraction (BPM/key/sections) | Rekordbox / djay prep analysis | Parity-path | `analysis.rs` `AnalysisQuality` + octave fold; FFT hop spectral centroid (`spectral_centroid_hz`); MixReport `prep_quality_gate_pass`; gold `expected_analyze.json` regression |
| Harmonic suggestions | Rekordbox / DJ.Studio Camelot ordering | Parity MVP | `camelot.rs`; `key_pair_cost` / `suggest_next_keys`; `prefers_harmonic_neighbor` |
| Energy-arc setcraft | Club open→peak→cool programming | Parity-path | `ArcPhase` + `pair_cost_at` + quiet opener; MixReport order |
| Transition type palette | djay Filter / Echo / Tremolo / Riser / Dissolve / Neural Mix; SetFlow six-core (EQ, bass swap, cut, blend) | Parity-path | `eq.rs` Dissolve + Riser + Tremolo + Filter/Echo/EqSwap; Softmax 8-class (incl. dissolve); playlist `transition_out`; CI `--golden transitions` / `transition_model` / `transition_overrides`; Neural Mix / stems Later |
| Mix-in / mix-out cueing | VirtualDJ Smart automix (outro→intro ~4–8s); djay start/end points | Parity-path | `cue.rs` `plan_segment_cues` + `CueOverrides` (`mix_in_secs`/`mix_out_secs`); MixReport `cue_mix_override_applied` + `cue_phrase_aligned`; CI `--golden cue_overrides` / `rekordbox_cue_overrides` |
| Library prep (Rekordbox XML) | Rekordbox playlist export → DJ.Studio | Parity-path | `rekordbox.rs` + `--playlist` `.xml`; BPM/key/grid; Type 1/2 → absolute mix windows; `--preserve-order`; CI `--golden rekordbox` / `rekordbox_cue_overrides` |
| Stems-aware transitions | Serato Stems (vocal / melody / bass / drums pads) | Gap | no stem DSP; deferred v2 |
| Phrase sync | Rekordbox phrase analysis / djay BPM+Beats sync | Parity-path | per-track `bars_per_phrase` + LCM join snap + measured `cue_phrase_aligned` + Softmax v3 phrase bars/mismatch; Automix Editor 4/8/16 pin |
| Learning from setcraft | Mixonset / tracklist research tools | Parity-path | SoftmaxLogReg v3 15-dim + **8 classes** (incl. dissolve); `autodj-train eval` ≥ 0.90; CI `--golden transition_model` / `policy_hints`; Tauri Browse + Load demo for `demo_policy_hints.json` |
| Live / controller | VirtualDJ / Serato dual-deck; djay Automix play / Mix Now | Parity-path (desktop shell) | Tauri Folder Automix + Mix & Listen + Mix Now seek (`join_timeline_secs`); Automix Editor seeded from MixReport cues/transitions; live dual-deck still Later |
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
| **P33** | Mix-in/out cue overrides | **Shipped** — playlist `mix_in_secs`/`mix_out_secs`; `--golden cue_overrides` |
| **P34** | Per-edge transition overrides | **Shipped** — playlist `transition_out`; `--golden transition_overrides` |
| **P35** | Dissolve transition FX | **Shipped** — `apply_dissolve_out_crossfade`; `uk_garage` preferred; Softmax stays 7-class |
| **P36** | Softmax 8-class Dissolve | **Shipped** — `dissolve` in Softmax `classes`; holdout regen; ordinary mix can select Dissolve |
| **P37** | Mix & Listen autoplay | **Shipped** — HTML5 player + `convertFileSrc`; Autoplay after mix; Set order (AutoDJ) |
| **P38** | FFT spectral centroid | **Shipped** — hop FFT magnitude centroid replaces ZCR proxy; Softmax dim 6 honesty |
| **P39** | Folder Automix one-shot | **Shipped** — Tauri `automix_folder`; scan wav/flac → Softmax solve → autoplay |
| **P40** | Mix Now + cue timeline | **Shipped** — `join_timeline_secs` + cue `mix_timeline_*`; Mix Now seek; now-playing highlight |
| **P41** | Key-route harmonic discount | **Shipped** — `key_pair_cost` + `HARMONIC_ROUTE_DISCOUNT` via `suggest_next_keys` in `pair_cost_edge` |
| **P42** | Automix Matching UX | **Shipped** — `rank_next_matches` + CLI `suggest`/`remix`; Tauri Next matches + Queue & remix |
| **P43** | Set-order Camelot + BPM | **Shipped** — cue `key`; transition key/ΔBPM/route/cost; Set order + Transitions UI |
| **P44** | Join timeline markers | **Shipped** — player join ticks from `join_timeline_secs`; Transition row click→seek |
| **P45** | Set-order prep-quality pills | **Shipped** — per-track trusted/weak from `prep_tracks[]` on Set order |
| **P46** | Mix Now keyboard shortcut | **Shipped** — `N` / `Shift+N` next/prev join seek (djay listen parity) |
| **P47** | Gate→hint listening rubric | **Shipped** — gate pill tooltips + Strict QA fail callout + Listen while playing checklist |
| **P48** | MixReport override-flag parity | **Shipped** — cue/transition/phrase flags in gate strip; `bars_per_phrase` on Set order |
| **P49** | Tauri LUFS target control | **Shipped** — Streaming −14 / Broadcast −23 presets; Strict QA locks −14 |
| **P50** | Join-aware mix timeline | **Shipped** — cue segments + active/next join chrome + class listen hints |
| **P51** | Tauri Automix Editor | **Shipped** — mix-in/out + `transition_out` draft → `mix_editor_remix` preserve-order |
| **P52** | Broadcast −23 CI golden | **Shipped** — `--golden broadcast` + CI; `BROADCAST_TARGET_LUFS` asserts |
| **P53** | FLAC decode parity | **Shipped** — `claxon` + `read_audio`; gold `loop_a_128.flac`; CI analyze |
| **P54** | Policy-hints E2E golden | **Shipped** — `--golden policy_hints`; house 12s vs builtin 8s |
| **P55** | Rekordbox absolute cue remap | **Shipped** — Type 1/2 → `mix_in_secs`/`mix_out_secs`; `--golden rekordbox_cue_overrides` |
| **P56** | Automix Editor baseline seed + CI | **Shipped** — seed mix-in/out + `transition_out` from MixReport; clippy allow on suggest/remix |
| **P57** | MixReport sidecar export | **Shipped** — `{out}.mixreport.json` + optional `--report-out`; CI smoke sidecar assert |
| **P58** | Tauri policy-hints discoverability | **Shipped** — Browse… + Load demo → `demo_policy_hints.json` |
| **P59** | Automix Editor `bars_per_phrase` | **Shipped** — seed + 4/8/16 select → temp playlist pin |
| Next | Export Automix Editor playlist JSON | Save dialog → reusable `--playlist` JSON |
| Later | Stems / live dual-deck; neural phrase ML | gated / user-owned audio |

## Re-rank bias

Prefer agent-doable product code that closes remaining mix-quality gaps before brochure fluff. P0–P59 are in place; stems remain gated.
