# autodj-app (Tauri 2 stub)

Desktop shell that will queue a playlist, show analyze/mix progress, and export WAV via `autodj-cli` / `autodj-core`.

**Status:** stub only — scaffold Tauri 2 in a dedicated PR after CLI MVP is green on CI.

Planned:

```text
apps/autodj-app/
  src/           # web UI
  src-tauri/     # Rust commands wrapping autodj-core
```

Do not depend on RouteFinder `GlassmorphicModifiers`; AutoDJ defines its own UI language.
