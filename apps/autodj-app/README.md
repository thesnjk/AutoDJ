# autodj-app (Tauri 2)

Desktop shell: **Automix folder** scans a library directory for `wav`/`flac`, Softmax-solves order, renders, and **listens** in-app (`<audio>` + Autoplay after mix). **Next matches** ranks neighbors for the now-playing track; **Queue & remix** pins a pick after the played prefix and re-solves the rest. **Mix Now** / `N` seeks to the next join; `Shift+N` seeks to the previous; cue-segment spans + join ticks mark the mix; click a segment, tick, Transition row, or Set-order row (Camelot + BPM + phrase bars + prep trusted/weak) to seek (`join_timeline_secs` / `mix_timeline_*`); active/next join chrome + class listen hints update with the playhead. Transitions list key route / ΔBPM / edge cost. Gate pills show mix-quality rubric tooltips plus cue/transition/phrase override flags; **Listen while playing** opens the human checklist. Loudness target: Streaming (−14) / Broadcast (−23); Strict QA locks −14. JSON/Rekordbox playlists remain under Advanced. Softmax + solver pick order when Preserve-order is off. Output WAV must be under `/tmp` or `$HOME` (asset protocol scope). Feature DB is passed to `mix --db` after folder analyze.

**UI language:** AutoDJ’s own (not RouteFinder GlassmorphicModifiers).

## Dev (macOS Apple Silicon)

From the AutoDJ repo root:

```bash
# Ensure CLI builds
cargo build -p autodj-cli

cd apps/autodj-app
npm install
npm run tauri dev
```

Use **Browse…** for a library folder (e.g. `testdata/gold`) and output WAV, then **Automix folder**. For Rekordbox XML, open Advanced → playlist picker; enable preserve-order and set playlist name `Gold Set` if desired.

Resolution order for the CLI:

1. `AUTODJ_BIN` env (absolute path to `autodj`)
2. `autodj` on `PATH`
3. `cargo run -p autodj-cli` from the repo root (auto-detected)

Optional: `AUTODJ_REPO_ROOT` if the app cannot find the workspace.

## Layout

```text
apps/autodj-app/
  src/           # Vite + TypeScript UI
  src-tauri/     # Tauri commands wrapping autodj-cli
```
