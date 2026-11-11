# Getting started

## Prerequisites

- **Primary platform:** macOS Apple Silicon (`aarch64-apple-darwin`). Offline DSP is pure Rust (EQ, WSOLA tempo blend, R128 — no Linux-only audio backends).
- Rust stable (1.75+)
- Python 3.12+ (train package)
- Optional desktop: Node 20+ for `apps/autodj-app` (Tauri 2)
- CI also runs the Rust suite on `macos-latest` and `ubuntu-latest` (hosted Mac runners need Actions minutes on private repos).

## First mix

```bash
cargo run -p autodj-cli -- mix --golden smoke --out /tmp/autodj-smoke.wav
# Rekordbox XML path (preserve-order setlist from testdata/rekordbox/gold_playlist.xml)
cargo run -p autodj-cli -- mix --golden rekordbox --out /tmp/autodj-rekordbox.wav
# Full builtin genre palette preferred classes (incl. uk_garage / open_format)
cargo run -p autodj-cli -- mix --golden transitions --out /tmp/autodj-transitions.wav
# Softmax artifact drives mix classes for all six builtins (committed transition_model.json)
cargo run -p autodj-cli -- mix --golden transition_model --out /tmp/autodj-transition-model.wav
# Optional: classic EBU R 128 programme loudness
# cargo run -p autodj-cli -- mix --golden smoke --lufs -23 --out /tmp/autodj-smoke.wav
afplay /tmp/autodj-smoke.wav   # macOS
```
Default export target is **−14 LUFS** (streaming-style). Pass `--lufs -23` for broadcast R 128.

## Your playlist

Create `party.json`:

```json
{
  "genre": "house",
  "tracks": [
    {"path": "/absolute/path/a.wav", "key": "8A", "bpm": 124},
    {"path": "/absolute/path/b.wav", "key": "9A", "bpm": 125}
  ]
}
```

```bash
cargo run -p autodj-cli -- analyze --input /absolute/path --db features.sqlite
# analyze lines include bpm_conf / key_conf / sparse / octave_corr (AnalysisQuality)
cargo run -p autodj-cli -- mix --playlist party.json --db features.sqlite --out mix.wav --genre house
# MixReport includes beat_phase_mean_ms / beat_phase_gate_pass / true_peak / peak_gate_pass
#   + max_join_flux_ratio / join_continuity_pass / spectral_continuity_pass
#   + max_bass_overlap / bass_conflict_gate_pass / transitions[{class,crossfade_secs}]
#   + max_stretch_clamp_excess / tempo_stretch_gate_pass
#   + prep_quality_gate_pass / prep_tracks (AnalysisQuality; BPM override trusts weak analyze)
# --deterministic (default true): fixed −14 LUFS + hard-fail prep/beat-phase/peak/join/bass/stretch gates
# --no-deterministic: relax hard gates (desktop QA checkbox off)

# Rekordbox DJ_PLAYLISTS XML (user export). Keeps setlist order with --preserve-order.
cargo run -p autodj-cli -- mix --playlist testdata/rekordbox/gold_playlist.xml \
  --preserve-order --playlist-name "Gold Set" --out mix.wav --genre house

# Ordinary mix auto-loads committed Softmax (per-edge classes). Opt out:
cargo run -p autodj-cli -- mix --playlist party.json --out mix.wav --no-transition-model
# Optional setcraft / classifier overrides
cargo run -p autodj-cli -- mix --playlist party.json --out mix.wav \
  --policy-hints python/autodj_train/datasets/aggregates/demo_policy_hints.json
cargo run -p autodj-cli -- mix --playlist party.json --out mix.wav \
  --transition-model python/autodj_train/models/artifacts/transition_model.json
```

Keys/BPM/`beat_offset_secs`/`intro_end`/`outro_start`/`bars_per_phrase` (`4`/`8`/`16`) in JSON (or Rekordbox TEMPO/Tonality/POSITION_MARK) override analyzer estimates when present. Absolute mix windows: optional `mix_in_secs` / `mix_out_secs` pin play start/end (beat-snapped; win over section heuristics — unlike `intro_end`/`outro_start`, which only bias the planner). Per-edge FX: optional `transition_out` (`hard_cut`, `eq_swap`, `filter_sweep`, `echo_out`, `tremolo`, `riser`, `dissolve`, `linear_crossfade`, …) pins the class for the edge **from this track to the next**, overriding Softmax/policy and recomputing class-aware crossfade length. Without an override, mix resolves phrase length per track via `estimate_bars_per_phrase` (section span) with a genre fallback. A playlist BPM override also satisfies the prep quality gate when analysis is sparse or low-confidence. Without `--preserve-order`, the solver may reorder for harmonic/energy-arc.

Cue / transition override goldens:

```bash
cargo run -p autodj-cli -- mix --golden cue_overrides --out /tmp/autodj-cues.wav
cargo run -p autodj-cli -- mix --golden transition_overrides --out /tmp/autodj-to.wav
```

Train gate (CI) — committed artifact is Softmax **v3** (15 dims, **8 classes** incl. dissolve):

```bash
cd python && pip install -e ".[dev]"
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
# Retrain (optional; use enough epochs for ≥0.90):
# autodj-train train --out autodj_train/models/artifacts/transition_model.json --n 20000 --seed 7 --epochs 20000 --lr 0.25
# Holdout regen (150×8): autodj-train synth --out autodj_train/eval/gold/synthetic_holdout.jsonl --n 1200 --seed 13 --balance
```

## Desktop shell (Tauri)

```bash
cargo build -p autodj-cli
cd apps/autodj-app
npm install
npm run tauri dev
```

Primary flow: **Browse…** a library folder of `wav`/`flac`, set output under `/tmp` or `$HOME`, leave **Preserve playlist order** unchecked, then **Automix folder**. AutoDJ writes a temp playlist, optionally analyzes into the Feature DB, Softmax-solves order, renders, and **Autoplay after mix** plays the continuous WAV. Click a track in **Set order (AutoDJ)** to seek; **Mix Now** jumps to the next transition (`join_timeline_secs`). The report panel also shows gates and transitions. JSON/Rekordbox playlists remain under **Advanced**. Strict QA (deterministic) needs trusted BPM (or playlist BPM overrides); gold loops may need Strict QA off. CLI resolution: `AUTODJ_BIN` → `autodj` on `PATH` → `cargo run -p autodj-cli` from the repo root.
