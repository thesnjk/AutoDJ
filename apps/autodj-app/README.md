# autodj-app (Tauri 2)

Desktop shell for AutoDJ’s **full-auto private DJ** path (P80–P83). Deep user guide: [`Docs/getting-started.md`](../../Docs/getting-started.md). North Star ladder: [`Docs/full-auto-roadmap.md`](../../Docs/full-auto-roadmap.md).

**Primary flow — continuous radio**

1. **Radio libraries** — Add one or more local folders you own (`wav`/`flac`); list persists in `localStorage`.
2. **Feature DB** — Browse/New on the main panel; optional **Index libraries** (`analyze_libraries`).
3. **Set length** — Continuous or 30 / 45 / 60 / 90 min; **Vibe** — Standard (house) / Chill (open_format) / Peak (trance).
4. **Start radio** — short Automix seed (`horizon` 3), Softmax next-track picks, **−20s** prerender before the upcoming join, hold-then-swap at handoff; timed sets **auto-stop**. **Stop radio** ends the loop (works while CLI busy).

**Power-user paths**

- **Automix folder** — one-shot mix across Radio libraries (or a single **Library folder**).
- **Next matches** / **Queue & remix** — manual Softmax neighbors for the now-playing track (multi-root via temp symlink staging).
- **Mix Now** / `N` / `Shift+N` — seek joins; cue spans + Automix Editor; Advanced playlist / Softmax / MixReport path / Peak limit only.

Output audio (`.wav`/`.flac`) must be under `/tmp` or `$HOME` (asset protocol). Strict QA locks −14 LUFS; otherwise Streaming (−14) / Broadcast (−23).

**UI language:** AutoDJ’s own (not RouteFinder GlassmorphicModifiers).

## Dev (macOS Apple Silicon)

From the AutoDJ repo root:

```bash
cargo build -p autodj-cli

cd apps/autodj-app
npm install
npm run tauri dev
```

Smoke: Add `testdata/gold` as a Radio library root, set output under `/tmp`, **Start radio** (Strict QA off for short gold loops). Or **Automix folder** for a one-shot render. Rekordbox XML: Advanced → playlist + preserve-order + playlist name.

CLI resolution:

1. `AUTODJ_BIN` (absolute path to `autodj`)
2. `autodj` on `PATH`
3. `cargo run -p autodj-cli` from the repo root

Optional: `AUTODJ_REPO_ROOT` if the app cannot find the workspace.

## Layout

```text
apps/autodj-app/
  src/           # Vite + TypeScript UI
  src-tauri/     # Tauri commands wrapping autodj-cli
```
