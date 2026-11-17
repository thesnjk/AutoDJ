# Founder OS audit — AutoDJ

**Date:** 2026-11-17 (reconcile on `main` after #170)  
**Evidence labels:** Verified fact (repo state) · Working assumption (founder defaults) · Proposed experiment (validation backlog)

## A. Findings

| Area | Status | Label |
|------|--------|-------|
| Agent prompt | Local-only `Docs/autoDJ-master-prompt.md` (`.git/info/exclude`) | Verified fact |
| Tracked `Docs/founder-master-prompt.md` (#170) | **Deleted** in this upgrade — generic venture prompt with RouteFinder calibration; commit had `Co-authored-by: Cursor Agent` | Verified fact |
| Product thesis | Offline lawful mix engine; North Star continuous radio from local roots | Verified fact |
| Shipped engineering | P0–P86 on `main`; North Star P80–P83 complete | Verified fact |
| Commercial validation | No interviews, pilots, or paid commitments in repo | Verified fact |
| License / NOTICE | MIT; sole copyright **Jacob Hackett**; trademarks/support not granted beyond MIT | Verified fact (this PR) |
| Strategic posture | **NARROW** — private paid product intent; public MIT code; no production SaaS spend | Working assumption |

## B. Preserve

- Product thesis and North Star locked defaults (local libraries; no YouTube/open-web scrape in mainline)
- Shipment ledger P0–P86; known gates and failed approaches
- Git cadence: issue → branch → PR → substantive review → print merge commands
- Legal audio rules; synthetic/aggregate training only; no secrets in git
- Docs hygiene, MixReport/CI goldens, Softmax eval gate
- Local-only master prompt; no tracked `AGENTS.md` / `.cursor/`
- Autonomy rules: no pick-lists; skip gated assets silently
- Useful #170 stubs retained as pointers (`commercial/`, `finance/`, `strategy/`, `operations/`, top-level `research/*`)

## C. Gaps (addressed or backlog)

- Idle Next after North Star → replaced by `product/backlog.md`
- Dual founder OS (#170 generic prompt vs AutoDJ OS) → single OS; tracked founder prompt removed
- Copyright “contributors” → Jacob Hackett + NOTICE
- Discovery/pilot templates → under `research/templates/`
- Local master prompt A–Q upgrade → local-only (not committed)

## D. Contradictions → corrections

| Conflict | Reliable source | Correction |
|----------|-----------------|------------|
| Idle vs never-idle | Founder OS upgrade | `product/backlog.md` is Next; append evidence rows if empty |
| MIT research framing vs paid private intent | Founder direction + preserve thesis | Charter posture NARROW; MIT code retained; commercial packaging founder-led |
| Copyright “contributors” vs sole owner | Founder | LICENSE → Jacob Hackett; NOTICE clarifies trademarks/support |
| Generic B2B SaaS OS / RouteFinder calibration vs AutoDJ | Architecture + legal policy | Delete `founder-master-prompt.md`; local-first Tauri+CLI; AutoDJ charter |
| #170 paths vs `research/templates/` | This reconcile | Pointers + canonical templates; no silent loss of pricing hypotheses |
| Cursor Agent co-author on #170 | Founder scrub | History rewrite: drop `da839da` (#170); keep post-#173 tree as one sole-author commit on top of `3cfbbdc`; founder runs `git push --force-with-lease origin main`. Ban future `Co-authored-by: Cursor Agent` |

## E. Upgrade structure (clean tip)

Committed (this tip, parent `3cfbbdc`): charter, backlog, portfolio, scorecard, risk register, research templates (+ CRM / 24m finance templates), LICENSE/NOTICE, developer-map and legal cross-links, writeup append; no `Docs/founder-master-prompt.md`; #170-era stubs as pointers.  
Local: upgraded master prompt (A–Q adapted; never committed).  
**History:** `#170` / `da839da` (Cursor Agent co-author) is intentionally absent from this tip’s ancestry after founder force-with-lease.

## F–N checklist

| Artefact | Path |
|----------|------|
| Charter | `Docs/project-charter.md` |
| Developer map | `Docs/developer-map.md` |
| Backlog | `product/backlog.md` |
| Scorecard | `research/validation-scorecard.md` |
| Risk register | `research/risk-register.md` |
| Portfolio | `research/opportunity-portfolio.md` |
| Templates | `research/templates/*` |
| Ownership | `LICENSE`, `NOTICE` |
