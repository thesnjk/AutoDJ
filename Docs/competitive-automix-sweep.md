# Competitive automix sweep

Living gap matrix. Updated 2026-11-11 after **P74** competitive ungated closure (post-P73 niche flag parity + macOS Tauri compile). Firecrawl notes stay in local `.firecrawl/` (not committed). Every remaining **Gap** needs an evidence path; ungated product surfaces below are **Shipped** / **Parity (closed)**.

## Surface matrix

| Surface | Competitor cue | AutoDJ status | Evidence |
|---------|----------------|---------------|----------|
| Offline automix render | djay Automix (playlist → continuous blend; Mix Now / cue points) | **Shipped** (Parity closed) | CLI `mix` + MixReport sidecar/Reveal; Tauri Folder Automix / Mix & Listen / Mix Now — P7, P37–P40, P57, P61, P73 |
| Library I/O (wav/flac) | Serato / Rekordbox lossless libraries | **Shipped** (Parity closed) | `read_audio` / `write_audio` (claxon + flacenc); CI FLAC analyze; Tauri Browse — P53, P67 |
| Feature extraction (BPM/key/sections) | Rekordbox / djay prep analysis | **Shipped** (Parity closed) | `AnalysisQuality`, spectral centroid, prep gates, Analyze Browse — P8, P23, P38, P64 |
| Harmonic suggestions | Rekordbox / DJ.Studio Camelot ordering | **Shipped** (Parity closed) | `camelot.rs`; `key_pair_cost` / key-route discount; Set-order Camelot — P41, P43 |
| Energy-arc setcraft | Club open→peak→cool programming | **Shipped** (Parity closed) | `ArcPhase` + solver costs; MixReport order — P4 |
| Transition type palette | djay Filter / Echo / Tremolo / Riser / Dissolve / Neural Mix; SetFlow six-core | **Shipped** (Parity closed); Neural/stems Later | Softmax 8-class + EQ/FX goldens; `transition_out` — P2, P15, P28–P36; Neural Mix Later |
| Mix-in / mix-out cueing | VirtualDJ Smart automix; djay start/end points | **Shipped** (Parity closed) | `plan_segment_cues` + `CueOverrides`; cue goldens; Editor — P1, P21, P33, P51, P55 |
| Library prep (Rekordbox XML) | Rekordbox playlist export → DJ.Studio | **Shipped** (Parity closed) | `rekordbox.rs`; preserve-order goldens; Editor Save playlist — P10–P13, P55, P60 |
| Stems-aware transitions | Serato Stems (vocal / melody / bass / drums pads) | **Gap** (Later / gated) | no stem DSP; needs user-owned stem path — v2 |
| Phrase sync | Rekordbox phrase analysis / djay BPM+Beats sync | **Shipped** (Parity closed) | phrase bars + LCM + Softmax v3 phrase dims; Editor 4/8/16 — P24–P27, P59 |
| Learning from setcraft | Mixonset / tracklist research tools | **Shipped** (Parity closed); live scrapes gated | SoftmaxLogReg v3 + offline policy_hints; Tauri Browse — P5–P6, P54, P58, P63, P66 |
| Live / controller | VirtualDJ / Serato dual-deck; djay Automix play / Mix Now | **Shipped** desktop shell; dual-deck Later | Folder Automix + Mix & Listen + Mix Now + Editor — P7, P37–P51, P68; live dual-deck Later |
| Loudness / broadcast | EBU R 128 (−23 LUFS); radio continuous sets | **Shipped** (Parity closed) | R128 + `--lufs` + broadcast golden; Tauri LUFS / peak-limit — P0, P49, P52, P73 |
| Tempo blend | djay Sync + Tempo Blend | **Shipped** (Parity closed) | bidirectional WSOLA + stretch gates — P3, P17, P32 |
| Spectral continuity eval | Mix-quality rubric adjacent flux | **Shipped** (Parity closed) | `eval.rs` hop-RMS; join continuity — P4, P12 |

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
- Ubuntu WebKit / full `tauri build`: **deferred** — CI runs `cargo check --manifest-path apps/autodj-app/src-tauri/Cargo.toml` on `macos-latest` only (P73); `npm run tauri dev` remains Mac-local.

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
| **P60** | Export Automix Editor playlist JSON | **Shipped** — Save playlist… → CLI-compatible JSON |
| **P61** | Tauri MixReport sidecar reveal | **Shipped** — log + Reveal MixReport… (`revealItemInDir`) |
| **P62** | Load editor playlist into Advanced | **Shipped** — Save playlist… fills `#playlist` + preserve-order |
| **P63** | Advanced Softmax model Browse | **Shipped** — Browse… + Load committed → `transition_model.json` |
| **P64** | Analyze input Browse | **Shipped** — Browse folder… + Browse file… (wav/flac) |
| **P65** | Feature DB `#db` Browse | **Shipped** — Browse… (open) + New… (save) for `.sqlite` |
| **P66** | Tauri `--no-transition-model` | **Shipped** — Advanced Skip Softmax checkbox → CLI flag |
| **P67** | FLAC export parity | **Shipped** — `write_audio` / `flacenc`; `--out` `.wav`/`.flac` |
| **P68** | UI reliability + Docs completeness | **Shipped** — busy guards, honest autoplay, folder infer for matches, Doc accuracy |
| **P69** | Living project writeup | **Shipped** — `Docs/project-writeup.md` + README refresh; append-on-ship hygiene |
| **P70** | NEA-style professional writeup | **Shipped** — Analysis→Evaluation structure; persona + developer-led eval |
| **P72** | Deepen NEA-style writeup | **Shipped** — mermaid diagrams; algorithm depth; CI/unit evidence tables; Appendix E placeholders |
| **P73** | CLI↔Tauri niche flags + macOS Tauri compile | **Shipped** — `--report-out` / `--no-loudness-norm` in Advanced; CI `cargo check` src-tauri on macos-latest |
| **P74** | Competitive ungated closure & matrix honesty | **Shipped** — surface matrix re-labeled Shipped/Parity (closed); stems Gap only; writeup/README closure |
| **P76** | Thesis-grade writeup + data contracts | **Shipped** — REQ register; `data-contracts.md`; pair_cost model; Softmax metrics; failure/ops matrices |
| **P78** | UI busy-lock honesty + offline polish | **Shipped** — report-panel/timeline busy lock; `aria-busy`; no Google Fonts CDN |
| **P80** | Multi-root Radio libraries + Feature DB index | **Shipped** — Tauri roots list (localStorage); `analyze_libraries`; `automix_folder` unions roots; [`full-auto-roadmap.md`](full-auto-roadmap.md) |
| **P81** | Continuous radio mode | **Shipped** — Start/Stop radio; Softmax top-1 → `remix_queue_next` → seek handoff; multi-root Matching via symlink stage |
| **P82** | Transition timing depth | **Shipped** — upcoming-join −20s prerender; `remix --horizon 3`; hold-then-swap at handoff (±0.15s) |
| Later | Stems / live dual-deck; neural phrase ML | gated / user-owned audio |
| Next | **P83** set-length + vibe | full-auto ladder |

## Re-rank bias

Competitive ungated matrix closed after P0–P78. **Full-auto North Star** continues: ship **P83** per [`full-auto-roadmap.md`](full-auto-roadmap.md)—**never** idle while those phases remain, **never** pick-lists, **never** start stems without a user-owned stem path. Stems / live dual-deck / neural phrase ML / streaming OAuth / open-web scrape remain Later/gated.
