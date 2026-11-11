# AutoDJ — project writeup

Living narrative of what AutoDJ is, what shipped, and how to verify it. Prefer **appending** short evidence notes when a new P-row lands; do not invent unshipped features. Deep specs stay in the linked Docs below.

**Matrix / status:** [`competitive-automix-sweep.md`](competitive-automix-sweep.md) (P0–P69 as of this seed).  
**How to run:** [`getting-started.md`](getting-started.md).  
**Layout / tests:** [`developer-map.md`](developer-map.md).  
**Pipeline detail:** [`architecture.md`](architecture.md).

---

## Thesis

AutoDJ is a research-grade automatic mix engine and learning pipeline. It produces genre-aware, harmonically coherent offline DJ mixes from playlists and library folders. Setcraft and transition policy come from lawful, attributable signals (tracklists, charts, public analyses, licensed metadata, user-owned libraries)—never a pirate training corpus. See [`legal-data-policy.md`](legal-data-policy.md).

Primary runtime: **macOS Apple Silicon**. Offline DSP is pure Rust (no Linux-only audio backends). License: MIT.

---

## What it is today

End-to-end product surface on `main`:

1. **Ingest** — WAV/FLAC libraries (`read_audio` / `claxon`) or JSON / Rekordbox XML playlists; optional Feature DB (`analyze --db`).
2. **Solve** — Softmax v3 8-class transition model (default) or genre `preferred_transition` (`--no-transition-model`); Camelot + tempo + energy-arc costs; optional preserve-order / Automix Matching (`suggest` / `remix`).
3. **Cue & render** — Phrase-aware cues, class-aware EQ/FX crossfades (EqSwap, Filter, Echo, Tremolo, Riser, Dissolve, HardCut), bidirectional WSOLA tempo blend, EBU R128 loudness, MixReport gates.
4. **Export** — WAV or FLAC (`write_audio` / `flacenc`) + `{out}.mixreport.json` sidecar.
5. **Desktop** — Tauri Folder Automix → Mix & Listen → Next matches / Queue & remix → Automix Editor → Advanced playlist/analyze/Softmax/Feature DB controls.

```
playlist / folder → analyze (optional) → autodj-graph solve → autodj-core render → WAV/FLAC + MixReport
                                      ↘ Softmax / policy hints / cue overrides
```

---

## Capability ledger

Grouped by surface. Evidence is code/tests/Docs—not marketing. Full P-row table: [`competitive-automix-sweep.md`](competitive-automix-sweep.md).

### Library I/O

- Decode WAV + FLAC via `read_audio` (`claxon`); gold `loop_a_128.flac` + CI analyze (**P53**).
- Encode WAV + FLAC via `write_audio` (`hound` / `flacenc`); CLI `--out` extension dispatch (**P67**).
- SQLite Feature DB for analyze/mix; Tauri Browse… / New… on main panel (**P65**).

### Analysis & cues

- BPM/key/sections with `AnalysisQuality`, octave fold, FFT spectral centroid (**P8**, **P38**).
- Beat grid + beat-phase / peak MixReport gates (**P9**).
- Phrase/section cues, drop-aware planning, per-track `bars_per_phrase`, LCM join snap (**P1**, **P21**, **P24**, **P26**).
- Absolute mix windows: playlist `mix_in_secs`/`mix_out_secs`; Rekordbox Type 1/2 remap (**P33**, **P55**).
- Prep quality gate + Tauri trusted/weak pills (**P23**, **P45**).

### Solver & setcraft

- Greedy NN + energy-arc phases + genre policy (`genre-policy.md`) (**P4**).
- Key-route harmonic discount via `suggest_next_keys` (**P41**).
- Softmax default per-edge classes; `--no-transition-model` opt-out; six-genre goldens (**P6**, **P16**, **P20**).
- Softmax v2/v3 features (stretch, spectral, phrase bars); 8-class Dissolve (**P25**, **P27**, **P36**).
- Policy hints from offline tracklist aggregates (**P5**, **P54**).
- Automix Matching: `rank_next_matches` / `build_remix_order`; CLI `suggest`/`remix`; Tauri Next matches + Queue & remix (**P42**).
- Playlist `transition_out` per-edge FX pins (**P34**).

