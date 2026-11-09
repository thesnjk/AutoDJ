# Mix quality rubric

Automated gates (CI) plus listening checklist for v1+.

## Automated (must pass)

| Check | Pass criterion |
|-------|----------------|
| Golden smoke CLI | `autodj mix --golden smoke` exits 0 and writes WAV |
| Peak limit | True-peak proxy ≤ 0.99 full scale |
| Integrated loudness (R128 / BS.1770) | After normalize, `integrated_lufs` within ±2 LU of `--lufs` (golden default −14 LUFS; use `--lufs -23` for classic EBU R 128). Evidence: `loudness.rs`, MixReport `loudness_gate_pass` |
| Phrase / section cues | MixReport `cue_phrase_aligned: true`; cue windows from `plan_segment_cues` (evidence: `cue.rs`, unit snap tests). Crossfade durations snapped to whole beats. |
| Beat-phase (v1) | Mean downbeat offset across transitions < 50 ms on gold loops — **partial**: unit coverage via `align_crossfade_to_downbeat` / `snap_to_phrase`; multi-BPM join metric TBD |
| Spectral continuity (v1) | Adjacent-window flux ratio within genre policy bounds |
| Solver unit tests | Harmonic neighbor preferred over distant key when BPM similar |

## Listening checklist (human)

- [ ] Transitions feel phrase-aware on real tracks (automated planner covers gold; listen for awkward early cuts)
- [ ] Bass conflict avoided during EQ-swap style blends
- [ ] Energy arc matches genre (house long blend vs DnB snap)
- [ ] No obvious clicks/pops at join points
- [ ] Subjective loudness even across the set (automated gate covers ±2 LU; listen for pumping / over-limiting)

Fail any automated row → block merge. Listening fails → open follow-up issue, do not paper over with doc-only checkboxes.
