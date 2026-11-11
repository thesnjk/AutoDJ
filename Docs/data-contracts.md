# AutoDJ data contracts

Normative shapes for playlist ingest, MixReport export, Softmax artefacts, Feature DB, and policy hints. Extracted from shipped code (`crates/autodj-cli`, `autodj-graph`, `python/autodj_train`). Do not invent fields.

Companion: [`project-writeup.md`](project-writeup.md) §2.3 · [`architecture.md`](architecture.md) · [`getting-started.md`](getting-started.md)

---

## 1. Playlist JSON

Root object (CLI `mix --playlist` JSON path):

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `genre` | string | yes (or CLI `--genre`) | Builtin policy id (`house`, `trance`, …) |
| `tracks` | array | yes | Ordered list of track objects |

### 1.1 Track object (`PlaylistTrack`)

Source: `crates/autodj-cli/src/main.rs` `PlaylistTrack`.

| Field | Type | Default | Semantics |
|-------|------|---------|-----------|
| `path` | string | required | Absolute or resolvable path to WAV/FLAC |
| `key` | string? | null | Camelot / key override |
| `bpm` | number? | null | BPM override (prep-trusted when set) |
| `beat_offset_secs` | number? | null | Downbeat offset override |
| `intro_end` | number? | null | Section mark (secs) |
| `outro_start` | number? | null | Section mark (secs) |
| `mix_in_secs` | number? | null | Absolute mix-in; overrides section heuristics |
| `mix_out_secs` | number? | null | Absolute mix-out; overrides section heuristics |
| `transition_out` | string? | null | Snake_case `TransitionClass` for edge to next |
| `bars_per_phrase` | u32? | null | Must be `4`, `8`, or `16` when set |

Rekordbox XML is an alternate ingest; Type 1/2 markers map to `mix_in_secs` / `mix_out_secs`. See goldens `rekordbox` / `rekordbox_cue_overrides`.

---

## 2. MixReport JSON

Stdout + sidecar `{out}.mixreport.json` (override `--report-out`). Source: `MixReport`, `CueReport`, `PrepTrackReport`, `TransitionEdgeReport`.

### 2.1 Root (`MixReport`)

| Field | Type | Semantics |
|-------|------|-----------|
| `order` | string[] | Absolute paths in mix order |
| `total_cost` | f32 | Solver path cost |
| `genre` | string | Policy id used |
| `out` | string | Output audio path |
| `integrated_lufs` | f64? | Measured integrated loudness |
| `target_lufs` | f64? | Normalize target (−14 / −23 / null if peak-limit) |
| `loudness_gate_pass` | bool? | Within ±2 LU of target |
| `cues` | CueReport[] | Per-track play windows + timeline |
| `cue_phrase_aligned` | bool | Phrase planner residual OK / skip-with-pass |
| `cue_mix_override_applied` | bool | Absolute mix-in/out used |
| `transition_override_applied` | bool | `transition_out` pins used |
| `cue_drop_aware` | bool | Drop-ordered section marks |
| `per_track_phrase_applied` | bool | All cues have bars ∈ {4,8,16} |
| `prep_tracks` | PrepTrackReport[] | Per-track analysis quality |
| `prep_quality_gate_pass` | bool | All tracks prep-trusted |
| `tempo_blend_applied` | bool | Any WSOLA multi-BPM blend |
| `spectral_flux_ratio` | f32 | Adjacent hop-RMS ratio |
| `spectral_continuity_pass` | bool | ≤ 4.0 default |
| `max_join_flux_ratio` | f32 | Worst join neighbourhood ratio |
| `join_continuity_pass` | bool | ≤ 6.0 default |
| `beat_phase_mean_ms` | f32 | Mean same-tempo join offset |
| `beat_phase_gate_pass` | bool | < 50 ms |
| `true_peak` | f64? | Linear FS peak |
| `peak_gate_pass` | bool? | ≤ 0.99 FS |
| `preserve_order` | bool | Playlist order locked |
| `max_bass_overlap` | f32 | Worst EqSwap bass ownership |
| `bass_conflict_gate_pass` | bool | ≤ 0.40 |
| `transitions` | TransitionEdgeReport[] | Per-edge decisions |
| `max_stretch_clamp_excess` | f32 | Worst ±15% clamp excess |
| `tempo_stretch_gate_pass` | bool | Excess == 0 |
| `join_timeline_secs` | f64[] | Mix Now seek points |

### 2.2 `CueReport`

