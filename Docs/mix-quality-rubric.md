# Mix quality rubric

Automated gates (CI) plus listening checklist for v1+.

## Automated (must pass)

| Check | Pass criterion |
|-------|----------------|
| Golden smoke CLI | `autodj mix --golden smoke` exits 0 and writes WAV |
| Peak limit | True-peak proxy ≤ 0.99 full scale |
| Beat-phase (v1) | Mean downbeat offset across transitions < 50 ms on gold loops |
| Spectral continuity (v1) | Adjacent-window flux ratio within genre policy bounds |
| Solver unit tests | Harmonic neighbor preferred over distant key when BPM similar |

## Listening checklist (human)

- [ ] Transitions land on phrase boundaries (8/16 bars) when structure is clear
- [ ] Bass conflict avoided during EQ-swap style blends
- [ ] Energy arc matches genre (house long blend vs DnB snap)
- [ ] No obvious clicks/pops at join points
- [ ] Loudness consistent across the set (R128 target ±2 LU)

Fail any automated row → block merge. Listening fails → open follow-up issue, do not paper over with doc-only checkboxes.
