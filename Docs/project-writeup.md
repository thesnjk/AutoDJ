# AutoDJ — Professional Project Writeup

**Status:** P0–P70 on `main` (as of this revision).  
**Living document:** expand / append; never invent unshipped features.  
**Companion specs:** [`competitive-automix-sweep.md`](competitive-automix-sweep.md) · [`architecture.md`](architecture.md) · [`getting-started.md`](getting-started.md) · [`mix-quality-rubric.md`](mix-quality-rubric.md) · [`legal-data-policy.md`](legal-data-policy.md) · [`developer-map.md`](developer-map.md) · [`genre-policy.md`](genre-policy.md)

This writeup follows a professional project structure (Analysis → Design → Implementation → Testing → Evaluation → User Guide → Appendices). There are **no clients and no fabricated interviews**; Analysis uses product personas, and Evaluation is **developer-led** against measurable objectives and automated evidence.

---

## Table of contents

1. [Analysis](#1-analysis)  
2. [Design](#2-design)  
3. [Implementation](#3-implementation)  
4. [Testing](#4-testing)  
5. [Evaluation](#5-evaluation)  
6. [User guide](#6-user-guide)  
7. [Bibliography](#7-bibliography)  
8. [Appendices](#8-appendices)

---

## 1. Analysis

### 1.1 Problem statement

DJs and content creators need **continuous, genre-aware mixes** from their own libraries: tracks ordered for harmonic and energy flow, transitions that feel intentional (EQ swaps, filters, cuts, phrase-aware blends), and loudness suitable for streaming or broadcast. Doing this by hand requires cue points, Camelot knowledge, and careful gain riding.

**AutoDJ** addresses that problem as an **offline automatic mix engine**: ingest a folder or playlist, analyse features, solve a mix order, render a beat-aligned mix to WAV/FLAC, and expose QA (MixReport) plus a desktop shell for listen / edit / rematch—without uploading user libraries or training on pirated commercial sets.

### 1.2 Background

Skilled DJs combine:

- **Harmonic mixing** (Camelot / relative keys) so adjacent tracks share compatible tonality.  
- **Tempo management** (BPM matching, short stretch windows) to avoid audible warble.  
- **Phrase and section awareness** (intro / drop / outro) so mixes land on musical boundaries.  
- **Transition vocabulary** (EQ bass swap, filter sweeps, echo outs, hard cuts, longer blends).  
- **Programme loudness** (EBU R 128 / BS.1770 family) for consistent playback level.

Commercial tools (djay Automix, VirtualDJ Automix, Serato Stems, Rekordbox prep) productise parts of this. AutoDJ targets a **research-grade, offline, open (MIT)** pipeline with **deterministic goldens** and a **lawful learning path** (synthetic transitions + offline tracklist aggregates)—see competitive citations in §7 and [`competitive-automix-sweep.md`](competitive-automix-sweep.md).

### 1.3 Stakeholders / personas

| Persona | Needs | Weight in re-rank |
|---------|--------|-------------------|
| Bedroom DJ / content creator | One-shot folder → mix → listen; Matching; Editor pins | Highest |
| Radio / podcast continuous mix | Stable loudness (−14 / −23 LUFS); long coherent sets | High |
| Club open-format | Wide BPM tolerance; diverse transition classes | Medium |
| Label promo pack | Preserve playlist order; Rekordbox XML fidelity | Medium |

Personas come from standing competitive re-rank guidance (not client interviews).

### 1.4 Research into the problem

**Competitive surface matrix** (summary): offline automix render, feature extraction, harmonic suggestions, energy-arc setcraft, transition FX palette, cueing, Rekordbox prep, phrase sync, learning from setcraft, loudness, tempo blend, spectral continuity. Stems / live dual-deck / neural phrase ML remain **Later / gated**.

**Legal research:** [`legal-data-policy.md`](legal-data-policy.md) forbids commercial set scrapes and unlicensed stems in git/CI; tracklist ingest commits **statistics + citations only**; live 1001Tracklists / Beatport clients are gated.

**Technical research outcomes encoded in product:** SoftmaxLogReg v3 (15-dim, 8 classes), MixReport hard gates under Strict QA, Tauri thin shell over CLI.

### 1.5 Alternative solutions

| Approach | Strengths | Gaps vs AutoDJ goals |
|----------|-----------|----------------------|
| djay / VirtualDJ Automix | Polished UX, Mix Now, FX palette | Closed; limited offline golden/CI story |
| Serato Stems | Stem-aware FX | Stem DSP gated for AutoDJ without user-owned stems |
| Rekordbox + DJ.Studio | Prep + Camelot messaging | Not a full offline render/QA pipeline under MIT |
| Manual Ableton / DJ software | Full control | High skill/time cost |
| **AutoDJ** | Offline MIT core; goldens; Softmax; Tauri; lawful data | No stems/live yet; WebKit CI deferred |

### 1.6 Objectives

#### 1.6.1 Functional objectives (FO)

| ID | Objective | Measurable success |
|----|-----------|-------------------|
| FO1 | Analyse library audio (WAV/FLAC) for BPM/key/sections | Gold analyze ±1 BPM; FLAC CI analyze |
| FO2 | Solve mix order (harmonic + tempo + energy arc) | Graph unit tests; Folder Automix |
| FO3 | Softmax per-edge transition classes (8-class) | `--golden transition_model`; train eval ≥ 0.90 |
| FO4 | Genre preferred transitions without Softmax | `--no-transition-model`; `--golden transitions` |
| FO5 | Phrase/section-aware cues + overrides | `cue_phrase_aligned`; cue/Rekordbox override goldens |
| FO6 | Class-aware FX crossfades + tempo blend | EQ/Dissolve unit tests; smoke `tempo_blend_applied` |
| FO7 | R128 loudness (−14 / −23) + peak limit | smoke/broadcast goldens; MixReport gates |
| FO8 | MixReport QA sidecar + stdout JSON | `{out}.mixreport.json`; CI smoke assert |
| FO9 | Rekordbox XML ingest + preserve-order | `--golden rekordbox` / `rekordbox_cue_overrides` |
| FO10 | Automix Matching (suggest / remix) | Graph tests; CLI + Tauri Next matches |
| FO11 | Tauri Folder Automix → listen → Editor | Desktop flows; Mix Now / `N` |
| FO12 | Policy hints from offline aggregates | `--golden policy_hints` |
| FO13 | Export WAV or FLAC | `write_audio`; `--out .flac` |

#### 1.6.2 Non-functional objectives (NFO)

| ID | Objective | Measurable success |
|----|-----------|-------------------|
| NFO1 | Deterministic CI goldens | All listed goldens exit 0 on CI |
| NFO2 | Offline / on-device library privacy | No upload path; legal policy |
| NFO3 | Modular maintainability | Cargo crates + Python train split |
| NFO4 | Mac-first desktop usability | Tauri busy guards; path Browse; a11y basics (P68) |
| NFO5 | Lawful data only | No forbidden audio in git; gated scrapes |
| NFO6 | Real-time-safe DSP discipline | Offline render path; no RT alloc on hot docs rules |

#### 1.6.3 Objective prioritisation

Shipped P0–P69 closed the ungated mix-quality and shell gaps. Remaining matrix **Gap** is stems-aware transitions (v2 / user-owned audio). Personas weighted bedroom DJ and radio continuous mix first—hence loudness, Folder Automix, Matching, and Editor before gated live/stems.

### 1.7 Modelling the problem

#### 1.7.1 Decomposition

1. **Decode** → `AudioBuffer`  
2. **Analyse** → features + quality → optional SQLite Feature DB  
3. **Solve** → order + transition classes (Softmax or preferred)  
4. **Cue** → mix-in/out windows, phrase bars, overrides  
5. **Render** → WSOLA + EQ/FX + R128 + eval gates  
6. **Report** → MixReport JSON + audio out  
7. **Shell** → Tauri invokes CLI; listen / match / edit  

#### 1.7.2 Logical architecture

```
playlist JSON / Rekordbox XML ──► apps/autodj-app (Tauri) ──┐
        │                                                    │
        ▼                                                    ▼
 autodj-io ──► autodj-core (analyze + rekordbox) ──► SQLite feature cache
        │
        ▼
 autodj-graph (solve_order | preserve_order + TransitionModel)
        │
        ▼
 autodj-core (cues → WSOLA + EQ xfade + R128 + eval) ──► WAV/FLAC + MixReport
        │
 autodj_train: synth → Softmax JSON + policy_hints aggregates
```

#### 1.7.3 Softmax / transition model

Softmax **v3**: 15-dim vector (BPM pair, |ΔBPM|, Camelot distance, energy ΔdB, stretch clamp excess, spectral centroid Δ, phrase bars + mismatch, genre one-hots) → **8 classes** including `dissolve`. Artifact: `python/autodj_train/models/artifacts/transition_model.json`.

#### 1.7.4 MixReport as QA model

MixReport carries order, cues, transitions, loudness, spectral/join/bass/stretch/beat-phase/peak/prep gates, override flags, and timeline fields for Mix Now UI. Sidecar: `{out}.mixreport.json`.

### 1.8 Requirement derivation

Requirements flow from: persona needs × competitive gaps × legal constraints → FO/NFO → P-row backlog in the sweep → shipped symbols on `main`. Each FO maps to at least one golden, unit test, or Tauri command (see §2.6).

### 1.9 Constraints

- MIT license; no copyrighted commercial audio in git/CI.  
- Offline-first; user libraries stay local.  
- Primary UI platform: macOS Apple Silicon; Tauri WebKit CI on Ubuntu deferred.  
- Encode/decode: WAV + FLAC only (export default WAV for HTML5 autoplay).  
- Stems / live dual-deck / neural phrase ML not in v1 scope.

### 1.10 Assumptions

- Input audio is user-owned or royalty-free fixtures.  
- Gold loops are short synthetic WAVs (and one FLAC) sufficient for CI, not for human listening sign-off on real vinyl.  
- Committed Softmax artefact remains available at the documented path for ordinary mix.  
- CLI is the source of truth; Tauri may omit niche flags (`--report-out`, `--no-loudness-norm`).

---

## 2. Design

### 2.1 Design goals / rationale

| Goal | Rationale |
|------|-----------|
| Modular Rust crates + Python train | Clear ownership; train never links into CLI runtime |
| CLI as source of truth | Goldens, scripting, Tauri all share one binary surface |
| Thin Tauri shell | Faster iteration; DSP stays in core |
| Deterministic Strict QA mode | Reproducible CI; hard-fail on critical MixReport gates |
| Lawful learning only | Synthetic + aggregates; competitive excellence without piracy |
| Extensible Softmax / FX classes | Versioned artefacts; class palette grows with evidence |

**Technology:** Rust (analysis/DSP/solver/CLI), Python numpy Softmax (CI), Tauri 2 + TypeScript (desktop), SQLite feature cache, hound/claxon/flacenc I/O.

**Trade-offs:** No stems yet; HTML5 may fail FLAC autoplay (WAV preferred); greedy solver (not full TSP) for interactive latency; Softmax trained on synthetic pairs (not commercial sets).

### 2.2 System architecture

Layers:

1. **I/O** (`autodj-io`) — `read_audio` / `write_audio`.  
2. **Core** (`autodj-core`) — analysis, Rekordbox, cues, EQ/FX, WSOLA, R128, eval.  
3. **Graph** (`autodj-graph`) — costs, Softmax, matching/remix, policy hints.  
4. **CLI** (`autodj-cli`) — `analyze` / `mix` / `suggest` / `remix` + goldens.  
5. **Train** (`python/autodj_train`) — synth, train, eval, aggregates.  
6. **App** (`apps/autodj-app`) — Tauri commands wrapping CLI.

Control flow: UI → invoke → `run_autodj` argv → MixReport JSON → UI panels / player.

Error handling: CLI non-zero + stderr; Tauri surfaces plain report + progress log; Strict QA bails on hard gates.

### 2.3 Data / module design

| Concept | Role |
|---------|------|
| `AudioBuffer` | Interleaved f32 PCM, sample rate, channels |
| Playlist JSON / Rekordbox XML | Paths + optional BPM/key/cues/`transition_out`/`bars_per_phrase` |
| Feature DB (SQLite) | Cached analysis for mix/suggest |
| `TransitionModel` | Softmax weights/classes (v1–v3 loaders) |
| `GenrePolicy` | Builtin BPM centres, xfades, preferred transitions |
| MixReport | QA + timeline + order for UI |
| CueOverrides / EditorTrack | Absolute mix windows + pins for re-mix |

Ownership: decode/analyse produce features; graph consumes features → order/classes; renderer consumes buffers + plan → audio + MixReport.

### 2.4 Algorithm design

| Algorithm | Purpose | Notes |
|-----------|---------|-------|
| Feature extraction | BPM/key/sections/spectral centroid | Offline; `AnalysisQuality` |
| `solve_order` | Greedy NN + energy-arc phases | Key-route discount; stretch costs |
| Softmax `predict_transition_class_edge` | Per-edge FX class | Default on ordinary mix |
| `plan_segment_cues` | Mix-in/out windows | Phrase LCM; drop-aware; overrides win |
| WSOLA tempo blend | Multi-BPM joins | Head + tail; clamp ±15% |
| EQ/FX crossfades | Class-specific blends | EqSwap bass-overlap scored |
| R128 normalize + true-peak | Loudness / peak | Streaming −14 / Broadcast −23 |
| Eval gates | Spectral/join/bass/stretch/beat-phase | Hard-fail when deterministic |

Complexity (prose): analysis roughly linear in samples; greedy solve O(n²) pairwise costs for small n (DJ setlists); Softmax predict O(features × classes) per edge; render linear in output samples.

### 2.5 Interface design

**Main panel:** library folder, output audio, genre, preserve-order, Strict QA, LUFS, Feature DB Browse/New, Automix folder.  
**Player:** HTML5 audio, cue segments, join ticks, Mix Now, Autoplay, Next matches, Automix Editor.  
**Advanced:** playlist Browse, analyze Browse, policy hints, Softmax path / Skip Softmax.  
**Reliability (P68):** full-form busy lock; Mix Now / `N` blocked while CLI busy; honest autoplay failure; folder infer for matches after Mix playlist.

### 2.6 Objective traceability matrix

| Objective | Primary modules | Primary evidence |
|-----------|-----------------|------------------|
| FO1 | `autodj-core` analysis, `autodj-io` | gold analyze; FLAC CI |
| FO2 | `autodj-graph` | unit tests; Folder Automix |
| FO3 | Softmax + CLI | `transition_model` golden; train eval |
| FO4 | CLI `--no-transition-model` | `transitions` golden; Tauri checkbox |
| FO5 | `cue.rs`, Rekordbox | cue / rb cue goldens |
| FO6 | `eq.rs`, `stretch.rs`, render | unit + smoke |
| FO7 | `loudness.rs` | smoke + broadcast |
| FO8 | CLI MixReport | sidecar CI |
| FO9 | `rekordbox.rs` | rekordbox goldens |
| FO10 | graph matching + Tauri | tests + suggest/remix |
| FO11 | `apps/autodj-app` | Tauri flows |
| FO12 | policy hints | `policy_hints` golden |
| FO13 | `write_audio` | FLAC roundtrip test |
| NFO1–NFO6 | CI, legal Doc, crate layout, P68 UX | workflows + Docs |

---

## 3. Implementation

### 3.1 Overview / file responsibility

| Path | Responsibility |
|------|----------------|
| `crates/autodj-io/` | Decode/encode WAV/FLAC |
| `crates/autodj-core/` | Analysis, cues, DSP, eval, Rekordbox |
| `crates/autodj-graph/` | Solver, Softmax, matching, genre policy |
| `crates/autodj-cli/` | User commands + goldens |
| `python/autodj_train/` | Synth, Softmax train/eval, aggregates |
| `apps/autodj-app/` | Tauri 2 UI over CLI |
| `testdata/gold/` | Royalty-free fixtures |
| `Docs/` | Specs + this writeup |

`apps/autodj-app/src-tauri` is **excluded** from workspace members; verify with its own `Cargo.toml`.

### 3.2 Library I/O

`read_audio` / `write_audio` dispatch on extension. FLAC encode via `flacenc` (16-bit PCM → stream → file). Evidence: `crates/autodj-io/src/lib.rs`; roundtrip unit test; CLI `write_audio` in `cmd_mix`.

### 3.3 Analysis and cues

Offline BPM/key/sections with confidence and sparsity flags; FFT spectral centroid; cue planner with phrase bars and absolute overrides (JSON + Rekordbox Type 1/2). Prep quality feeds MixReport and Tauri pills.

### 3.4 Graph / Softmax / matching

`pair_cost_edge` combines Camelot, tempo, energy arc, stretch excess, Softmax/preferred class. `rank_next_matches` and `build_remix_order` power suggest/remix. Policy hints merge aggregate BPM/crossfade into builtin policy.

### 3.5 Render / loudness / eval

Renderer applies class-aware crossfades and bidirectional WSOLA; R128 normalize; eval populates MixReport gates. Strict QA (`--deterministic`) hard-fails critical gates and locks −14 LUFS.

### 3.6 CLI

Subcommands `analyze`, `mix`, `suggest`, `remix`. Ordinary mix auto-loads committed Softmax unless `--no-transition-model` or explicit `--transition-model`. Sidecar beside `--out`; optional `--report-out`.

### 3.7 Train

Synthetic pairs → SoftmaxLogReg; `autodj-train eval` gate ≥ 0.90 on holdout; committed JSON artefact consumed by Rust.

### 3.8 Tauri shell

Commands: `automix_folder`, `mix`, `analyze`, `suggest_matches`, `remix_queue_next`, `mix_editor_remix`, `export_editor_playlist`, resolve helpers. Forwards policy/Softmax/Skip Softmax/db/preserve-order/deterministic/lufs. Busy state disables controls; progress events fill the log.

### 3.9 Error handling and robustness

Invalid paths → CLI/UI errors; sparse analysis → prep gate; autoplay decode failure → listen hint (not “playing”); Skip Softmax disables Softmax path controls. CLI-only: `--report-out`, `--no-loudness-norm` (documented, no Tauri UI).

### 3.10 Objective fulfilment (implementation view)

All FO1–FO13 have landed symbols on `main` through P69; this P70 revision documents them. Later FO for stems/live are explicitly **out of scope**.

---

## 4. Testing

### 4.1 Strategy

| Layer | Mechanism |
|-------|-----------|
| Unit | `cargo test` (core/graph/io) |
| Golden E2E | `autodj mix --golden …` |
| Train gate | `autodj-train eval` |
| CI | GitHub Actions Rust (macOS + Ubuntu) + Python |
| Desktop | Mac-local `cargo check` / `tauri dev` |
| Human | Listening checklist on real libraries (not CI) |

Justification: goldens lock mix-quality regressions; units lock DSP/solver invariants; train eval locks classifier quality; Tauri remains Mac-verified while WebKit CI is deferred.

### 4.2 Mapping tests → objectives

| Test / golden | Objectives |
|---------------|------------|
| gold analyze + FLAC analyze | FO1 |
| graph unit tests | FO2, FO10 |
| `--golden transition_model` + train eval | FO3, NFO1 |
| `--golden transitions` | FO4 |
| cue / rb cue / transition_overrides | FO5, FO9 |
| EQ/Dissolve units + smoke blend | FO6 |
| smoke + broadcast | FO7, NFO1 |
| sidecar CI assert | FO8 |
| suggest/remix tests + CLI | FO10 |
| Tauri manual / P68 behaviours | FO11, NFO4 |
| `--golden policy_hints` | FO12 |
| `write_audio` FLAC roundtrip | FO13 |

### 4.3 Evidence index

Canonical commands: [`developer-map.md`](developer-map.md). Rubric pass criteria: [`mix-quality-rubric.md`](mix-quality-rubric.md). Appendix A lists smoke artefacts.

### 4.4 Residual risk

Human listening checklist rows remain unchecked by design (real tracks / vinyl). Short gold loops may skip-with-pass some phrase gates. Tauri WebKit not in CI.

---

## 5. Evaluation

### 5.1 Method (developer-led)

Without clients, evaluation uses:

1. **Automated evidence** — CI + local goldens + rubric automated table.  
2. **Persona walkthroughs** — Folder Automix → listen → Matching → Editor for bedroom DJ; Broadcast −23 for radio persona; preserve-order Rekordbox for promo pack.  
3. **Honesty rules** — Partial only where gated/deferred; never claim stems/live.

### 5.2 Functional objectives achievement

| FO | Status | Evidence |
|----|--------|----------|
| FO1 | **Pass** | Gold analyze; FLAC CI |
| FO2 | **Pass** | Graph tests; Folder Automix |
| FO3 | **Pass** | Softmax golden; eval ≥ 0.90 |
| FO4 | **Pass** | transitions golden; Tauri Skip Softmax |
| FO5 | **Pass** | cue goldens; phrase MixReport fields |
| FO6 | **Pass** | DSP units; smoke tempo blend |
| FO7 | **Pass** | smoke + broadcast goldens |
| FO8 | **Pass** | sidecar CI |
| FO9 | **Pass** | rekordbox goldens |
| FO10 | **Pass** | matching tests + UI |
| FO11 | **Pass** | Tauri shell P37–P68 |
| FO12 | **Pass** | policy_hints golden |
| FO13 | **Pass** | FLAC export P67 |

### 5.3 Non-functional objectives achievement

| NFO | Status | Evidence |
|-----|--------|----------|
| NFO1 | **Pass** | CI golden matrix |
| NFO2 | **Pass** | local-only design; legal Doc |
| NFO3 | **Pass** | crate/train split |
| NFO4 | **Pass / Mac-local** | P68 UX; WebKit CI deferred |
| NFO5 | **Pass** | legal gates enforced in Docs + process |
| NFO6 | **Pass (offline path)** | DSP in core; RT rules in standing orders |

### 5.4 Limitations and technical debt

- No stems-aware transitions; no live dual-deck; no neural phrase ML.  
- Softmax trained on synthetic data—transfer to all real genres is imperfect.  
- HTML5 FLAC autoplay uneven; default remains WAV.  
- Ubuntu Tauri CI deferred.  
- Listening checklist requires human libraries (not automated).  
- Niche CLI flags not in Tauri (`--report-out`, `--no-loudness-norm`).

### 5.5 Future improvements (gated / honest)

1. User-owned stem path → stems-aware FX.  
2. Live dual-deck product design.  
3. Neural phrase models with lawful labelled data.  
4. Optional WebKit CI when infrastructure allows.  
5. Real-library optional eval fixtures if rights-cleared packs are donated.

### 5.6 Summary

Against declared FO/NFO and the competitive ungated backlog through P69, AutoDJ **meets its v1 professional objectives**. Remaining gaps are explicitly Later/gated, not undocumented failures.

---

## 6. User guide

Full flag and Tauri detail: [`getting-started.md`](getting-started.md).

### 6.1 First CLI mix

```bash
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav
# → /tmp/autodj-smoke.wav.mixreport.json
afplay /tmp/autodj-smoke.wav   # macOS
```

Optional FLAC: `--out /tmp/autodj-smoke.flac` (in-app Autoplay still prefers WAV).

### 6.2 Desktop (macOS)

```bash
cargo build -p autodj-cli
cd apps/autodj-app && npm install && npm run tauri dev
```

1. Browse library folder (wav/flac) and output under `/tmp` or `$HOME`.  
2. Optionally set Feature DB; Automix folder.  
3. Listen; use Mix Now / `N`; Next matches; Automix Editor Apply / Save playlist.  
4. Advanced: playlist XML/JSON, analyze Browse, policy hints, Softmax Browse or Skip Softmax.

### 6.3 Matching CLI

```bash
cargo run -p autodj-cli -- suggest --folder /path/to/tracks --from /path/to/now.wav
cargo run -p autodj-cli -- remix --folder /path/to/tracks \
  --prefix /path/a.wav --next /path/c.wav --out /tmp/remix.wav
```

### 6.4 Train eval

```bash
cd python && pip install -e ".[dev]"
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
```

---

## 7. Bibliography

Paraphrase + link (from competitive sweep; not scraped corpora):

- djay Automix user manual — transition FX, Mix Now.  
- VirtualDJ Automix — Smart mix-in/out, Automix Editor concepts.  
- Serato Using Stems — stem pad model (Later for AutoDJ).  
- Rekordbox × DJ.Studio prep — Camelot / phrase messaging.  
- SetFlow transition technique — six-core transition vocabulary.  
- EBU R 128 / BS.1770 — programme loudness.  

Internal: all `Docs/*.md` listed in the front matter; MIT `LICENSE`.

---

## 8. Appendices

### Appendix A — Test evidence index

| Command / check | Expected artefact |
|-----------------|-------------------|
| `cargo test --workspace` | units pass |
| `mix --golden smoke --out /tmp/autodj-smoke.wav` | WAV + `.mixreport.json` |
| `mix --golden broadcast …` | −23 LUFS gates |
| `mix --golden rekordbox …` | preserve-order |
| `mix --golden transitions …` | preferred classes |
| `mix --golden transition_model …` | Softmax classes |
| `mix --golden cue_overrides …` | absolute cues |
| `mix --golden rekordbox_cue_overrides …` | Type 1/2 cues |
| `mix --golden transition_overrides …` | `transition_out` |
| `mix --golden policy_hints …` | hinted xfade |
| `analyze … loop_a_128.flac` | FLAC decode OK |
| `autodj-train eval …` | accuracy ≥ 0.90 |
| `cargo test -p autodj-io` | FLAC roundtrip |

### Appendix B — Data / legal sources

- Synthetic gold WAVs + `loop_a_128.flac` under `testdata/gold/`.  
- Synthetic Rekordbox XML under `testdata/rekordbox/`.  
- Offline tracklist/chart fixtures → aggregate `policy_hints` only.  
- Softmax trained on synthetic pairs; committed JSON artefact.  
- Forbidden: commercial set scrapes, unlicensed stems (see legal policy).

### Appendix C — Capability ledger (P0–P69)

Living checklist; **append** new P-rows here when shipping.

#### Library I/O

- Decode WAV + FLAC (`read_audio` / claxon) — **P53**  
- Encode WAV + FLAC (`write_audio` / flacenc) — **P67**  
- Feature DB Browse/New — **P65**

#### Analysis & cues

- AnalysisQuality / octave fold / spectral centroid — **P8**, **P38**  
- Beat-phase / peak gates — **P9**  
- Phrase cues, drop-aware, bars_per_phrase, LCM joins — **P1**, **P21**, **P24**, **P26**  
- Absolute cues JSON + Rekordbox Type 1/2 — **P33**, **P55**  
- Prep quality + UI pills — **P23**, **P45**

#### Solver & setcraft

- Energy-arc solver — **P4**  
- Key-route discount — **P41**  
- Softmax default + opt-out + goldens — **P6**, **P16**, **P20**  
- Softmax v2/v3 / 8-class Dissolve — **P25**, **P27**, **P36**  
- Policy hints — **P5**, **P54**  
- Matching suggest/remix — **P42**  
- `transition_out` pins — **P34**

#### FX / DSP / loudness

- EQ/Filter/Echo + bass gate — **P2**, **P14**  
- Tremolo / Riser / Dissolve — **P28–P31**, **P35–P36**  
- Class-aware xfade lengths — **P22**  
- Bidirectional WSOLA + stretch gates — **P3**, **P17**, **P32**  
- Spectral/join continuity — **P4**, **P12**  
- R128 −14/−23 — **P0**, **P49**, **P52**

#### CLI / CI / Tauri / train

- MixReport sidecar — **P57**; Reveal — **P61**  
- Rekordbox / transitions / cue goldens — **P13**, **P15**, **P33–P34**, **P55**  
- Folder Automix, Mix & Listen, Mix Now, Matching, Editor, Browse ladder, Skip Softmax, UI polish — **P37–P68**  
- Living writeup seed — **P69**; NEA-style expansion — **P70**  
- Softmax train/eval — **P6**

### Appendix D — How this Doc grows

1. Each new **P-row** PR: update sweep + **append** one evidence bullet under Appendix C (and a short Shipped note below).  
2. If FO/NFO change, update §1.6 and §2.6 in the same PR.  
3. Keep README Status to a few honest sentences linking here.  
4. Never invent clients, never invent features, never commit illegal audio.

### Shipped notes

- **2026-11-11 — P69:** Seeded living writeup + README refresh.  
- **2026-11-11 — P70:** Expanded into full professional Analysis→Evaluation structure (persona-based; developer-led; no clients).
