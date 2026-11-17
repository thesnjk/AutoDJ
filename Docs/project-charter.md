# Project charter — AutoDJ

**Source of truth for current direction.** Operating rules live in the local master prompt (`Docs/autoDJ-master-prompt.md`, local-only).  
**Currency:** GBP. Fees/prices are **ex-VAT** unless stated.  
**Evidence:** label every commercial claim. Do not invent interviews, pilots, market sizes, or customer commitments.

| Field | Value | Label |
|-------|-------|-------|
| Project name | AutoDJ | Verified fact |
| One-line description | Offline automatic mix engine and continuous private radio from user-owned local libraries, with Softmax transition policy, DSP render, and MixReport QA | Verified fact |
| Current strategic posture | **NARROW** — private paid product intent; public MIT engineering surface; **no production SaaS spend** until validation thresholds met | Working assumption |
| Primary customer | UK mobile/club DJs and small event operators who already own local music libraries | Working assumption |
| Anti-persona | Enterprise radio chains with IT procurement; residencies needing live dual-deck; users who expect cloud library sync or stems out of the box | Working assumption |
| Primary problem | Building and running coherent continuous or timed sets without babysitting every transition | Working assumption |
| Primary workflow | Add library roots → index → Start radio (or timed set + vibe) → leave running → Stop; optional MixReport QA | Verified fact (shipped UX) |
| Promised outcome | Launch-and-leave mixes that feel like skilled setcraft (harmonic/tempo/energy-aware), not random crossfades | Working assumption (listening quality not certified) |
| Human approval | User selects libraries/genre/export; listens before publishing; founder approves commercial terms | Working assumption |
| Revenue model | Paid private product / paid concierge pilot first; packaging and support founder-led. MIT code remains public | Working assumption |
| Pilot price hypothesis | GBP 150–300 ex VAT for a 2-week fixed-scope pilot (1 library, capped tracks, deliverable + MixReport review) | Working assumption |
| Initial geography | United Kingdom (England-first outreach practicality); product not geo-locked technically | Working assumption |
| Active opportunity | Continuous private radio + timed sets for library-owning DJs (see `research/opportunity-portfolio.md`) | Working assumption |
| Backup opportunity | Offline automix + MixReport QA for bedroom producers / podcast editors | Working assumption |
| Explicit exclusions | YouTube/open-web music scrape marketplace; stem SaaS without user-owned stems; streaming OAuth without founder secrets; unpaid brochure “AI DJ” features; production multi-tenant cloud before validation | Verified fact (gates) + Working assumption |
| Founder constraints | Solo founder; agent-assisted engineering; must not commit secrets/customer audio; founder owns sales and legal review | Working assumption |
| GO / NARROW / PIVOT / STOP | **NARROW** | Working assumption |
| Next decision gate | Discovery evidence: recurring problem + alternative inadequacy + willingness to pay for a defined pilot (see validation scorecard). Thresholds are **founder decision thresholds**, not validated industry benchmarks | Proposed experiment |

## Product thesis (preserved)

Learn setcraft and transition policy from lawful, attributable signals. Render mixes from **user-owned** libraries. Never build a pirate training corpus. North Star: multi-root local libraries + continuous radio + optional timed set/vibe (P80–P83 shipped).

## Ownership

Copyright: **Jacob Hackett (2026)**. Public repository under MIT — see `LICENSE` and `NOTICE`. Trademarks, product name, and unpaid commercial support are not granted beyond MIT. Commercial packaging and paid pilots are founder-led and private for now.
