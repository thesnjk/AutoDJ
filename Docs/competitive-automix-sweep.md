# Competitive automix sweep

Living gap matrix. Update after Firecrawl research; every Gap needs an evidence path.

| Surface | Competitor cue | AutoDJ status | Evidence |
|---------|----------------|---------------|----------|
| Offline automix render | djay Automix | Parity-path MVP | `autodj-cli` mix + `render.rs` |
| Harmonic suggestions | Rekordbox Camelot | Parity MVP | `camelot.rs`, solver tests |
| Stems-aware transitions | Serato Stems | Gap | no stem DSP yet |
| Phrase sync | Rekordbox phrase | Gap | section marks are coarse |
| Learning from setcraft | Mixonset / research tools | Gap→stub | `autodj_train` synthetic only |
| Live / controller | VirtualDJ / Serato | Gap | Tauri stub only |
| Loudness / broadcast | Pacemaker-style radio | Partial | peak limiter only; R128 TBD |

## Re-rank bias

Prefer agent-doable product code that closes **mix_impact** gaps (phrase cueing, EQ swap DSP, R128) before UI chrome.
