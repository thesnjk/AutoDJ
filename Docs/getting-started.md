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
# MixReport sidecar lands next to the WAV (override with --report-out PATH)
# → /tmp/autodj-smoke.wav.mixreport.json
# EBU R 128 broadcast loudness (−23 LUFS) on the same gold loops
cargo run -p autodj-cli -- mix --golden broadcast --out /tmp/autodj-broadcast.wav
# Rekordbox XML path (preserve-order setlist from testdata/rekordbox/gold_playlist.xml)
cargo run -p autodj-cli -- mix --golden rekordbox --out /tmp/autodj-rekordbox.wav
# Rekordbox Type 1/2 → absolute mix windows
cargo run -p autodj-cli -- mix --golden rekordbox_cue_overrides --out /tmp/autodj-rb-cues.wav
# Full builtin genre palette preferred classes (incl. uk_garage / open_format)
cargo run -p autodj-cli -- mix --golden transitions --out /tmp/autodj-transitions.wav
# Softmax artifact drives mix classes for all six builtins (committed transition_model.json)
cargo run -p autodj-cli -- mix --golden transition_model --out /tmp/autodj-transition-model.wav
# Offline setcraft policy hints (house crossfade 12s vs builtin 8s)
cargo run -p autodj-cli -- mix --golden policy_hints --out /tmp/autodj-policy-hints.wav
# Ordinary playlist broadcast target (Strict QA off; equals form so clap accepts the negative)
# cargo run -p autodj-cli -- mix --playlist party.json --no-deterministic --lufs=-23 --out /tmp/autodj-broadcast-mix.wav
afplay /tmp/autodj-smoke.wav   # macOS
```
Default export target is **−14 LUFS** (streaming-style). Pass `--lufs=-23` for broadcast R 128, or use `--golden broadcast` for the CI-locked −23 suite. Successful `mix` / `remix` also write MixReport JSON beside the WAV as `{out}.mixreport.json` (override with `--report-out`); stdout JSON is unchanged for scripting / Tauri. Library folders may include **WAV or FLAC** (decoded via `read_audio`); export remains WAV. Smoke FLAC: `cargo run -p autodj-cli -- analyze --input testdata/gold/loop_a_128.flac --db /tmp/flac.sqlite`.

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

Keys/BPM/`beat_offset_secs`/`intro_end`/`outro_start`/`bars_per_phrase` (`4`/`8`/`16`) in JSON (or Rekordbox TEMPO/Tonality) override analyzer estimates when present. Absolute mix windows: optional `mix_in_secs` / `mix_out_secs` (JSON) or Rekordbox `POSITION_MARK` Type 1/2 pin play start/end (beat-snapped; win over section heuristics — unlike `intro_end`/`outro_start`, which only bias the planner). Per-edge FX: optional `transition_out` (`hard_cut`, `eq_swap`, `filter_sweep`, `echo_out`, `tremolo`, `riser`, `dissolve`, `linear_crossfade`, …) pins the class for the edge **from this track to the next**, overriding Softmax/policy and recomputing class-aware crossfade length. Without an override, mix resolves phrase length per track via `estimate_bars_per_phrase` (section span) with a genre fallback. A playlist BPM override also satisfies the prep quality gate when analysis is sparse or low-confidence. Without `--preserve-order`, the solver may reorder for harmonic/energy-arc (DJ key-routes from `suggest_next_keys` get a bounded cost discount).

Cue / transition override goldens:

```bash
cargo run -p autodj-cli -- mix --golden cue_overrides --out /tmp/autodj-cues.wav
cargo run -p autodj-cli -- mix --golden rekordbox_cue_overrides --out /tmp/autodj-rb-cues.wav
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

Primary flow: **Browse…** a library folder of `wav`/`flac`, set output under `/tmp` or `$HOME`, leave **Preserve playlist order** unchecked, then **Automix folder**. AutoDJ writes a temp playlist, optionally analyzes into the Feature DB, Softmax-solves order, renders, and **Autoplay after mix** plays the continuous WAV. **Next matches** lists top neighbors for the now-playing track; **Queue & remix** keeps the played prefix, pins your pick, and re-solves the remainder. Click a track in **Set order (AutoDJ)** to seek (rows show Camelot + BPM + phrase bars + prep trusted/weak); the player shows cue-segment spans plus join ticks; click a segment, tick, or Transition row to seek; playhead lights the active/next join and a class listen hint within ±4 s of a blend; **Mix Now** / `N` jumps to the next join (`join_timeline_secs`); `Shift+N` jumps to the previous. The report panel also shows gates (hover for rubric criteria; Strict QA fail callout; **Listen while playing** checklist), **Reveal MixReport…** for `{out}.mixreport.json`, and Transitions with key route / ΔBPM / edge cost. **Automix Editor:** after a mix, fields seed from computed cue play windows, edge transition classes, and phrase bars; tweak mix-in/out + phrase (4/8/16) on Set order and transition type on Transitions, then **Apply & re-mix** (locked order; clear / Auto drops that pin) or **Save playlist…** for a CLI-compatible JSON that also fills Advanced `#playlist` and turns on preserve-order (`autodj mix --playlist … --preserve-order`). **Reset overrides** re-seeds from the latest MixReport. JSON/Rekordbox playlists remain under **Advanced**; there **Browse folder…** / **Browse file…** fill Analyze input, **Browse…** / **Load demo** fill Policy hints, and **Browse…** / **Load committed** fill the Softmax transition model path (empty still auto-loads the committed artifact). Strict QA (deterministic) needs trusted BPM (or playlist BPM overrides) and locks loudness to −14 LUFS; with Strict QA off, pick **Streaming (−14)** or **Broadcast (−23)**. Gold loops may need Strict QA off. CLI: `autodj suggest --folder … --from …` / `autodj remix --folder … --prefix … --next …`. CLI resolution: `AUTODJ_BIN` → `autodj` on `PATH` → `cargo run -p autodj-cli` from the repo root.
