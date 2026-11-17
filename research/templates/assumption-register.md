# Assumption register

**Last synced:** 2026-11-17 (UTC)  
**Sources:** [`Docs/project-charter.md`](../../Docs/project-charter.md), [`research/opportunity-portfolio.md`](../opportunity-portfolio.md), [`finance/pricing.md`](../../finance/pricing.md)  
**Rules:** Labels required. **No invented interviews, quotations, WTP Actuals, or pilot results.** Update after every discovery block; log refusals in [`negative-findings-log.md`](negative-findings-log.md).

| ID | Assumption | Label | Status | Charter / portfolio link | Evidence / next test | Owner |
|----|------------|-------|--------|--------------------------|----------------------|-------|
| A1 | Primary buyers are UK mobile/club DJs and small event operators who already own local music libraries | Working assumption | Open | Charter: Primary customer | Interview guide + recruitment tracker | Founder |
| A2 | Primary problem is building/running coherent continuous or timed sets without babysitting every transition | Working assumption | Open | Charter: Primary problem | Discovery interviews; observation checklist | Founder |
| A3 | Launch-and-leave continuous radio / timed set + vibe is a valued job-to-be-done (O1) | Working assumption | Open | Charter: Active opportunity; Portfolio O1 | Discovery interviews; observation of set prep | Founder |
| A4 | Shipped UX (library roots → index → Start radio / timed set → Stop; MixReport QA) is the right primary workflow to validate | Verified fact (capability) + Working assumption (buyer fit) | Open (buyer fit) | Charter: Primary workflow | Capability: P80–P83 / app README. Buyer fit: interviews | Founder |
| A5 | Promised outcome (mixes that feel like skilled setcraft) is believable enough to trial — listening quality not certified | Working assumption | Open | Charter: Promised outcome | Pilot listen sessions + MixReport | Founder |
| A6 | Incumbent Automix is inadequate for continuous/timed sets with QA for this ICP | Working assumption | Open | Scorecard: alternative inadequacy | Interview alternative questions; dated competitive pricing (B4) | Founder |
| A7 | Local-first (no library upload) is required for trust | Working assumption | Open | Charter: exclusions + legal policy | Interview confidentiality questions | Founder |
| A8 | Buyers will pay for a defined concierge pilot (GBP ex-VAT) before any production SaaS | Working assumption | Open | Charter: Revenue model; Portfolio O3 | Pilot offer; fee or documented waiver | Founder |
| A9 | Concierge pilot GBP 150–300 ex VAT (2-week fixed scope: 1 library, capped tracks, deliverable + MixReport review) is a testable band | Working assumption | Open | Charter: Pilot price hypothesis; `finance/pricing.md` | Price test in interviews; leave blank if founder unprepared | Founder |
| A10 | Backup ICP (bedroom producers / podcast editors — O2) stays secondary until O1 fails scorecard | Working assumption | Open | Charter: Backup; Portfolio O2 | Scorecard STOP/PIVOT on O1 | Founder |
| A11 | MIT public code does not block paid private packaging/support (trademarks/support not granted beyond MIT) | Working assumption | Open | Charter: Ownership; `NOTICE` | Founder commercial terms (legal review) | Founder |
| A12 | Softmax synthetic training is acceptable if pilot listening quality holds | Working assumption | Open | Legal/data policy; train path | Pilot listens + MixReport Strict QA | Founder / agent (QA tooling only) |
| A13 | Strategic posture NARROW is correct: no production SaaS spend until validation scorecard GO | Working assumption | Open | Charter: Posture; Next decision gate | Scorecard thresholds (founder decision, not industry benchmarks) | Founder |
| A14 | Human approval is required: user selects libraries/genre/export and listens before publishing; founder owns commercial/legal terms | Working assumption | Open | Charter: Human approval | Pilot brief responsibilities; interview approval questions | Founder |
| A15 | Anti-personas (enterprise radio chains, live dual-deck residencies, cloud-sync/stems-out-of-box buyers) are correctly excluded for now | Working assumption | Open | Charter: Anti-persona / exclusions | Negative-findings log if they appear as only reachable leads | Founder |
| A16 | Explicit exclusions hold: no YouTube/open-web scrape marketplace; no stem SaaS without user-owned stems; no streaming OAuth without founder secrets; no multi-tenant cloud before validation | Verified fact (engineering gates) + Working assumption (commercial) | Open (commercial) | Charter: Explicit exclusions; Portfolio O8/O9/O15 | Do not build gated surfaces without assets; log pressure to violate in negative findings | Founder |
| A17 | Support burden stays solo-survivable post-onboarding | Working assumption | Open | Charter: Founder constraints; risk register | Pilot time log | Founder |
| A18 | macOS-primary is acceptable for first paying users | Working assumption | Open | Charter geography / platform | Lost deals in negative-findings log | Founder |
| A19 | UK-first (England-first outreach) does not block early adopters; product not geo-locked technically | Working assumption | Open | Charter: Initial geography | Recruitment mix | Founder |
| A20 | Next decision gate is discovery evidence (recurring problem + alternative inadequacy + WTP for defined pilot) before GO | Proposed experiment | Open | Charter: Next decision gate; validation scorecard | Fill scorecard Actuals only from real discovery | Founder |

## Sync checklist

- [x] Rows map to current charter fields (2026-11-17)
- [x] O1 primary / O2 backup reflected
- [x] No interview Actuals invented
- [ ] After first discovery block: update Status / Evidence columns; tick refusals into negative-findings log