| Field | Type | Semantics |
|-------|------|-----------|
| `track` | string | Path |
| `play_start_secs` / `play_end_secs` | f64 | Within-source window |
| `crossfade_secs` | f32 | Cue-planned xfade |
| `bpm` | f32 | |
| `key` | string | |
| `bpm_to` | f32? | Next track BPM |
| `bars_per_phrase` | u32 | 4 / 8 / 16 |
| `mix_timeline_start_secs` / `mix_timeline_end_secs` | f64 | Position in rendered mix |

### 2.3 `PrepTrackReport`

| Field | Type | Semantics |
|-------|------|-----------|
| `track` | string | |
| `sparse_audio` | bool | |
| `bpm_confidence` / `key_confidence` | f32 | |
| `bpm_octave_corrected` | bool | |
| `bpm_overridden` / `key_overridden` / `beat_offset_overridden` | bool | Playlist pins |
| `trusted` | bool | `!sparse && bpm_conf ≥ 0.25` **or** BPM override |

### 2.4 `TransitionEdgeReport`

| Field | Type | Semantics |
|-------|------|-----------|
| `from` / `to` | usize | Indices into solved order |
| `class` | TransitionClass | Enum / snake_case in JSON |
| `policy_crossfade_secs` | f32 | Before cue snap |
| `crossfade_secs` | f32 | Rendered |
| `key_from` / `key_to` | string | |
| `camelot_distance` | u8 | |
| `bpm_delta` | f32 | |
| `key_on_route` | bool | `suggest_next_keys` |
| `edge_cost` | f32 | Pairwise solver cost |

Gate pass criteria: [`mix-quality-rubric.md`](mix-quality-rubric.md).

---

## 3. Softmax transition model JSON

Trained by `autodj-train`; consumed by Rust `TransitionModel`. Feature lockstep: `python/autodj_train/models/features.py` ↔ `crates/autodj-graph/src/transition_model.rs`.

| Version | Numeric dims | + genre one-hots | Total | Extra numerics vs prior |
|---------|-------------:|-----------------:|------:|-------------------------|
| v1 | 5 | 6 | 11 | bpm_a, bpm_b, \|ΔBPM\|, Camelot dist, energy ΔdB |
| v2 | 7 | 6 | 13 | + stretch_clamp_excess, spectral_centroid_delta_hz |
| v3 (default) | 9 | 6 | 15 | + bars_per_phrase_a, \|bars_a−bars_b\| |

Classes (v3, eight): `linear_crossfade`, `eq_swap`, `filter_sweep`, `echo_out`, `tremolo`, `riser`, `dissolve`, `hard_cut`.

Committed default: `python/autodj_train/models/artifacts/transition_model.json` (`version: 3`). Legacy snapshots (`transition_model_v1.json`, `transition_model_v2.json`, `transition_model_pre_*.json`) still load via version/class dispatch.

Stretch clamp shared with DSP: ratio outside `[0.85, 1.15]` → excess (must match `autodj_core::stretch`).

---

## 4. Feature DB (SQLite)

`CREATE TABLE tracks` from `open_db` in `crates/autodj-cli/src/main.rs`:

| Column | Type | Notes |
|--------|------|-------|
| `path` | TEXT PK | Absolute path |
| `bpm` | REAL | |
| `key` | TEXT | |
| `loudness_db` | REAL | |
| `duration_secs` | REAL | |
| `sample_rate` | INTEGER | |
| `intro_end` / `drop_start` / `outro_start` | REAL | Section marks |
| `spectral_centroid_hz` | REAL | |
| `json` | TEXT | Full `TrackFeatures` serde blob |

Upsert on `path` conflict. Used by `analyze --db` and mix/suggest when `--db` set.

---

## 5. Policy hints JSON

From `autodj-train aggregate-tracklists --hints-out` / `policy_hints_from_stats`:

```json
{
  "<genre_id>": {
    "bpm_center": 128.0,
    "default_crossfade_secs": 8.0
  }
}
```

Demo fixture: `python/autodj_train/datasets/aggregates/demo_policy_hints.json`. Merged into builtin `GenrePolicy` via CLI `--policy-hints` / Tauri Browse + Load demo. No HTML / track identities in committed aggregates ([`legal-data-policy.md`](legal-data-policy.md)).

---

## 6. Transition class vocabulary

Shared Rust enum `TransitionClass` (`autodj-core`): `LinearCrossfade`, `EqSwap`, `FilterSweep`, `EchoOut`, `Tremolo`, `Riser`, `Dissolve`, `HardCut`. Playlist / JSON use snake_case (`eq_swap`, …).

---

## 7. Suggest / matching stdout

`SuggestCandidateReport`: `path`, `cost`, `key`, `bpm`, `class`, `on_route` (plus ranking context from CLI). Consumed by Tauri Next matches.
