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

Keys/BPM/`beat_offset_secs`/`intro_end`/`outro_start`/`bars_per_phrase` (`4`/`8`/`16`) in JSON (or Rekordbox TEMPO/Tonality/POSITION_MARK) override analyzer estimates when present. Without an override, mix resolves phrase length per track via `estimate_bars_per_phrase` (section span) with a genre fallback. A playlist BPM override also satisfies the prep quality gate when analysis is sparse or low-confidence. Without `--preserve-order`, the solver may reorder for harmonic/energy-arc.

Train gate (CI) — committed artifact is Softmax **v2** (13 dims: stretch + spectral delta + genres):

```bash
cd python && pip install -e ".[dev]"
autodj-train eval --model autodj_train/models/artifacts/transition_model.json
# Retrain (optional): autodj-train train --out autodj_train/models/artifacts/transition_model.json --n 2000 --seed 7
```

## Desktop shell (Tauri)

```bash
cargo build -p autodj-cli
cd apps/autodj-app
npm install
npm run tauri dev
```

Use **Browse…** to pick a playlist JSON or Rekordbox XML (e.g. `testdata/rekordbox/gold_playlist.xml`), set an output WAV, optionally enable **Preserve playlist order**, a Rekordbox playlist NODE name, and **Strict QA gates (deterministic)** (default on). After Mix, the report panel shows a gate strip (loudness/join/bass/stretch/…) plus per-edge transitions. CLI resolution: `AUTODJ_BIN` → `autodj` on `PATH` → `cargo run -p autodj-cli` from the repo root.
