# Project Charter

Canonical direction for AutoDJ as a venture. **Verified fact** items cite repo docs; **Working assumption** items are unproven and need validation.

## Project identity

- **Working name:** AutoDJ
- **One-line description:** Offline automatic DJ mix engine: analyse a library or playlist, solve a genre-aware mix graph, render a beat-aligned mix to WAV/FLAC with QA (MixReport), plus a macOS desktop shell for listen / edit / rematch. (*Verified fact — `README.md`.)
- **Product category:** Offline DJ automix / library prep tooling (not live club deck replacement).
- **Initial geography:** **Working assumption:** UK-first founder-led sales; product is not geo-locked technically (macOS primary).
- **Current strategic posture:** **NARROW** — technical v1 is shipped; commercial validation and paid pilots are not yet established.

## Customer

- **Primary buyer:** Bedroom DJ / content creator who owns a music library and needs finished mixes for practice, streams, or promo without hours of manual cueing. (*Reasoned inference from persona table in `Docs/project-writeup.md` §1.3.*)
- **Primary user:** Same as buyer (solo operator).
- **Anti-persona:** Enterprise radio chains with IT procurement; club residencies needing live dual-deck; users who expect cloud library sync or stem isolation out of the box (*explicit product exclusions / Later-gated features*).
- **Initial customer segment:** macOS-based creators with local WAV/FLAC libraries (not streaming-catalog-only DJs).

## Problem

- **Recurring problem:** Producing a coherent, genre-aware DJ mix from an owned library requires harmonic knowledge, cue placement, transition craft, and loudness work—repeated whenever they publish or practise. (*Verified fact — problem statement in `Docs/project-writeup.md` §1.1.*)
- **Frequency:** **Working assumption:** weekly to monthly per active creator (not daily for most).
- **Current workaround:** Manual prep in Rekordbox/Serato/djay; partial automix in closed apps; or giving up on full mixes.
- **Cost of the problem:** **Working assumption:** 2–6 hours per mix for skilled manual work; higher for beginners (**Unresolved question** until interviews confirm).
- **Evidence status:** **No customer discovery interviews logged** (*Verified fact — writeup states no fabricated interviews*).

## Proposed product

- **Core workflow:** Folder or playlist → analyse features → solve mix order → render offline mix → listen / QA (MixReport) → optional edit / rematch in Tauri shell.
- **Promised outcome:** A listenable, beat-aligned mix from the user’s lawful library in one automated pass, with repeatable QA gates—not a guarantee of “club ready” taste.
- **Human approval points:** User selects library/playlist, genre policy, export path, and any cue overrides; user listens before publishing.
- **Explicit exclusions (until validation GO):** Multi-tenant SaaS; cloud library upload; streaming OAuth; stems neural mix; live dual-deck; new P-row feature expansion beyond validation/pilot support.
- **Data sensitivity:** User music libraries stay local; no committed commercial audio in git (*Verified fact — `Docs/legal-data-policy.md`*).

## Business model

- **Revenue model:** **Working assumption:** paid concierge pilot (founder-delivered mix QA + onboarding) leading to one-time desktop license or low-touch subscription only after 2+ paying retainers (see §6 founder thresholds).
- **Pilot price hypothesis:** GBP 150–300 ex VAT for a 2-week fixed-scope pilot (1 library, up to N tracks, one deliverable mix + MixReport review). **Working assumption.**
- **Subscription price hypothesis:** GBP 8–15/month ex VAT desktop updates — **Working assumption; not validated.**
- **Primary acquisition channel:** Founder-led outreach to named accounts (DJs, small radio/podcast operators, content creators) — not paid ads before message validation.

## Founder constraints

- **Available hours:** **Working assumption:** part-time venture alongside RouteFinder maintenance (RouteFinder is capacity benchmark only, not expanded here).
- **Technical strengths:** Native macOS, offline Rust DSP, mapping/route-scale engineering discipline (*RouteFinder benchmark in founder master prompt*).
- **Commercial constraints:** Solo founder sales; no fabricated social proof.
- **Cash constraints:** Bootstrap; no grant funding assumed.
- **Non-negotiables:** Evidence honesty (§5 founder prompt); no copyrighted audio in repo; no production SaaS before commercial validation GO.

## Current opportunity portfolio

| Rank | Opportunity | Status | Why |
|------|-------------|--------|-----|
| 1 | AutoDJ commercial validation (sell workflow + desktop) | Active | Shipped technical core; MIT; differentiated offline QA story |
| 2 | *(unset)* | Hold | Do not develop until active opportunity stops or pivots |
| — | Greenfield 12–15 opportunity hunt | Excluded | Explicit scope for separate session unless founder pivots repo purpose |

## Active opportunity

- **Name:** AutoDJ commercial validation
- **Why selected:** Substantial shipped engine (P0–P86) reduces build risk; problem aligns with solo-maintainable offline architecture.
- **What must be validated:** Recurring problem severity; inadequacy of substitutes for this job; willingness to pay for pilot; retention after pilot; support time economics.
- **Stop conditions:** Fewer than 3/10 interviews show same recurring problem; zero paid pilot interest after 20 qualified outreaches; support cost exceeds pilot revenue with no path to automate.

## Backup opportunity

- **Name:** TBD
- **Rule:** No parallel product development until active opportunity is GO, STOP, or explicitly parked with founder approval.

## Decision posture

- **GO / NARROW / PIVOT / STOP:** **NARROW**
- **Reason:** Technical delivery meets v1 objectives (*Verified fact — writeup evaluation*); commercial evidence thresholds in `research/validation-scorecard.md` are not met.
- **Next decision gate:** At least 3 businesses agree to pay for a defined pilot (founder decision threshold, not industry benchmark).