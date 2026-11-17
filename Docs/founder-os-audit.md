# Founder OS audit — AutoDJ

**Date:** 2026-11-17  
**Evidence labels:** Verified fact (repo state) · Working assumption (founder defaults) · Proposed experiment (validation backlog)

## A. Findings

| Area | Status | Label |
|------|--------|-------|
| Agent prompt | Local-only `Docs/autoDJ-master-prompt.md` (`.git/info/exclude`) | Verified fact |
| Product thesis | Offline lawful mix engine; North Star continuous radio from local roots | Verified fact |
| Shipped engineering | P0–P86 on `main`; North Star P80–P83 complete | Verified fact |
| Charter / backlog / scorecard | Absent before this upgrade | Verified fact |
| Commercial validation | No interviews, pilots, or paid commitments in repo | Verified fact |
| License | MIT; copyright previously “AutoDJ contributors” | Verified fact |
| Strategic posture after upgrade | NARROW — private paid product intent; public MIT code; no production SaaS spend | Working assumption |

## B. Preserve

- Product thesis and North Star locked defaults (local libraries; no YouTube/open-web scrape in mainline)
- Shipment ledger P0–P86; known gates and failed approaches
- Git cadence: issue → branch → PR → substantive review → print merge commands
- Legal audio rules; synthetic/aggregate training only; no secrets in git
- Docs hygiene, MixReport/CI goldens, Softmax eval gate
- Local-only master prompt; no tracked `AGENTS.md` / `.cursor/`
- Autonomy rules: no pick-lists; skip gated assets silently

## C. Gaps

- No project charter, ordered commercial backlog, validation scorecard, or risk register
- “Next=idle” after North Star left agents with no evidence-producing commercial track
- Copyright line did not name sole owner
- No discovery/pilot templates adapted to DJ buyers
- Paid-product intent undocumented while Docs emphasised open research packaging only

## D. Contradictions → corrections

| Conflict | Reliable source | Correction |
|----------|-----------------|------------|
| Idle vs never-idle | Founder OS upgrade | `product/backlog.md` is Next; append evidence rows if empty |
| MIT research framing vs paid private intent | Founder direction + preserve thesis | Charter posture NARROW; MIT code retained; commercial packaging founder-led |
| Copyright “contributors” vs sole owner | Founder | LICENSE → Jacob Hackett; NOTICE clarifies trademarks/support |
| Generic B2B SaaS OS vs local-first app | Architecture + legal policy | Local-first / Tauri+CLI; concierge pilot before cloud SaaS |

## E. Upgrade structure (implemented)

Committed: charter, backlog, portfolio, scorecard, risk register, research templates, LICENSE/NOTICE, developer-map and legal cross-links, writeup append.  
Local: upgraded master prompt (A–Q adapted; never committed).
