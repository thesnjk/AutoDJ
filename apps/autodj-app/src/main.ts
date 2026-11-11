import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

type TransitionEdge = {
  from: number;
  to: number;
  class: string;
  policy_crossfade_secs?: number;
  crossfade_secs?: number;
};

type CueReport = {
  track: string;
  play_start_secs: number;
  play_end_secs: number;
  crossfade_secs: number;
  bpm: number;
  bpm_to?: number | null;
  bars_per_phrase: number;
  mix_timeline_start_secs: number;
  mix_timeline_end_secs: number;
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
  prep_quality_gate_pass?: boolean;
  cue_drop_aware?: boolean;
  transitions?: TransitionEdge[];
  cues?: CueReport[];
  join_timeline_secs?: number[];
};

/** Last successful MixReport — drives now-playing highlight + Mix Now. */
let lastMixReport: MixReport | null = null;

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
    "btn-automix-folder",
    "btn-mix-now",
    "btn-browse-playlist",
    "btn-browse-folder",
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
    r.prep_quality_gate_pass,
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
    ["prep", r.prep_quality_gate_pass],
    ["loudness", r.loudness_gate_pass],
    ["spectral", r.spectral_continuity_pass],
    ["join", r.join_continuity_pass],
    ["bass", r.bass_conflict_gate_pass],
    ["beat-phase", r.beat_phase_gate_pass],
    ["peak", r.peak_gate_pass],
    ["stretch", r.tempo_stretch_gate_pass],
    ["cue", r.cue_phrase_aligned],
    ["drop", r.cue_drop_aware],
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

  const orderWrap = document.createElement("div");
  orderWrap.className = "set-order";
  const orderTitle = document.createElement("h3");
  orderTitle.className = "transitions-title";
  orderTitle.textContent = r.preserve_order
    ? "Set order (playlist)"
    : "Set order (AutoDJ)";
  orderWrap.appendChild(orderTitle);
  const orderUl = document.createElement("ol");
  orderUl.id = "set-order-list";
  const tracks = r.order ?? [];
  const cues = r.cues ?? [];
  if (tracks.length === 0) {
    const li = document.createElement("li");
    li.textContent = "(none)";
    orderUl.appendChild(li);
  } else {
    tracks.forEach((path, i) => {
      const li = document.createElement("li");
      li.dataset.index = String(i);
      const cue = cues[i];
      if (cue) {
        li.dataset.start = String(cue.mix_timeline_start_secs);
        li.dataset.end = String(cue.mix_timeline_end_secs);
        li.title = `${path} · ${fmt(cue.mix_timeline_start_secs, 1)}s–${fmt(cue.mix_timeline_end_secs, 1)}s`;
      } else {
        li.title = path;
      }
      li.textContent = basename(path);
      li.tabIndex = 0;
      li.setAttribute("role", "button");
      li.addEventListener("click", () => seekToTrack(i));
      li.addEventListener("keydown", (ev) => {
        if (ev.key === "Enter" || ev.key === " ") {
          ev.preventDefault();
          seekToTrack(i);
        }
      });
      orderUl.appendChild(li);
    });
  }
  orderWrap.appendChild(orderUl);

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

  panel.replaceChildren(summary, strip, metrics, orderWrap, transitionsWrap);
}

/** Load the rendered mix into the HTML5 player; autoplay when requested. */
async function loadMixPlayer(outPath: string, autoplay: boolean) {
  const audio = $<HTMLAudioElement>("mix-player");
  if (!audio || !outPath) return;
  audio.src = convertFileSrc(outPath);
  audio.load();
  if (!autoplay) return;
  try {
    await audio.play();
  } catch (e) {
    appendLog(`autoplay blocked or failed: ${e}`);
  }
}

function seekToTrack(index: number) {
  const audio = $<HTMLAudioElement>("mix-player");
  const cue = lastMixReport?.cues?.[index];
  if (!audio || !cue) return;
  audio.currentTime = Math.max(0, cue.mix_timeline_start_secs);
  void audio.play().catch(() => undefined);
  highlightPlayingTrack(index);
}

