# Genre policy defaults

Mirrored in `GenrePolicy::builtin` (`autodj-graph`). Adjust weights in code + keep this Doc honest.

| Genre id | BPM center | Max ΔBPM | Default xfade | Preferred transition |
|----------|------------|----------|---------------|----------------------|
| `house` (default) | 124 | 6 | 8s | `eq_swap` |
| `drum_and_bass` | 174 | 6 | 0.25s | `hard_cut` |
| `hip_hop` | 90 | 8 | 1s | `echo_out` |
| `uk_garage` | 132 | 5 | 8s | `eq_swap` |
| `trance` | 138 | 4 | 16s | `filter_sweep` |
| `open_format` | 120 | 20 | 4s | `linear_crossfade` |

Aliases: `dnb` → drum_and_bass; `garage` → uk_garage; `rnb` → hip_hop.

Forbidden styles (enforced as soft penalties later): long 16s blends in DnB; hard cuts mid-phrase in trance intros.
