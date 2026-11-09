# autodj-app (Tauri 2)

Desktop shell: pick a playlist JSON, run analyze/mix via `autodj-cli`, watch progress, export WAV.

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
