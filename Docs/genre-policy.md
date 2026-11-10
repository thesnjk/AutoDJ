# Genre policy defaults

Mirrored in `GenrePolicy::builtin` (`autodj-graph`). Adjust weights in code + keep this Doc honest.

Optional overrides from tracklist aggregates: `autodj mix --policy-hints path/to/demo_policy_hints.json` merges `bpm_center` and `default_crossfade_secs` via `GenrePolicy::with_hints` (energy-arc weight and preferred transition stay built-in).

Solver edge duration uses `crossfade_secs_for_class`: `HardCut` → 0s; `EchoOut` → short tail (`min(default, max(1s, 2 beats))`); `EqSwap` / `FilterSweep` / `LinearCrossfade` keep the genre default. Cue planning still caps/snaps for short fixtures.

| Genre id | BPM center | Max ΔBPM | Default xfade | Preferred transition | Energy-arc weight |
|----------|------------|----------|---------------|----------------------|-------------------|
| `house` (default) | 124 | 6 | 8s | `eq_swap` | 1.0 |
| `drum_and_bass` | 174 | 6 | 0.25s | `hard_cut` | 1.0 |
| `hip_hop` | 90 | 8 | 1s | `echo_out` | 1.0 |
| `uk_garage` | 132 | 5 | 8s | `eq_swap` | 1.0 |
| `trance` | 138 | 4 | 16s | `filter_sweep` | 1.25 |
| `open_format` | 120 | 20 | 4s | `tremolo` | 0.5 |

Aliases: `dnb` → drum_and_bass; `garage` → uk_garage; `rnb` → hip_hop.

## Energy arc

Solver uses `ArcPhase` (Opening → Build → Peak → Cooldown by set position). Free start picks the quietest in-band opener. Soft style penalty: DnB + crossfade > 1s → `+5` cost.

Forbidden styles (soft penalties): long 16s blends in DnB; hard cuts mid-phrase in trance intros (preference already biases away via `preferred_transition`).
