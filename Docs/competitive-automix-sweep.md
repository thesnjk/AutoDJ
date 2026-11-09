# Competitive automix sweep

Living gap matrix. Updated 2026-11-09 after Firecrawl research (raw notes in local `.firecrawl/`, not committed). Every Gap needs an evidence path.

## Surface matrix

| Surface | Competitor cue | AutoDJ status | Evidence |
|---------|----------------|---------------|----------|
| Offline automix render | djay Automix (playlist → continuous blend; Mix Now / cue points) | Parity-path MVP | `autodj-cli` `mix`; `crates/autodj-core/src/render.rs` |
| Harmonic suggestions | Rekordbox / DJ.Studio Camelot ordering | Parity MVP | `camelot.rs`; `autodj-graph` `prefers_harmonic_neighbor` |
| Transition type palette | djay Filter / Echo / Tremolo / Riser / Neural Mix; SetFlow six-core (EQ, bass swap, cut, blend) | Partial | `TransitionClass` enum in `analysis.rs`; renderer mostly equal-power crossfade |
| Mix-in / mix-out cueing | VirtualDJ Smart automix (outro→intro ~4–8s); djay start/end points | Gap | `SectionMarks` coarse; CLI does not yet align xfades to phrase |
| Stems-aware transitions | Serato Stems (vocal / melody / bass / drums pads) | Gap | no stem DSP; deferred v2 |
| Phrase sync | Rekordbox phrase analysis | Gap | section marks only; no 8/16-bar phrase grid |
| Learning from setcraft | Mixonset / tracklist research tools | Gap→stub | `python/autodj_train` synthetic + `tracklist_stats.py` |
| Live / controller | VirtualDJ / Serato dual-deck | Gap | `apps/autodj-app` README stub only |
| Loudness / broadcast | EBU R 128 (−23 LUFS integrated; true-peak limits); radio-style continuous sets | Parity-path (measure+norm shipped) | `loudness.rs` + CLI `--lufs`; golden gate ±2 LU (`Docs/mix-quality-rubric.md`) |
| Tempo blend | djay Sync + Tempo Blend | Partial | solver tempo cost; no time-stretch blend in renderer |

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

## Persona re-rank (mix_impact × gap)

Formula note: `(competitor best − AutoDJ) × persona weight × mix_impact`. Bedroom DJ / content creator and radio/podcast continuous mix weighted highest this session.

| Priority | Gap | Why |
|----------|-----|-----|
| **P0** | EBU R128 normalize + CI loudness gate | **Shipped** — keep regression via golden `loudness_gate_pass` |
| **P1** | Phrase / section-aware cue points | VDJ Smart + djay cue points; largest remaining “feels like a DJ” jump |
| **P2** | 3-band EQ / bass-swap transition classes | SetFlow + house/techno policy; needs section cues to land well |
| **P3** | Tempo blend / stretch | djay Sync+Tempo Blend; higher DSP risk |
| Later | Stems, live controller, setcraft learning | v2 / gated |

## Re-rank bias

Prefer agent-doable product code that closes **mix_impact** gaps (phrase cueing, EQ swap DSP) before UI chrome. R128 loudness gate is in place.
