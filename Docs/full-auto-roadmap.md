# Full-auto private DJ — roadmap

**North Star:** Launch AutoDJ and leave it alone. The app sources music from **user-owned local library roots**, chooses the next track, decides **transition class + exact cue timing**, renders or hands off gaplessly, and keeps going until **Stop**. Minimal UI: Stop, library roots, Strict QA / vibe knobs.

This is **not** “pick one folder → one-shot mix.” That flow remains as a power-user path; continuous radio is the product default once P81 ships.

## Locked defaults

| Decision | Choice |
|----------|--------|
| Source (v1) | Multi-root **local** libraries (Music / Downloads / …). User points once; app indexes into Feature DB. |
| Streaming | Personal OAuth only when the user later supplies credentials. |
| Open-web / YouTube scrape | **Out of OSS mainline** (legal gate). Private forks are out of scope for this Doc. |
| Runtime | **Continuous radio** (play until Stop) + optional timed set length (P83). |
| Engine | Keep Rust mix core, Softmax, cues, MixReport, Tauri shell — evolve into an always-on loop. |

## Phase ladder

| Phase | Goal | Status |
|-------|------|--------|
| **P80** | Library roots + Feature DB index; Automix across roots | **Shipped** |
| **P81** | Continuous radio mode: now-playing → Softmax/`rank_next_matches` → cue-timed transition → queue/render next; Stop ends loop | **Shipped** |
| **P82** | Transition timing depth: stronger live join scheduling from MixReport timelines; less whole-set-only render when radio is on | **Shipped** |
| **P83** | Optional set-length + energy/vibe targets on top of radio | Next |
| **Later / gated** | Stems (need path); streaming OAuth (need secrets); open-web sourcing (out of OSS) | Gated |

## Non-goals

- Asking the user which phase to ship next (agents follow this ladder).
- Shipping stems, paid scrapes, or YouTube ingest in the MIT tree.
- Rewriting the mix engine from scratch.

## Evidence

Update this table’s Status column and [`competitive-automix-sweep.md`](competitive-automix-sweep.md) when a phase PR merges. Narrative appends go in [`project-writeup.md`](project-writeup.md).
