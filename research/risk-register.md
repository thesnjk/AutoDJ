# Risk register — AutoDJ

**Owner:** Founder (Jacob Hackett) unless noted. Probabilities are judgement (**Working assumption**), not measured rates.

| Risk | Prob. | Impact | Early warning | Mitigation | Stop / pivot trigger |
|------|-------|--------|---------------|------------|----------------------|
| No willingness to pay | H | H | Interviews praise but refuse fee/waiver | Concierge pilot with explicit fee field; scorecard gate | <3 paid/waiver pilots after discovery window |
| Existing products solve enough | H | H | “I just use djay/VirtualDJ Automix” | Document specific job failures; MixReport QA differentiation | Alternatives adequate for ICP after 10 interviews |
| Too few reachable customers | M | H | Empty recruitment tracker after sustained outreach | Named-account list; referrals; geography UK-first | Cannot book ≥6 discoveries in founder time budget |
| Expensive acquisition | M | M | Only cold outreach converts with high effort | Founder-led sales; no paid ads until message validated | CAC (cash) exceeds early contribution |
| Confidentiality / library trust blocks adoption | M | H | Refusal to point AutoDJ at libraries | Local-first; on-device; legal policy; no upload path | ICP requires cloud library they will not grant |
| AI / Softmax output errors | M | M | Bad transitions in pilot listens | MixReport Strict QA; human Stop; honest synthetic-train label | Systematic poor listens vs baseline tools |
| Bespoke-work trap | M | H | Every pilot needs custom engineering | Fixed pilot scope; MoSCoW; exclusions | >50% delivery time is one-off custom |
| Excessive onboarding / support | M | H | Hours teaching library roots / Feature DB | Thin getting-started; concierge script | Support hours > pilot fee contribution |
| Low workflow frequency | M | M | DJs mix rarely | Target event operators with weekly use | Usage <1 meaningful session / month |
| Seasonal demand | L | M | Only festival season interest | Timed-set use cases year-round | Revenue cliffs with no off-season plan |
| Vendor dependency | L | M | Critical paid API for core path | Local-first; no required cloud AI | Core path blocked without paid vendor |
| Founder sales bottleneck | H | H | Pipeline stalls when coding | Time-box sales vs build; agent does engineering | Zero discoveries while building brochure features |
| Customer churn | M | H | Pilots do not continue | Measure retention gate before GO | <2 continuations |
| Regulatory / safety misinterpretation | L | H | Claims about “compliance” or PRS | No compliance claims; user owns music rights | Any claim implying license clearance |
| Scope creep (stems/streaming/scrape) | H | M | Backlog invents gated epics | Gates in prompt + charter exclusions | Building gated surfaces without assets |

## Pre-mortem

**The business failed after 12 months because…**  
We kept shipping technically interesting mix features on a public MIT repo while never confirming that UK DJs would pay for continuous radio or a concierge pilot; support for library edge-cases consumed founder time; Softmax transitions were “good enough” next to free Automix in incumbents; and we mistook GitHub activity for customer evidence.