### FX / DSP / loudness

- EqSwap + FilterSweep + EchoOut; bass-overlap gate (**P2**, **P14**).
- Tremolo / Riser / Dissolve FX + Softmax class expansions (**P28–P31**, **P35–P36**).
- Class-aware crossfade durations (**P22**).
- Bidirectional WSOLA tempo blend + stretch-clamp costs/gates (**P3**, **P17**, **P32**).
- Spectral + join continuity eval (**P4**, **P12**).
- EBU R128 normalize; Streaming −14 / Broadcast −23; `--golden broadcast` (**P0**, **P49**, **P52**).

### CLI & CI goldens

- `analyze` / `mix` / `suggest` / `remix`; MixReport stdout + sidecar (**P57**).
- Goldens: `smoke`, `broadcast`, `rekordbox`, `rekordbox_cue_overrides`, `cue_overrides`, `transition_overrides`, `transitions`, `transition_model`, `policy_hints` (**P13**, **P15**, **P16**, **P33**, **P34**, **P52**, **P54**, **P55**).
- Rubric automated table: [`mix-quality-rubric.md`](mix-quality-rubric.md).

### Tauri desktop shell

- Folder Automix one-shot (**P39**); Mix & Listen autoplay (**P37**).
- Mix Now + cue/join timeline + keyboard `N` / `Shift+N` (**P40**, **P44**, **P46**, **P50**).
- Set-order Camelot/BPM + prep pills; gate→rubric listening UX (**P43**, **P45**, **P47**, **P48**).
- Automix Editor: seed mix-in/out / transition / phrase → Apply & re-mix / Save playlist → Advanced (**P51**, **P56**, **P59–P62**).
- Discoverability: policy-hints Browse/Load demo; Softmax Browse/Load committed; Skip Softmax; Analyze Browse; Feature DB Browse (**P58**, **P63–P66**).
- Reveal MixReport sidecar (**P61**).
- Busy-state + honest autoplay + folder infer for matches after Mix playlist (**P68**).

### Train pipeline

- `python/autodj_train`: synthetic pairs, SoftmaxLogReg (numpy CI), `autodj-train eval` ≥ 0.90 holdout (**P6**).
- Committed artifact `python/autodj_train/models/artifacts/transition_model.json` (v3, 8 classes).

---

## How to verify

Do not duplicate command walls here.

- First mix / Tauri / flags: [`getting-started.md`](getting-started.md).
- Workspace tests + golden cmds: [`developer-map.md`](developer-map.md).
- Pass/fail gates: [`mix-quality-rubric.md`](mix-quality-rubric.md).

Smoke:

```bash
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav
# sidecar → /tmp/autodj-smoke.wav.mixreport.json
```

---

## Roadmap / gates

**Later / gated** (need user-owned stems or explicit product ungate—not agent-invented):

- Stems-aware transitions (Serato-style vocal/melody/bass/drums).
- Live dual-deck / controller.
- Neural phrase ML.
- Paid chart / live scrape clients (ToS / anti-bot)—offline JSON/CSV only.

**Deferred infra:** Tauri CI on ubuntu WebKit (Mac-local verify).

---

## How this Doc grows

1. When a new **P-row** ships, append a short bullet under the matching Capability ledger subsection (or a dated **Shipped notes** subsection below) with one evidence path.
2. Update [`competitive-automix-sweep.md`](competitive-automix-sweep.md) in the **same** PR; keep this writeup aligned—never claim Gap when symbols exist on `main`.
3. Keep README Status to 2–4 honest sentences pointing here.
4. Never invent features, never commit copyrighted audio, never weaken legal gates.

### Shipped notes

- **2026-11-11 — P69:** Seeded this writeup through **P68** capabilities; added living-ledger hygiene; refreshed README from “MVP scaffold” to P0–P69 public front door.
