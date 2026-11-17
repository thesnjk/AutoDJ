# Product backlog — AutoDJ

Ordered, self-service. On bare `continue` / master-prompt attach: take the **first unfinished** row, implement on a feature branch, open PR, mark done **in the same PR**. If empty after completion, append 3–5 agent-doable evidence-producing rows (never filler, never gated stems/streaming/scrape without assets).

**Row must:** produce evidence, reduce risk, improve conversion/retention/maintainability, or advance a validated workflow.  
**Forbid:** generic Docs-only fluff, premature SaaS scaffolding, speculative features, unvalidated integrations, renumber-only PRs, tracked agent master prompts.

| ID | Status | Action | Why | Evidence produced |
|----|--------|--------|-----|-------------------|
| B1 | done | Reconcile founder OS on main: delete `Docs/founder-master-prompt.md`, sole-owner LICENSE/NOTICE, charter/backlog/scorecard/risk/templates, thin #170 stubs to pointers | One validation-first OS; never-idle Next | `Docs/project-charter.md`, `product/backlog.md`, `research/*`, `NOTICE` |
| B2 | done | Sync `research/templates/assumption-register.md` from charter/portfolio (labelled; no invented interviews/Actuals) | Make assumptions greppable | `research/templates/assumption-register.md` A1–A20 |
| B3 | done | Refine DJ buyer interview guide + recruitment tracker for UK mobile/club ICP (templates only; no invented accounts) | Enable founder discovery | `research/templates/interview-guide.md`, `recruitment-tracker.md` |
| B4 | done | Competitive pricing observation sheet: dated GBP public-page rows only (or explicit blanks + tool limitations) | Price honesty | `research/templates/competitive-pricing-observations.md` (2026-11-17) |
| B5 | done | Complete paid concierge pilot brief fields agents can fill without fees; leave fee/waiver for founder | Pilot readiness | `research/templates/pilot-brief.md` |
| B6 | done | Keep CRM named-account tracker skeleton honest (blank Actuals); link scorecard acquisition row | Founder-led sales prep | `research/templates/crm-named-accounts.md` |
| B7 | done | Refine observation checklist for radio/timed-set shadowing; link scorecard workflow-evidence row | Workflow evidence capture | `research/templates/observation-checklist.md` |
| B8 | done | Align interview-synthesis + negative-findings-log with O1 stages/labels (blank Actuals) | Discovery synthesis hygiene | `research/templates/interview-synthesis.md`, `negative-findings-log.md` |
| B9 | done | Index `customer-discovery-programme.md` to all live templates (CRM, pilot-brief, assumptions) | Single discovery index | `research/customer-discovery-programme.md` |
| B10 | done | Risk-register cross-links to scorecard gates + pilot-brief fee field (no invented residual scores) | Risk ↔ validation wiring | `research/risk-register.md` |
| B11 | done | Thin root stubs `research/interview-guide.md`, `assumption-register.md`, `negative-findings.md` to pointers at `templates/` | Link hygiene | `research/*.md` stubs |
| B12 | done | Align `finance/pricing.md` hypothesis labels with pilot-brief (not an offered price) | Price honesty | `finance/pricing.md` |
| B13 | done | Cross-link `Docs/getting-started.md` to charter NARROW + discovery programme (commercial readers) | Docs discovery path | `Docs/getting-started.md` |
| B14 | done | Scorecard Retention / Support effort Evidence paths to pilot-brief + risk-register (Actuals blank) | Scorecard completeness | `research/validation-scorecard.md` |
| B15 | done | Scorecard Recurring problem → interview-synthesis; Repeat usage → pilot-brief (Actuals blank) | Scorecard completeness | `research/validation-scorecard.md` |
| B16 | done | Align `finance/financial-model.md` with `templates/financial-model-24m.md` gate (no invented numbers) | Finance gate honesty | `finance/financial-model.md` |
| B17 | done | Point Existing-alternative inadequacy Evidence path at competitive-pricing-observations + competitor-matrix | Scorecard completeness | `research/validation-scorecard.md` |
| B18 | done | Add `Docs/developer-map.md` one-liner to charter + `product/backlog.md` Next | Agent orientation | `Docs/developer-map.md` |
| B19 | done | Scorecard Contribution margin Evidence path → `finance/financial-model.md`; `finance/pricing.md` (Actuals blank) | Scorecard completeness | `research/validation-scorecard.md` |
| B20 | done | Add discovery programme to developer-map Docs pointers | Agent orientation | `Docs/developer-map.md` |
| B21 | done | Cross-link `Docs/founder-os-audit.md` status date to current backlog Next (no invented gaps) | Audit freshness | `Docs/founder-os-audit.md` |
| B22 | done | Point scorecard Recommendation rationale at discovery programme index (keep NARROW) | Scorecard completeness | `research/validation-scorecard.md` |
| B23 | todo | Getting-started Desktop: short Stop / leave-alone authority + local-libraries-only callout | O1 workflow clarity | `Docs/getting-started.md` |
| B24 | todo | Link MixReport gates in getting-started to `Docs/mix-quality-rubric.md` | Docs honesty | `Docs/getting-started.md` |
| B25 | todo | Point `apps/autodj-app/README.md` at charter NARROW + getting-started radio flow | App orientation | `apps/autodj-app/README.md` |
| B26 | todo | Add Softmax eval verify one-liner to `Docs/developer-map.md` Tests section | DX / verify | `Docs/developer-map.md` |

## Completed engineering ledger (do not renumber)

Historical P0–P86 remain in `Docs/project-writeup.md` and the local master prompt shipment index. North Star P80–P83 complete. Do not re-open as pick-lists.
