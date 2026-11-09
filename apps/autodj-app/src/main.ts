import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

type TransitionEdge = {
  from: number;
  to: number;
  class: string;
  policy_crossfade_secs?: number;
  crossfade_secs?: number;
};

type MixReport = {
  order?: string[];
  genre?: string;
  out?: string;
  total_cost?: number;
  integrated_lufs?: number | null;
  target_lufs?: number | null;
  loudness_gate_pass?: boolean | null;
  spectral_continuity_pass?: boolean;
  spectral_flux_ratio?: number;
  cue_phrase_aligned?: boolean;
  tempo_blend_applied?: boolean;
  beat_phase_gate_pass?: boolean;
  beat_phase_mean_ms?: number;
  peak_gate_pass?: boolean | null;
  true_peak?: number | null;
  preserve_order?: boolean;
  join_continuity_pass?: boolean;
  max_join_flux_ratio?: number;
  bass_conflict_gate_pass?: boolean;
  max_bass_overlap?: number;
  tempo_stretch_gate_pass?: boolean;
  max_stretch_clamp_excess?: number;
  transitions?: TransitionEdge[];
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

function showPlainReport(text: string, error = false) {
  const panel = $<HTMLDivElement>("report-panel");
  const pre = $<HTMLPreElement>("report");
  if (panel) {
    panel.hidden = true;
    panel.innerHTML = "";
    panel.classList.remove("ok", "fail");
  }
  if (!pre) return;
  pre.hidden = false;
  pre.classList.toggle("error", error);
  pre.textContent = text;
}

function gateClass(v: boolean | null | undefined): string {
  if (v === true) return "pass";
  if (v === false) return "fail";
  return "na";
}

function gateLabel(v: boolean | null | undefined): string {
  if (v === true) return "pass";
  if (v === false) return "fail";
  return "n/a";
}

function basename(path: string | undefined): string {
  if (!path) return "?";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function fmt(n: number | null | undefined, digits = 3): string {
  if (n === null || n === undefined || Number.isNaN(n)) return "n/a";
  return n.toFixed(digits);
}

function hardGatesOk(r: MixReport): boolean {
  const hard: Array<boolean | null | undefined> = [
    r.loudness_gate_pass,
    r.beat_phase_gate_pass,
    r.peak_gate_pass,
    r.join_continuity_pass,
    r.bass_conflict_gate_pass,
    r.tempo_stretch_gate_pass,
  ];
  return hard.every((g) => g !== false);
}

function renderMixReport(r: MixReport, deterministic: boolean) {
  const panel = $<HTMLDivElement>("report-panel");
  const pre = $<HTMLPreElement>("report");
  if (pre) {
    pre.hidden = true;
    pre.textContent = "";
  }
  if (!panel) return;

  const gates: Array<[string, boolean | null | undefined]> = [
    ["loudness", r.loudness_gate_pass],
    ["spectral", r.spectral_continuity_pass],
    ["join", r.join_continuity_pass],
    ["bass", r.bass_conflict_gate_pass],
    ["beat-phase", r.beat_phase_gate_pass],
    ["peak", r.peak_gate_pass],
    ["stretch", r.tempo_stretch_gate_pass],
    ["cue", r.cue_phrase_aligned],
    ["tempo-blend", r.tempo_blend_applied],
  ];

  const ok = !deterministic || hardGatesOk(r);
  panel.hidden = false;
  panel.classList.toggle("ok", ok);
  panel.classList.toggle("fail", !ok);

  const summary = document.createElement("div");
  summary.className = "report-summary";
  summary.textContent = [
    `genre: ${r.genre ?? "?"}`,
    `preserve_order: ${String(r.preserve_order ?? false)}`,
    `tracks: ${(r.order ?? []).length}`,
    `cost: ${fmt(r.total_cost, 2)}`,
    `out: ${basename(r.out)}`,
  ].join(" · ");

  const strip = document.createElement("div");
  strip.className = "gate-strip";
  strip.setAttribute("aria-label", "Mix quality gates");
  for (const [name, value] of gates) {
    const pill = document.createElement("span");
    pill.className = `gate ${gateClass(value)}`;
    pill.textContent = `${name}: ${gateLabel(value)}`;
    strip.appendChild(pill);
  }

  const metrics = document.createElement("p");
  metrics.className = "report-metrics";
  metrics.textContent = [
    `lufs ${fmt(r.integrated_lufs, 2)} / target ${fmt(r.target_lufs, 1)}`,
    `true_peak ${fmt(r.true_peak, 4)}`,
    `beat_phase_ms ${fmt(r.beat_phase_mean_ms, 1)}`,
    `flux ${fmt(r.spectral_flux_ratio, 2)}`,
    `join_flux ${fmt(r.max_join_flux_ratio, 2)}`,
    `bass_overlap ${fmt(r.max_bass_overlap, 3)}`,
    `stretch_excess ${fmt(r.max_stretch_clamp_excess, 4)}`,
  ].join(" · ");

  const transitionsWrap = document.createElement("div");
  transitionsWrap.className = "transitions";
  const h = document.createElement("h3");
  h.className = "transitions-title";
  h.textContent = "Transitions";
  transitionsWrap.appendChild(h);
  const ul = document.createElement("ul");
  const edges = r.transitions ?? [];
  if (edges.length === 0) {
    const li = document.createElement("li");
    li.textContent = "(none)";
    ul.appendChild(li);
  } else {
    for (const e of edges) {
      const li = document.createElement("li");
      const xfade = fmt(e.crossfade_secs, 2);
      const policy = fmt(e.policy_crossfade_secs, 2);
      li.textContent = `${e.from}→${e.to}: ${e.class} (xfade ${xfade}s, policy ${policy}s)`;
      ul.appendChild(li);
    }
  }
  transitionsWrap.appendChild(ul);

  panel.replaceChildren(summary, strip, metrics, transitionsWrap);
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
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-browse-out")?.addEventListener("click", async () => {
    try {
      await browseOut();
    } catch (e) {
      showPlainReport(String(e), true);
      appendLog(String(e));
    }
  });

  $<HTMLButtonElement>("btn-version")?.addEventListener("click", async () => {
    setBusy(true);
    appendLog("--- version ---");
    try {
      const v = await invoke<string>("autodj_version");
      appendLog(v);
      showPlainReport(v);
    } catch (e) {
      showPlainReport(String(e), true);
      appendLog(String(e));
    } finally {
      setBusy(false);
    }
  });

  $<HTMLButtonElement>("btn-analyze")?.addEventListener("click", async () => {
    const input = $<HTMLInputElement>("analyze-input")?.value.trim() ?? "";
    const db = $<HTMLInputElement>("db")?.value.trim() || "features.sqlite";
    if (!input) {
      showPlainReport("Set analyze input path", true);
      return;
    }
    setBusy(true);
    appendLog("--- analyze ---");
    try {
      const out = await invoke<string>("analyze", { args: { input, db } });
      appendLog(out.trim());
      showPlainReport(out.trim() || "analyze ok");
    } catch (e) {
      showPlainReport(String(e), true);
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
    const deterministic =
      $<HTMLInputElement>("deterministic")?.checked ?? true;
    if (!playlist || !out) {
      showPlainReport(
        "Set playlist (JSON or Rekordbox XML) and output WAV paths",
        true,
      );
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
          deterministic,
        },
      });
      renderMixReport(report, deterministic);
      appendLog("mix complete");
    } catch (e) {
      showPlainReport(String(e), true);
      appendLog(String(e));
    } finally {
      setBusy(false);
    }
  });
});
