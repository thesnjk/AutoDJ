import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

type MixReport = {
  order?: string[];
  genre?: string;
  out?: string;
  integrated_lufs?: number | null;
  loudness_gate_pass?: boolean | null;
  spectral_continuity_pass?: boolean;
  cue_phrase_aligned?: boolean;
  tempo_blend_applied?: boolean;
  beat_phase_gate_pass?: boolean;
  peak_gate_pass?: boolean | null;
  total_cost?: number;
};

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T | null;

function appendLog(line: string) {
  const log = $<HTMLPreElement>("log");
  if (!log) return;
  log.textContent = `${log.textContent ?? ""}${line}\n`;
  log.scrollTop = log.scrollHeight;
}

function setBusy(busy: boolean) {
  for (const id of [
    "btn-version",
    "btn-analyze",
    "btn-mix",
    "btn-browse-playlist",
    "btn-browse-out",
  ]) {
    const btn = $<HTMLButtonElement>(id);
    if (btn) btn.disabled = busy;
  }
}

function showReport(text: string, error = false) {
  const el = $<HTMLPreElement>("report");
  if (!el) return;
  el.hidden = false;
  el.classList.toggle("error", error);
  el.textContent = text;
}

function formatReport(r: MixReport): string {
  const lines = [
    `genre: ${r.genre ?? "?"}`,
    `out: ${r.out ?? "?"}`,
    `tracks: ${(r.order ?? []).length}`,
    `loudness_gate: ${String(r.loudness_gate_pass)}`,
    `spectral_continuity: ${String(r.spectral_continuity_pass)}`,
    `cue_phrase_aligned: ${String(r.cue_phrase_aligned)}`,
    `tempo_blend: ${String(r.tempo_blend_applied)}`,
    `beat_phase_gate: ${String(r.beat_phase_gate_pass)}`,
    `peak_gate: ${String(r.peak_gate_pass)}`,
    `integrated_lufs: ${r.integrated_lufs ?? "n/a"}`,
    `total_cost: ${r.total_cost ?? "n/a"}`,
  ];
  if (r.order?.length) {
    lines.push("order:");
    for (const p of r.order) {
      lines.push(`  ${p}`);
    }
  }
  return lines.join("\n");
}

async function refreshBin() {
  const el = $<HTMLParagraphElement>("bin-path");
  try {
    const resolved = await invoke<string>("resolve_autodj_bin");
    if (el) el.textContent = resolved;
  } catch (e) {
    if (el) el.textContent = `resolve failed: ${e}`;
  }
}

async function browsePlaylist() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Select playlist",
    filters: [{ name: "Playlist", extensions: ["json", "xml"] }],
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("playlist");
    if (el) el.value = selected;
  }
}

async function browseOut() {
  const selected = await save({
    title: "Export mix WAV",
    filters: [{ name: "WAV", extensions: ["wav"] }],
    defaultPath: "mix.wav",
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("out");
    if (el) el.value = selected;
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  await listen<{ line: string }>("autodj-progress", (ev) => {
    appendLog(ev.payload.line);
  });

  await refreshBin();

  const outEl = $<HTMLInputElement>("out");
  if (outEl) outEl.value = "/tmp/autodj-mix.wav";

  $<HTMLButtonElement>("btn-browse-playlist")?.addEventListener(
    "click",
    async () => {
      try {
        await browsePlaylist();
      } catch (e) {
        showReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-browse-out")?.addEventListener("click", async () => {
    try {
      await browseOut();
    } catch (e) {
      showReport(String(e), true);
      appendLog(String(e));
    }
  });

  $<HTMLButtonElement>("btn-version")?.addEventListener("click", async () => {
    setBusy(true);
    appendLog("--- version ---");
    try {
      const v = await invoke<string>("autodj_version");
      appendLog(v);
      showReport(v);
    } catch (e) {
      showReport(String(e), true);
      appendLog(String(e));
    } finally {
      setBusy(false);
    }
  });

  $<HTMLButtonElement>("btn-analyze")?.addEventListener("click", async () => {
    const input = $<HTMLInputElement>("analyze-input")?.value.trim() ?? "";
    const db = $<HTMLInputElement>("db")?.value.trim() || "features.sqlite";
    if (!input) {
      showReport("Set analyze input path", true);
      return;
    }
    setBusy(true);
    appendLog("--- analyze ---");
    try {
      const out = await invoke<string>("analyze", { args: { input, db } });
      appendLog(out.trim());
      showReport(out.trim() || "analyze ok");
    } catch (e) {
      showReport(String(e), true);
      appendLog(String(e));
    } finally {
      setBusy(false);
    }
  });

  $<HTMLButtonElement>("btn-mix")?.addEventListener("click", async () => {
    const playlist = $<HTMLInputElement>("playlist")?.value.trim() ?? "";
    const out = $<HTMLInputElement>("out")?.value.trim() ?? "";
    const genre = $<HTMLSelectElement>("genre")?.value ?? "house";
    const policyHints = $<HTMLInputElement>("policy-hints")?.value.trim() || null;
    const transitionModel =
      $<HTMLInputElement>("transition-model")?.value.trim() || null;
    const preserveOrder =
      $<HTMLInputElement>("preserve-order")?.checked ?? false;
    const playlistName =
      $<HTMLInputElement>("playlist-name")?.value.trim() || null;
    if (!playlist || !out) {
      showReport("Set playlist (JSON or Rekordbox XML) and output WAV paths", true);
      return;
    }
    setBusy(true);
    appendLog("--- mix ---");
    try {
      const report = await invoke<MixReport>("mix", {
        args: {
          playlist,
          out,
          genre,
          policyHints,
          transitionModel,
          preserveOrder,
          playlistName,
        },
      });
      showReport(formatReport(report));
      appendLog("mix complete");
    } catch (e) {
      showReport(String(e), true);
      appendLog(String(e));
    } finally {
      setBusy(false);
    }
  });
});
