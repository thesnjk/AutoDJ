# Competitive pricing observations

**Observed:** 2026-11-17 (UTC) via Firecrawl live scrape (`maxAge: 0`) + Apple App Store GB listing.  
**Rules:** Date every row. Prefer **GBP**. State VAT treatment if disclosed. State tool/data limitations.  
**Forbid:** Invented competitor prices or market sizes.  
**Label:** Verified fact (URL + date) vs Unresolved question.

Engineering capability matrix (not pricing): [`Docs/competitive-automix-sweep.md`](../../Docs/competitive-automix-sweep.md).

| Date (UTC) | Competitor / product | Plan observed | Price | Currency | VAT note | URL / source | Limitation | Label |
|------------|----------------------|---------------|-------|----------|----------|--------------|------------|-------|
| 2026-11-17 | rekordbox | Professional (monthly) | 36 / month | GBP | Not stated on plan cards | https://rekordbox.com/en/plan/ | GB locale scrape; cloud add-ons separate; may change | Verified fact |
| 2026-11-17 | rekordbox | Professional (yearly billed; shown as monthly equiv.) | 30 / month equiv. (360 / year) | GBP | Not stated on plan cards | https://rekordbox.com/en/plan/ | Same; page also shows 432/year comparison | Verified fact |
| 2026-11-17 | rekordbox | Creative (monthly) | 26 / month | GBP | Not stated | https://rekordbox.com/en/plan/ | Cloud option tiers also listed | Verified fact |
| 2026-11-17 | rekordbox | Creative (yearly billed; monthly equiv.) | 21 / month equiv. (252 / year) | GBP | Not stated | https://rekordbox.com/en/plan/ | Same | Verified fact |
| 2026-11-17 | rekordbox | Core (monthly) | 21 / month | GBP | Not stated | https://rekordbox.com/en/plan/ | Same | Verified fact |
| 2026-11-17 | rekordbox | Core (yearly billed; monthly equiv.) | 17 / month equiv. (204 / year) | GBP | Not stated | https://rekordbox.com/en/plan/ | Same | Verified fact |
| 2026-11-17 | VirtualDJ | Home (monthly) | 4 / month | GBP | Not stated | https://virtualdj.com/buy/ | GB buy page; entry-level controllers only; branding remains | Verified fact |
| 2026-11-17 | VirtualDJ | VDJ PRO (monthly) | 17 / month | GBP | Not stated | https://virtualdj.com/buy/ | Annual also shown £181 / year | Verified fact |
| 2026-11-17 | VirtualDJ | Business (monthly) | 90 / month | GBP | Not stated | https://virtualdj.com/buy/ | Annual £910 / year; Infinity lifetime offered but AI features limited — amount not extracted on this pass | Verified fact (subs) / Unresolved question (Infinity £ amount) |
| 2026-11-17 | DJ.Studio | Studio (one-time) | 99 | GBP | Page: prices incl. taxes (British Pounds) | https://dj.studio/pricing?currency=GBP | Promo countdown on page; amounts can change | Verified fact |
| 2026-11-17 | DJ.Studio | Pro (one-time) | 129 | GBP | Incl. taxes (GBP) | https://dj.studio/pricing?currency=GBP | Same promo caveat | Verified fact |
| 2026-11-17 | DJ.Studio | Pro + Stems (one-time, promo) | 149 (was 277) | GBP | Incl. taxes (GBP) | https://dj.studio/pricing?currency=GBP | Limited-time discount banner; verify before quoting | Verified fact |
| 2026-11-17 | DJ.Studio | Ultimate (one-time, promo) | 189 (was 524) | GBP | Incl. taxes (GBP) | https://dj.studio/pricing?currency=GBP | Same | Verified fact |
| 2026-11-17 | DJ.Studio | Pro + Stems (monthly sub) | 29 / month | GBP | Incl. taxes (GBP); no refund for monthly | https://dj.studio/pricing?currency=GBP | Sub vs lifetime differ | Verified fact |
| 2026-11-17 | djay (Algoriddim) | Full Access to PRO (IAP) | 6.99 | GBP | App Store tax handling | https://apps.apple.com/gb/app/djay-dj-app-ai-mixer/id450527929 | Platform/store IAP; period appears monthly among listed IAPs | Verified fact |
| 2026-11-17 | djay (Algoriddim) | Full Access to PRO (IAP) | 48.99 | GBP | App Store tax handling | https://apps.apple.com/gb/app/djay-dj-app-ai-mixer/id450527929 | Appears annual among listed IAPs; also saw £26.49 tier — period ambiguous without App Store period label in scrape | Verified fact (amount) / Unresolved question (exact billing period for £26.49) |
| 2026-11-17 | djay desktop (algoriddim.com/store) | — | — | GBP | — | https://www.algoriddim.com/store/djay | Redirected to US App Store listing in one scrape; no stable GBP desktop storefront extracted | Unresolved question |

## Tool / data limitations

- Firecrawl live scrape (`maxAge: 0`), location GB / Europe/London timezone on successful pages.
- Prices geo-vary; promotional banners (esp. DJ.Studio) expire.
- Do not treat these as AutoDJ pricing recommendations or market-size evidence.
- Re-scrape before any external commercial claim.
