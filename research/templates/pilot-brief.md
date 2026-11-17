# Paid concierge pilot brief — AutoDJ (O1)

**Status:** Agent-completable template. **Fee / waiver blank until founder sets them.**  
**Principle:** Prefer paid concierge before building new software or cloud SaaS.  
**Labels:** Defaults below are **Working assumption** from charter / `finance/pricing.md` unless marked Verified fact.  
**Cross-links:** Assumptions A8/A9 ([`assumption-register.md`](assumption-register.md)); scorecard paid-pilot row ([`../validation-scorecard.md`](../validation-scorecard.md)); stages ([`recruitment-tracker.md`](recruitment-tracker.md), [`crm-named-accounts.md`](crm-named-accounts.md)).

## Fixed scope

- Customer provides **user-owned** local library roots (paths they control).
- Founder (with optional agent assist) configures AutoDJ **Radio libraries**, Feature DB index, continuous radio and/or timed set (30 / 45 / 60 / 90 min) + vibe (Standard / Chill / Peak).
- Deliver: running radio/set workflow + **MixReport** QA review of at least one rendered or live-handoff session.
- **Out of scope:** stems, streaming OAuth, open-web sourcing, custom DSP features, multi-tenant cloud, legal music licensing / PRS clearance.

## Responsibilities

| Party | Responsibility |
|-------|----------------|
| Customer | Lawful ownership/licence of audio; baseline estimates; Stop authority; listen feedback before any public use |
| Founder | Setup; scope control; data-handling boundary; fee or documented waiver; continuation decision |
| Agent (optional) | Template hygiene, MixReport tooling, Docs — **never** invents fee, customer, or pilot Actuals |

## Limits

| Field | Value | Label |
|-------|-------|-------|
| Volume limit | **1** customer library cohort; up to **3** indexed roots (founder may raise/lower) into **one** Feature DB; **one** continuous radio **or** **one** timed set + vibe for the window | Working assumption |
| Duration | **14 days** (2 weeks); founder may override in writing before start | Working assumption (charter) |
| Fee (GBP ex-VAT) | **___** (founder fills) **or** documented waiver | Unresolved until founder acts |
| Pricing hypothesis (not an offer) | Charter / `finance/pricing.md` band **GBP 150–300 ex VAT** for this 2-week scope — **do not quote as offered price** until fee field filled | Working assumption |
| VAT treatment | Ex-VAT unless founder states otherwise | Working assumption |
| Baseline measurement | See form below (blank Actuals) | — |
| Success criteria | See checklist below; agree in writing before start | Working assumption |
| Data-handling | On-device/local only; see Data-handling section | Verified fact (product posture) |
| Continuation decision point | End of duration → **Customer / Lost** on recruitment tracker / CRM | Working assumption |

## Baseline measurement (blank Actuals)

| Field | Actual | Label when filled |
|-------|--------|-------------------|
| Minutes to prepare a continuous/timed set today | ___ | Verified fact (customer-stated) |
| Tools used today | ___ | Verified fact |
| Leave-alone need (Y/N + context) | ___ | Verified fact / Reasoned inference |
| Pain last time (key/energy/silence/vibe) | ___ | Verified fact |

## Success criteria (checklist template)

Agree in writing before start; founder sets **N** where needed.

- [ ] Continuous radio or timed set runs without babysitting for ≥ **N** minutes (default N=30)
- [ ] ≥1 MixReport reviewed with customer (gates + listen notes)
- [ ] Customer listen feedback logged (synthesis / negative-findings as applicable)
- [ ] No library upload; no customer audio committed to git
- [ ] Scope stayed inside Out-of-scope list (no stems/streaming/scrape/custom DSP)

## Data-handling

- Honor [`Docs/legal-data-policy.md`](../../Docs/legal-data-policy.md) and [`NOTICE`](../../NOTICE).
- Feature DB and caches stay on paths the **customer** controls; AutoDJ does not grant music rights.
- **Never** commit customer audio, unredacted playlists, or PII-heavy exports.
- No multi-tenant cloud or upload path in v1 product.
- Softmax/AI may assist transitions; **only the customer** approves consequential public performance use of their library.

## Measure (during / after pilot)

- Time saved (baseline vs pilot)
- Errors prevented or caught (MixReport / listen notes)
- Rework avoided
- Human review effort
- Support effort (founder hours)
- Repeat usage
- Willingness to continue paying (not enthusiasm alone)

## Continuation

At end of duration: update [`recruitment-tracker.md`](recruitment-tracker.md) / [`crm-named-accounts.md`](crm-named-accounts.md) to **Customer** or **Lost**; refresh scorecard paid-pilot / retention rows only from real evidence.

## Prohibit

Unsupported claims about sales uplift, safety, compliance, incidents, or ROI.  
Do not invent fee, waiver, customer name, or pilot results.