function mixNowSeek() {
  const audio = $<HTMLAudioElement>("mix-player");
  const joins = lastMixReport?.join_timeline_secs ?? [];
  if (!audio || joins.length === 0) {
    appendLog("Mix Now: no joins in last mix");
    return;
  }
  const t = audio.currentTime;
  const next = joins.find((j) => j > t + 0.05);
  if (next === undefined) {
    appendLog("Mix Now: already past last transition");
    return;
  }
  audio.currentTime = next;
  void audio.play().catch(() => undefined);
  appendLog(`Mix Now → ${fmt(next, 2)}s`);
  updateNowPlayingHighlight();
}

function highlightPlayingTrack(index: number) {
  const list = $<HTMLOListElement>("set-order-list");
  if (!list) return;
  list.querySelectorAll("li").forEach((li, i) => {
    li.classList.toggle("playing", i === index);
  });
}

function updateNowPlayingHighlight() {
  const audio = $<HTMLAudioElement>("mix-player");
  const cues = lastMixReport?.cues;
  if (!audio || !cues || cues.length === 0) return;
  const t = audio.currentTime;
  let idx = 0;
  for (let i = 0; i < cues.length; i++) {
    if (t + 1e-3 >= cues[i].mix_timeline_start_secs) {
      idx = i;
    }
  }
  highlightPlayingTrack(idx);
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

async function browseFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
    title: "Select library folder",
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("folder");
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

async function finishMixReport(
  report: MixReport,
  out: string,
  deterministic: boolean,
) {
  lastMixReport = report;
  renderMixReport(report, deterministic);
  const autoplay = $<HTMLInputElement>("autoplay")?.checked ?? true;
  const playPath = report.out?.trim() || out;
  await loadMixPlayer(playPath, autoplay);
  updateNowPlayingHighlight();
  appendLog(
    autoplay
      ? `mix complete — playing ${basename(playPath)}`
      : `mix complete — loaded ${basename(playPath)} (autoplay off)`,
  );
}

window.addEventListener("DOMContentLoaded", async () => {
  await listen<{ line: string }>("autodj-progress", (ev) => {
    appendLog(ev.payload.line);
  });

  await refreshBin();

  const outEl = $<HTMLInputElement>("out");
  if (outEl) outEl.value = "/tmp/autodj-mix.wav";

  const audio = $<HTMLAudioElement>("mix-player");
  audio?.addEventListener("timeupdate", () => updateNowPlayingHighlight());
  audio?.addEventListener("seeked", () => updateNowPlayingHighlight());

  $<HTMLButtonElement>("btn-mix-now")?.addEventListener("click", () => {
    mixNowSeek();
  });

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

  $<HTMLButtonElement>("btn-browse-folder")?.addEventListener(
    "click",
    async () => {
      try {
        await browseFolder();
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

  $<HTMLButtonElement>("btn-automix-folder")?.addEventListener(
    "click",
    async () => {
      const folder = $<HTMLInputElement>("folder")?.value.trim() ?? "";
      const out = $<HTMLInputElement>("out")?.value.trim() ?? "";
      const genre = $<HTMLSelectElement>("genre")?.value ?? "house";
      const policyHints =
        $<HTMLInputElement>("policy-hints")?.value.trim() || null;
      const transitionModel =
        $<HTMLInputElement>("transition-model")?.value.trim() || null;
      const db = $<HTMLInputElement>("db")?.value.trim() || null;
      const preserveOrder =
        $<HTMLInputElement>("preserve-order")?.checked ?? false;
      const deterministic =
        $<HTMLInputElement>("deterministic")?.checked ?? true;
      if (!folder || !out) {
        showPlainReport("Set library folder and output WAV paths", true);
        return;
      }
      setBusy(true);
      appendLog("--- automix folder ---");
      try {
        const report = await invoke<MixReport>("automix_folder", {
          args: {
            folder,
            out,
            genre,
            policyHints,
            transitionModel,
            db,
            preserveOrder,
            deterministic,
          },
        });
        await finishMixReport(report, out, deterministic);
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      } finally {
        setBusy(false);
      }
    },
  );

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
    const db = $<HTMLInputElement>("db")?.value.trim() || null;
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
          db,
          preserveOrder,
          playlistName,
          deterministic,
        },
      });
      await finishMixReport(report, out, deterministic);
    } catch (e) {
      showPlainReport(String(e), true);
      appendLog(String(e));
    } finally {
      setBusy(false);
    }
  });
});
