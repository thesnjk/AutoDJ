import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

type TransitionEdge = {
  from: number;
  to: number;
  class: string;
  policy_crossfade_secs?: number;
  crossfade_secs?: number;
  key_from?: string;
  key_to?: string;
  camelot_distance?: number;
  bpm_delta?: number;
  key_on_route?: boolean;
  edge_cost?: number;
};

type CueReport = {
  track: string;
  play_start_secs: number;
  play_end_secs: number;
  crossfade_secs: number;
  bpm: number;
  key?: string;
  bpm_to?: number | null;
  bars_per_phrase: number;
  mix_timeline_start_secs: number;
  mix_timeline_end_secs: number;
};

type PrepTrackReport = {
  track: string;
  sparse_audio: boolean;
  bpm_confidence: number;
  key_confidence: number;
  bpm_octave_corrected: boolean;
  bpm_overridden: boolean;
  key_overridden: boolean;
  beat_offset_overridden: boolean;
  trusted: boolean;
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
  cue_mix_override_applied?: boolean;
  transition_override_applied?: boolean;
  per_track_phrase_applied?: boolean;
  prep_tracks?: PrepTrackReport[];
  transitions?: TransitionEdge[];
  cues?: CueReport[];
  join_timeline_secs?: number[];
};

type SuggestCandidate = {
  path: string;
  cost: number;
  key: string;
  bpm: number;
  class: string;
  on_route: boolean;
};

type SuggestReport = {
  from: string;
  candidates: SuggestCandidate[];
};

/** Last successful MixReport — drives now-playing highlight + Mix Now. */
let lastMixReport: MixReport | null = null;
/** Cue index used for the last matches refresh (avoid redundant suggests). */
let lastMatchesCueIndex = -1;
let matchesInFlight = false;

/** Draft track for Automix Editor (only non-null fields are written to playlist JSON). */
type EditorTrackDraft = {
  path: string;
  mixInSecs: number | null;
  mixOutSecs: number | null;
  transitionOut: string | null;
  key: string | null;
  bpm: number | null;
  /** `4` / `8` / `16`; null = Auto (omit from playlist). */
  barsPerPhrase: number | null;
};

const PHRASE_BAR_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "Auto" },
  { value: "4", label: "4-bar" },
  { value: "8", label: "8-bar" },
  { value: "16", label: "16-bar" },
];

/** Seeded from MixReport.order with computed play windows + edge classes. */
let editorDraft: EditorTrackDraft[] = [];

const TRANSITION_CLASS_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "Auto" },
  { value: "linear_crossfade", label: "Linear crossfade" },
  { value: "eq_swap", label: "EQ swap" },
  { value: "filter_sweep", label: "Filter sweep" },
  { value: "echo_out", label: "Echo out" },
  { value: "tremolo", label: "Tremolo" },
  { value: "riser", label: "Riser" },
  { value: "dissolve", label: "Dissolve" },
  { value: "hard_cut", label: "Hard cut" },
];

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
    "btn-browse-analyze-folder",
    "btn-browse-analyze-file",
    "btn-browse-db",
    "btn-new-db",
    "btn-editor-apply",
    "btn-editor-save",
    "btn-editor-reset",
  ]) {
    const btn = $<HTMLButtonElement>(id);
    if (btn) btn.disabled = busy;
  }
  document
    .querySelectorAll<HTMLButtonElement>(".matches-list button")
    .forEach((btn) => {
      btn.disabled = busy;
    });
  if (!busy) syncEditorButtons();
}

function syncEditorButtons() {
  const hasDraft = editorDraft.length > 0;
  const apply = $<HTMLButtonElement>("btn-editor-apply");
  const saveBtn = $<HTMLButtonElement>("btn-editor-save");
  const reset = $<HTMLButtonElement>("btn-editor-reset");
  if (apply) apply.disabled = !hasDraft;
  if (saveBtn) saveBtn.disabled = !hasDraft;
  if (reset) reset.disabled = !hasDraft;
}

/** CamelCase EditorTrack payload shared by Apply & Save playlist. */
function editorTracksPayload() {
  return editorDraft.map((t) => ({
    path: t.path,
    mixInSecs: t.mixInSecs,
    mixOutSecs: t.mixOutSecs,
    transitionOut: t.transitionOut,
    key: t.key,
    bpm: t.bpm,
    barsPerPhrase: t.barsPerPhrase,
  }));
}

/** Seed editor draft from MixReport cues (play windows) + transition classes. */
function seedEditorDraft(report: MixReport) {
  const order = report.order ?? [];
  const cues = report.cues ?? [];
  const edges = report.transitions ?? [];
  editorDraft = order.map((path, i) => {
    const cue = cues[i];
    const edge = edges[i];
    const mixIn =
      cue && Number.isFinite(cue.play_start_secs) ? cue.play_start_secs : null;
    const mixOut =
      cue && Number.isFinite(cue.play_end_secs) ? cue.play_end_secs : null;
    const transitionOut =
      i < order.length - 1 && edge?.class?.trim() ? edge.class.trim() : null;
    const bars = cue?.bars_per_phrase;
    const barsPerPhrase =
      bars === 4 || bars === 8 || bars === 16 ? bars : null;
    return {
      path,
      mixInSecs: mixIn,
      mixOutSecs: mixOut,
      transitionOut,
      key: cue?.key?.trim() || null,
      bpm: cue?.bpm && Number.isFinite(cue.bpm) ? cue.bpm : null,
      barsPerPhrase,
    };
  });
  syncEditorButtons();
}

function resetEditorDraft() {
  if (!lastMixReport) {
    editorDraft = [];
    syncEditorButtons();
    return;
  }
  seedEditorDraft(lastMixReport);
  renderMixReport(
    lastMixReport,
    $<HTMLInputElement>("deterministic")?.checked ?? true,
  );
  appendLog("Automix Editor: overrides reset");
}

function parseOptionalSecs(raw: string): number | null {
  const t = raw.trim();
  if (!t) return null;
  const n = Number.parseFloat(t);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

function stopRowSeek(ev: Event) {
  ev.stopPropagation();
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

/** Default MixReport sidecar beside WAV (mirrors CLI `{out}.mixreport.json`). */
function mixReportSidecarPath(wavOut: string): string {
  return `${wavOut}.mixreport.json`;
}

async function revealMixReportSidecar(wavOut: string | undefined | null) {
  const out = wavOut?.trim();
  if (!out) {
    showPlainReport("No mix output path for MixReport sidecar", true);
    return;
  }
  const sidecar = mixReportSidecarPath(out);
  const exists = await invoke<boolean>("path_exists", { path: sidecar });
  if (!exists) {
    showPlainReport(`MixReport sidecar missing: ${sidecar}`, true);
    appendLog(`MixReport sidecar missing: ${sidecar}`);
    return;
  }
  try {
    await revealItemInDir(sidecar);
    appendLog(`Revealed MixReport → ${sidecar}`);
  } catch (e) {
    try {
      await openPath(sidecar);
      appendLog(`Opened MixReport → ${sidecar}`);
    } catch (e2) {
      showPlainReport(String(e2 ?? e), true);
      appendLog(String(e2 ?? e));
    }
  }
}

function basename(path: string | undefined): string {
  if (!path) return "?";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function pathsMatch(a: string, b: string): boolean {
  if (a === b) return true;
  const na = a.replace(/\\/g, "/");
  const nb = b.replace(/\\/g, "/");
  if (na === nb) return true;
  return basename(na) === basename(nb);
}

function findPrepForPath(
  prepTracks: PrepTrackReport[] | undefined,
  path: string,
  index: number,
): PrepTrackReport | undefined {
  if (!prepTracks || prepTracks.length === 0) return undefined;
  const byPath = prepTracks.find((p) => pathsMatch(p.track, path));
  if (byPath) return byPath;
  return prepTracks[index];
}

function prepTooltip(prep: PrepTrackReport): string {
  const bits = [
    prep.trusted ? "trusted" : "weak prep",
    `bpm_conf ${fmt(prep.bpm_confidence, 2)}`,
    `key_conf ${fmt(prep.key_confidence, 2)}`,
    prep.sparse_audio ? "sparse" : "dense",
  ];
  if (prep.bpm_overridden) bits.push("BPM override");
  if (prep.key_overridden) bits.push("key override");
  if (prep.beat_offset_overridden) bits.push("beat offset override");
  if (prep.bpm_octave_corrected) bits.push("octave corr");
  return bits.join(" · ");
}

function fmt(n: number | null | undefined, digits = 3): string {
  if (n === null || n === undefined || Number.isNaN(n)) return "n/a";
  return n.toFixed(digits);
}

/** Selected LUFS preset (−14 streaming / −23 broadcast); null when Strict QA locks −14. */
function selectedLufs(deterministic: boolean): number | null {
  if (deterministic) return null;
  const raw = $<HTMLSelectElement>("lufs-target")?.value ?? "-14";
  const n = Number.parseFloat(raw);
  return Number.isFinite(n) ? n : -14;
}

function syncLufsControl() {
  const locked = $<HTMLInputElement>("deterministic")?.checked ?? true;
  const sel = $<HTMLSelectElement>("lufs-target");
  if (!sel) return;
  sel.disabled = locked;
  sel.title = locked
    ? "Strict QA locks loudness to −14 LUFS"
    : "Target integrated loudness for normalize";
}

/** Softmax path vs CLI `--no-transition-model` (checkbox wins). */
function transitionModelInvokeArgs(): {
  transitionModel: string | null;
  noTransitionModel: boolean;
} {
  const noTransitionModel =
    $<HTMLInputElement>("no-transition-model")?.checked ?? false;
  if (noTransitionModel) {
    return { transitionModel: null, noTransitionModel: true };
  }
  return {
    transitionModel:
      $<HTMLInputElement>("transition-model")?.value.trim() || null,
    noTransitionModel: false,
  };
}

function syncTransitionModelControls() {
  const skip = $<HTMLInputElement>("no-transition-model")?.checked ?? false;
  const path = $<HTMLInputElement>("transition-model");
  const browse = $<HTMLButtonElement>("btn-browse-transition-model");
  const load = $<HTMLButtonElement>("btn-load-committed-transition-model");
  if (path) path.disabled = skip;
  if (browse) browse.disabled = skip;
  if (load) load.disabled = skip;
}

/** Short pass criteria from Docs/mix-quality-rubric.md (automated table). */
const GATE_RUBRIC: Record<string, string> = {
  prep: "Trusted when !sparse && bpm_conf ≥ 0.25 (or playlist BPM override). Hard-fail under Strict QA.",
  loudness: "Integrated LUFS within ±2 LU of target after normalize.",
  spectral: "Adjacent hop-RMS ratio ≤ 4.0 (spectral continuity).",
  join: "Worst join-neighborhood hop-RMS ratio ≤ 6.0 (clicks/pops). Hard-fail under Strict QA.",
  bass: "EqSwap concurrent low-band ownership ≤ 0.40. Hard-fail under Strict QA.",
  "beat-phase": "Mean same-tempo join offset < 50 ms. Hard-fail under Strict QA.",
  peak: "True-peak ≤ 0.99 full scale. Hard-fail under Strict QA.",
  stretch: "Worst stretch_clamp_excess == 0 (ratio inside ±15%). Hard-fail under Strict QA.",
  cue: "Phrase/section cues aligned (cue_phrase_aligned); short loops may skip-with-pass.",
  drop: "Drop-aware mix-in/out (cue_drop_aware) from section marks.",
  "tempo-blend": "Multi-BPM edges use bidirectional WSOLA tempo blend when applied.",
  "cue-override": "Playlist mix_in_secs / mix_out_secs pinned absolute cue windows (cue_mix_override_applied).",
  "transition-override": "Playlist transition_out pinned edge class (transition_override_applied).",
  "phrase-applied": "Per-track phrase heuristic resolved bars_per_phrase (per_track_phrase_applied).",
};

const HARD_GATE_NAMES = [
  "prep",
  "loudness",
  "join",
  "bass",
  "beat-phase",
  "peak",
  "stretch",
] as const;

const LISTENING_CHECKLIST = [
  "Transitions feel phrase-aware (listen for awkward early cuts)",
  "Bass conflict avoided during EQ-swap blends (listen for mud)",
  "Energy arc matches genre (house long blend vs DnB snap)",
  "No obvious clicks/pops at join points",
  "Subjective loudness even across the set (pumping / over-limiting)",
  "Large BPM gaps (±15%+) — listen for residual warble",
];

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

function hardGateValue(
  r: MixReport,
  name: (typeof HARD_GATE_NAMES)[number],
): boolean | null | undefined {
  switch (name) {
    case "prep":
      return r.prep_quality_gate_pass;
    case "loudness":
      return r.loudness_gate_pass;
    case "join":
      return r.join_continuity_pass;
    case "bass":
      return r.bass_conflict_gate_pass;
    case "beat-phase":
      return r.beat_phase_gate_pass;
    case "peak":
      return r.peak_gate_pass;
    case "stretch":
      return r.tempo_stretch_gate_pass;
  }
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
    ["cue-override", r.cue_mix_override_applied],
    ["transition-override", r.transition_override_applied],
    ["phrase-applied", r.per_track_phrase_applied],
  ];

  const ok = !deterministic || hardGatesOk(r);
  panel.hidden = false;
  panel.classList.toggle("ok", ok);
  panel.classList.toggle("fail", !ok);

  const summary = document.createElement("div");
  summary.className = "report-summary";
  const summaryText = document.createElement("span");
  summaryText.textContent = [
    `genre: ${r.genre ?? "?"}`,
    `preserve_order: ${String(r.preserve_order ?? false)}`,
    `tracks: ${(r.order ?? []).length}`,
    `cost: ${fmt(r.total_cost, 2)}`,
    `out: ${basename(r.out)}`,
  ].join(" · ");
  summary.appendChild(summaryText);
  const wavOut = r.out?.trim();
  if (wavOut) {
    const revealBtn = document.createElement("button");
    revealBtn.type = "button";
    revealBtn.className = "browse report-reveal";
    revealBtn.textContent = "Reveal MixReport…";
    revealBtn.title = mixReportSidecarPath(wavOut);
    revealBtn.addEventListener("click", () => {
      void revealMixReportSidecar(wavOut);
    });
    summary.appendChild(revealBtn);
  }

  const infoFlags = new Set([
    "cue-override",
    "transition-override",
    "phrase-applied",
  ]);
  const strip = document.createElement("div");
  strip.className = "gate-strip";
  strip.setAttribute("aria-label", "Mix quality gates");
  for (const [name, value] of gates) {
    const pill = document.createElement("span");
    const isInfo = infoFlags.has(name);
    // Info flags: true=pass, false/undefined=na (not a Strict QA failure).
    const klass = isInfo
      ? value === true
        ? "pass"
        : "na"
      : gateClass(value);
    const label = isInfo
      ? value === true
        ? "yes"
        : value === false
          ? "no"
          : "n/a"
      : gateLabel(value);
    pill.className = `gate ${klass}`;
    pill.textContent = `${name}: ${label}`;
    const hint = GATE_RUBRIC[name] ?? "";
    const tip = hint ? `${name}: ${label} — ${hint}` : `${name}: ${label}`;
    pill.title = tip;
    pill.setAttribute("aria-label", tip);
    strip.appendChild(pill);
  }

  let failCallout: HTMLElement | null = null;
  if (deterministic && !hardGatesOk(r)) {
    const failed = HARD_GATE_NAMES.filter(
      (name) => hardGateValue(r, name) === false,
    );
    failCallout = document.createElement("p");
    failCallout.className = "gate-fail-callout";
    failCallout.textContent =
      failed.length > 0
        ? `Strict QA failed: ${failed.join(", ")}. Hover a gate pill for the pass criterion.`
        : "Strict QA failed. Hover a gate pill for the pass criterion.";
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

  const listening = document.createElement("details");
  listening.className = "listening-rubric";
  const listeningSummary = document.createElement("summary");
  listeningSummary.textContent = "Listen while playing";
  listening.appendChild(listeningSummary);
  const listeningHint = document.createElement("p");
  listeningHint.className = "hint";
  listeningHint.textContent =
    "Human checklist from the mix-quality rubric (validate on real libraries; gold covers automated gates).";
  listening.appendChild(listeningHint);
  const listeningList = document.createElement("ul");
  for (const item of LISTENING_CHECKLIST) {
    const li = document.createElement("li");
    li.textContent = item;
    listeningList.appendChild(li);
  }
  listening.appendChild(listeningList);

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
  const prepTracks = r.prep_tracks ?? [];
  if (tracks.length === 0) {
    const li = document.createElement("li");
    li.textContent = "(none)";
    orderUl.appendChild(li);
  } else {
    tracks.forEach((path, i) => {
      const li = document.createElement("li");
      li.dataset.index = String(i);
      const cue = cues[i];
      const prep = findPrepForPath(prepTracks, path, i);
      const titleBits = [path];
      if (cue) {
        li.dataset.start = String(cue.mix_timeline_start_secs);
        li.dataset.end = String(cue.mix_timeline_end_secs);
        titleBits.push(
          `${fmt(cue.mix_timeline_start_secs, 1)}s–${fmt(cue.mix_timeline_end_secs, 1)}s`,
        );
      }
      if (prep) titleBits.push(prepTooltip(prep));
      li.title = titleBits.join(" · ");
      const name = document.createElement("span");
      name.className = "set-order-name";
      name.textContent = basename(path);
      li.appendChild(name);
      if (cue) {
        const meta = document.createElement("span");
        meta.className = "set-order-meta";
        const key = cue.key?.trim() || "?";
        const bars =
          cue.bars_per_phrase && cue.bars_per_phrase > 0
            ? ` · ${cue.bars_per_phrase}-bar`
            : "";
        meta.textContent = `${key} · ${fmt(cue.bpm, 1)} BPM${bars}`;
        li.appendChild(meta);
      }
      if (prep) {
        const pill = document.createElement("span");
        pill.className = `gate set-order-prep ${prep.trusted ? "pass" : "fail"}`;
        pill.textContent = prep.trusted ? "trusted" : "weak";
        pill.title = prepTooltip(prep);
        li.appendChild(pill);
      }
      if (editorDraft[i]) {
        const edits = document.createElement("span");
        edits.className = "editor-cue-fields";
        edits.addEventListener("click", stopRowSeek);
        edits.addEventListener("keydown", stopRowSeek);
        const inLab = document.createElement("label");
        inLab.className = "editor-field";
        inLab.textContent = "in";
        const inInput = document.createElement("input");
        inInput.type = "number";
        inInput.step = "0.01";
        inInput.min = "0";
        inInput.placeholder = "auto";
        inInput.title = "mix_in_secs override (empty = auto)";
        inInput.value =
          editorDraft[i].mixInSecs === null
            ? ""
            : String(editorDraft[i].mixInSecs);
        inInput.addEventListener("change", () => {
          editorDraft[i].mixInSecs = parseOptionalSecs(inInput.value);
        });
        inLab.appendChild(inInput);
        const outLab = document.createElement("label");
        outLab.className = "editor-field";
        outLab.textContent = "out";
        const outInput = document.createElement("input");
        outInput.type = "number";
        outInput.step = "0.01";
        outInput.min = "0";
        outInput.placeholder = "auto";
        outInput.title = "mix_out_secs override (empty = auto)";
        outInput.value =
          editorDraft[i].mixOutSecs === null
            ? ""
            : String(editorDraft[i].mixOutSecs);
        outInput.addEventListener("change", () => {
          editorDraft[i].mixOutSecs = parseOptionalSecs(outInput.value);
        });
        outLab.appendChild(outInput);
        const phraseLab = document.createElement("label");
        phraseLab.className = "editor-field";
        phraseLab.textContent = "phrase";
        const phraseSel = document.createElement("select");
        phraseSel.className = "editor-phrase";
        phraseSel.title = "bars_per_phrase override (Auto = heuristic)";
        for (const opt of PHRASE_BAR_OPTIONS) {
          const o = document.createElement("option");
          o.value = opt.value;
          o.textContent = opt.label;
          phraseSel.appendChild(o);
        }
        phraseSel.value =
          editorDraft[i].barsPerPhrase === null
            ? ""
            : String(editorDraft[i].barsPerPhrase);
        phraseSel.addEventListener("change", () => {
          const v = phraseSel.value.trim();
          editorDraft[i].barsPerPhrase = v ? Number.parseInt(v, 10) : null;
        });
        phraseLab.appendChild(phraseSel);
        edits.appendChild(inLab);
        edits.appendChild(outLab);
        edits.appendChild(phraseLab);
        li.appendChild(edits);
      }
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
  ul.id = "transitions-list";
  const edges = r.transitions ?? [];
  if (edges.length === 0) {
    const li = document.createElement("li");
    li.textContent = "(none)";
    ul.appendChild(li);
  } else {
    edges.forEach((e, i) => {
      const li = document.createElement("li");
      li.dataset.index = String(i);
      const xfade = fmt(e.crossfade_secs, 2);
      const policy = fmt(e.policy_crossfade_secs, 2);
      const keyFrom = e.key_from ?? "?";
      const keyTo = e.key_to ?? "?";
      const dist =
        e.camelot_distance === undefined ? "?" : String(e.camelot_distance);
      const bpmDelta = fmt(e.bpm_delta, 1);
      const cost = fmt(e.edge_cost, 2);
      const line = document.createElement("span");
      line.className = "transitions-line";
      line.textContent = `${e.from}→${e.to}: ${e.class} · ${keyFrom}→${keyTo} (d${dist}) · ΔBPM ${bpmDelta} · cost ${cost} · xfade ${xfade}s (policy ${policy}s)`;
      li.appendChild(line);
      if (e.key_on_route) {
        const badge = document.createElement("span");
        badge.className = "matches-route";
        badge.textContent = "key route";
        li.appendChild(badge);
      }
      const fromIdx = e.from;
      if (
        Number.isInteger(fromIdx) &&
        fromIdx >= 0 &&
        fromIdx < editorDraft.length
      ) {
        const sel = document.createElement("select");
        sel.className = "editor-transition";
        sel.title = "transition_out override on outgoing track (empty = auto)";
        for (const opt of TRANSITION_CLASS_OPTIONS) {
          const o = document.createElement("option");
          o.value = opt.value;
          o.textContent = opt.label;
          sel.appendChild(o);
        }
        sel.value = editorDraft[fromIdx].transitionOut ?? "";
        sel.addEventListener("click", stopRowSeek);
        sel.addEventListener("mousedown", stopRowSeek);
        sel.addEventListener("change", (ev) => {
          ev.stopPropagation();
          const v = sel.value.trim();
          editorDraft[fromIdx].transitionOut = v || null;
        });
        li.appendChild(sel);
      }
      li.tabIndex = 0;
      li.setAttribute("role", "button");
      li.title = `Seek to join ${fmt(r.join_timeline_secs?.[i], 2)}s`;
      li.addEventListener("click", () => seekToJoin(i));
      li.addEventListener("keydown", (ev) => {
        if (ev.key === "Enter" || ev.key === " ") {
          ev.preventDefault();
          seekToJoin(i);
        }
      });
      ul.appendChild(li);
    });
  }
  transitionsWrap.appendChild(ul);

  const kids: HTMLElement[] = [summary, strip];
  if (failCallout) kids.push(failCallout);
  kids.push(metrics, listening, orderWrap, transitionsWrap);
  panel.replaceChildren(...kids);
}

/** Load the rendered mix into the HTML5 player; autoplay when requested. */
async function loadMixPlayer(outPath: string, autoplay: boolean) {
  const audio = $<HTMLAudioElement>("mix-player");
  if (!audio || !outPath) return;
  audio.src = convertFileSrc(outPath);
  audio.load();
  renderMixTimeline();
  if (!autoplay) return;
  try {
    await audio.play();
  } catch (e) {
    appendLog(`autoplay blocked or failed: ${e}`);
  }
}

function transitionTooltip(edge: TransitionEdge | undefined, joinSecs: number): string {
  if (!edge) return `Join at ${fmt(joinSecs, 2)}s`;
  const keyFrom = edge.key_from ?? "?";
  const keyTo = edge.key_to ?? "?";
  const bpmDelta = fmt(edge.bpm_delta, 1);
  return `${edge.class} · ${keyFrom}→${keyTo} · ΔBPM ${bpmDelta} · ${fmt(joinSecs, 2)}s`;
}

const LISTEN_HINT_WINDOW_SECS = 4;

/** Rebuild cue segments + join ticks from the last MixReport. */
function renderMixTimeline() {
  renderCueSegments();
  renderJoinMarkers();
  updateNowPlayingHighlight();
}

/** Draw per-cue spans from cues[].mix_timeline_* under the player. */
function renderCueSegments() {
  const rail = $<HTMLDivElement>("cue-segments");
  const audio = $<HTMLAudioElement>("mix-player");
  if (!rail) return;
  rail.replaceChildren();
  const cues = lastMixReport?.cues ?? [];
  const duration = audio?.duration ?? 0;
  if (cues.length === 0 || !Number.isFinite(duration) || duration <= 0) {
    rail.setAttribute("aria-hidden", "true");
    return;
  }
  rail.removeAttribute("aria-hidden");
  const order = lastMixReport?.order ?? [];
  cues.forEach((cue, i) => {
    const start = Math.max(0, cue.mix_timeline_start_secs);
    const end = Math.max(start, cue.mix_timeline_end_secs);
    const left = (start / duration) * 100;
    const width = Math.max(0.4, ((end - start) / duration) * 100);
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "cue-segment";
    btn.dataset.index = String(i);
    btn.style.left = `${left}%`;
    btn.style.width = `${width}%`;
    const label = basename(order[i] ?? cue.track ?? `track ${i + 1}`);
    btn.title = `${label} · ${fmt(start, 1)}s–${fmt(end, 1)}s`;
    btn.setAttribute(
      "aria-label",
      `Seek to ${label} at ${fmt(start, 2)} seconds`,
    );
    btn.addEventListener("click", () => seekToTrack(i));
    rail.appendChild(btn);
  });
}

/** Place join ticks under the player from MixReport.join_timeline_secs. */
function renderJoinMarkers() {
  const rail = $<HTMLDivElement>("join-markers");
  const audio = $<HTMLAudioElement>("mix-player");
  if (!rail) return;
  rail.replaceChildren();
  const joins = lastMixReport?.join_timeline_secs ?? [];
  const duration = audio?.duration ?? 0;
  if (joins.length === 0 || !Number.isFinite(duration) || duration <= 0) {
    rail.setAttribute("aria-hidden", "true");
    return;
  }
  rail.removeAttribute("aria-hidden");
  const edges = lastMixReport?.transitions ?? [];
  joins.forEach((t, i) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "join-marker";
    btn.dataset.index = String(i);
    btn.style.left = `${(t / duration) * 100}%`;
    btn.title = transitionTooltip(edges[i], t);
    btn.setAttribute("aria-label", `Seek to join ${i + 1} at ${fmt(t, 2)} seconds`);
    btn.addEventListener("click", () => seekToJoin(i));
    rail.appendChild(btn);
  });
}

/** First join strictly after the playhead (Mix Now target). */
function nextJoinIndex(t: number): number {
  const joins = lastMixReport?.join_timeline_secs ?? [];
  for (let i = 0; i < joins.length; i++) {
    if (joins[i] > t + 0.05) return i;
  }
  return -1;
}

/** Nearest join within ±LISTEN_HINT_WINDOW_SECS, or -1. */
function activeJoinIndex(t: number): number {
  const joins = lastMixReport?.join_timeline_secs ?? [];
  let best = -1;
  let bestDist = LISTEN_HINT_WINDOW_SECS + 1;
  for (let i = 0; i < joins.length; i++) {
    const d = Math.abs(joins[i] - t);
    if (d <= LISTEN_HINT_WINDOW_SECS && d < bestDist) {
      best = i;
      bestDist = d;
    }
  }
  return best;
}

/** One-liner for listening near a join (class + MixReport metric). */
function listenHintText(joinIndex: number): string {
  const r = lastMixReport;
  if (!r) return "";
  const edge = r.transitions?.[joinIndex];
  if (!edge) return `Join ${joinIndex + 1}`;
  const cls = edge.class ?? "?";
  if (cls === "EqSwap" || cls === "eq_swap") {
    return `${cls} · bass overlap ${fmt(r.max_bass_overlap, 2)} · gate ${r.bass_conflict_gate_pass ? "pass" : "fail"}`;
  }
  if (cls === "HardCut" || cls === "hard_cut") {
    return `${cls} · join flux ${fmt(r.max_join_flux_ratio, 2)} · gate ${r.join_continuity_pass ? "pass" : "fail"}`;
  }
  return `${cls} · ΔBPM ${fmt(edge.bpm_delta, 1)} · cost ${fmt(edge.edge_cost, 2)}`;
}

function updateListenHint(joinIndex: number) {
  const el = $<HTMLParagraphElement>("listen-hint");
  if (!el) return;
  if (joinIndex < 0) {
    el.hidden = true;
    el.textContent = "";
    return;
  }
  el.textContent = listenHintText(joinIndex);
  el.hidden = false;
}

function seekToJoin(index: number) {
  const audio = $<HTMLAudioElement>("mix-player");
  const joins = lastMixReport?.join_timeline_secs ?? [];
  if (!audio || index < 0 || index >= joins.length) return;
  const t = joins[index];
  audio.currentTime = Math.max(0, t);
  void audio.play().catch(() => undefined);
  appendLog(`join ${index + 1} → ${fmt(t, 2)}s`);
  updateNowPlayingHighlight();
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

/** Seek to the previous join before the playhead (Shift+N). */
function mixNowSeekPrev() {
  const audio = $<HTMLAudioElement>("mix-player");
  const joins = lastMixReport?.join_timeline_secs ?? [];
  if (!audio || joins.length === 0) {
    appendLog("Mix Now: no joins in last mix");
    return;
  }
  const t = audio.currentTime;
  let prev: number | undefined;
  for (const j of joins) {
    if (j < t - 0.05) prev = j;
    else break;
  }
  if (prev === undefined) {
    appendLog("Mix Now: already before first transition");
    return;
  }
  audio.currentTime = prev;
  void audio.play().catch(() => undefined);
  appendLog(`Mix Now ← ${fmt(prev, 2)}s`);
  updateNowPlayingHighlight();
}

function isTypingTarget(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  const tag = el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (el.isContentEditable) return true;
  return Boolean(el.closest("input, textarea, select, [contenteditable='true']"));
}

function highlightPlayingTrack(index: number) {
  const list = $<HTMLOListElement>("set-order-list");
  if (!list) return;
  list.querySelectorAll("li").forEach((li, i) => {
    li.classList.toggle("playing", i === index);
  });
  const segments = $<HTMLDivElement>("cue-segments");
  segments?.querySelectorAll(".cue-segment").forEach((el) => {
    const i = Number((el as HTMLElement).dataset.index ?? -1);
    el.classList.toggle("active", i === index);
  });
}

function highlightJoinChrome(activeIdx: number, nextIdx: number) {
  const markers = $<HTMLDivElement>("join-markers");
  markers?.querySelectorAll(".join-marker").forEach((el) => {
    const i = Number((el as HTMLElement).dataset.index ?? -1);
    el.classList.toggle("active", i === activeIdx && activeIdx >= 0);
    el.classList.toggle("next", i === nextIdx && nextIdx >= 0);
  });
  const list = $<HTMLUListElement>("transitions-list");
  list?.querySelectorAll("li[data-index]").forEach((el) => {
    const i = Number((el as HTMLElement).dataset.index ?? -1);
    el.classList.toggle("active", i === activeIdx && activeIdx >= 0);
    el.classList.toggle("next", i === nextIdx && nextIdx >= 0);
  });
}

function currentCueIndex(): number {
  const audio = $<HTMLAudioElement>("mix-player");
  const cues = lastMixReport?.cues;
  if (!audio || !cues || cues.length === 0) return 0;
  const t = audio.currentTime;
  let idx = 0;
  for (let i = 0; i < cues.length; i++) {
    if (t + 1e-3 >= cues[i].mix_timeline_start_secs) {
      idx = i;
    }
  }
  return idx;
}

function updateNowPlayingHighlight() {
  const audio = $<HTMLAudioElement>("mix-player");
  const cues = lastMixReport?.cues;
  if (!audio || !cues || cues.length === 0) {
    highlightJoinChrome(-1, -1);
    updateListenHint(-1);
    return;
  }
  const t = audio.currentTime;
  const idx = currentCueIndex();
  highlightPlayingTrack(idx);
  const activeIdx = activeJoinIndex(t);
  const nextIdx = nextJoinIndex(t);
  highlightJoinChrome(activeIdx, nextIdx);
  updateListenHint(activeIdx);
  if (idx !== lastMatchesCueIndex) {
    void refreshMatchesForCue(idx);
  }
}

function clearMatchesPanel() {
  const panel = $<HTMLDivElement>("matches-panel");
  const list = $<HTMLUListElement>("matches-list");
  if (list) list.replaceChildren();
  if (panel) panel.hidden = true;
  lastMatchesCueIndex = -1;
}

function renderMatches(candidates: SuggestCandidate[]) {
  const panel = $<HTMLDivElement>("matches-panel");
  const list = $<HTMLUListElement>("matches-list");
  if (!panel || !list) return;
  list.replaceChildren();
  if (candidates.length === 0) {
    panel.hidden = true;
    return;
  }
  for (const c of candidates) {
    const li = document.createElement("li");
    const meta = document.createElement("span");
    meta.className = "matches-meta";
    meta.textContent = `${basename(c.path)} · ${c.key} · ${fmt(c.bpm, 1)} BPM · cost ${fmt(c.cost, 2)} · ${c.class}`;
    li.appendChild(meta);
    if (c.on_route) {
      const badge = document.createElement("span");
      badge.className = "matches-route";
      badge.textContent = "key route";
      li.appendChild(badge);
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = "Queue & remix";
    btn.addEventListener("click", () => {
      void queueRemix(c.path);
    });
    li.appendChild(btn);
    list.appendChild(li);
  }
  panel.hidden = false;
}

async function refreshMatchesForCue(cueIndex: number) {
  const folder = $<HTMLInputElement>("folder")?.value.trim() ?? "";
  const order = lastMixReport?.order ?? [];
  if (!folder || order.length < 2 || cueIndex < 0 || cueIndex >= order.length) {
    clearMatchesPanel();
    return;
  }
  if (matchesInFlight) return;
  matchesInFlight = true;
  lastMatchesCueIndex = cueIndex;
  const from = order[cueIndex];
  const genre = $<HTMLSelectElement>("genre")?.value ?? "house";
  const policyHints =
    $<HTMLInputElement>("policy-hints")?.value.trim() || null;
  const { transitionModel, noTransitionModel } = transitionModelInvokeArgs();
  const db = $<HTMLInputElement>("db")?.value.trim() || null;
  try {
    const report = await invoke<SuggestReport>("suggest_matches", {
      args: {
        folder,
        from,
        top: 5,
        genre,
        policyHints,
        transitionModel,
        noTransitionModel,
        db,
        position: cueIndex,
        setLen: order.length,
      },
    });
    // Drop stale responses if the cue moved while we were fetching.
    if (lastMatchesCueIndex !== cueIndex) return;
    renderMatches(report.candidates ?? []);
  } catch (e) {
    appendLog(`matches: ${e}`);
    clearMatchesPanel();
  } finally {
    matchesInFlight = false;
  }
}

async function queueRemix(nextPath: string) {
  const folder = $<HTMLInputElement>("folder")?.value.trim() ?? "";
  const out = $<HTMLInputElement>("out")?.value.trim() ?? "";
  const order = lastMixReport?.order ?? [];
  const cueIndex = currentCueIndex();
  if (!folder || !out || order.length === 0) {
    showPlainReport("Need a folder Automix result before Queue & remix", true);
    return;
  }
  const prefix = order.slice(0, cueIndex + 1);
  if (prefix.some((p) => p === nextPath)) {
    showPlainReport("Pick a track that is not already in the played prefix", true);
    return;
  }
  const genre = $<HTMLSelectElement>("genre")?.value ?? "house";
  const policyHints =
    $<HTMLInputElement>("policy-hints")?.value.trim() || null;
  const { transitionModel, noTransitionModel } = transitionModelInvokeArgs();
  const db = $<HTMLInputElement>("db")?.value.trim() || null;
  const deterministic =
    $<HTMLInputElement>("deterministic")?.checked ?? true;
  const lufs = selectedLufs(deterministic);
  setBusy(true);
  appendLog(`--- queue remix → ${basename(nextPath)} ---`);
  try {
    const report = await invoke<MixReport>("remix_queue_next", {
      args: {
        folder,
        prefix,
        next: nextPath,
        out,
        genre,
        policyHints,
        transitionModel,
        noTransitionModel,
        db,
        deterministic,
        lufs,
      },
    });
    lastMatchesCueIndex = -1;
    await finishMixReport(report, out, deterministic);
  } catch (e) {
    showPlainReport(String(e), true);
    appendLog(String(e));
  } finally {
    setBusy(false);
  }
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

async function browseAnalyzeFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
    title: "Select library to analyze",
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("analyze-input");
    if (el) el.value = selected;
  }
}

async function browseAnalyzeFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Select audio file to analyze",
    filters: [{ name: "Audio", extensions: ["wav", "flac"] }],
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("analyze-input");
    if (el) el.value = selected;
  }
}

async function browseOut() {
  const selected = await save({
    title: "Export mix audio",
    filters: [{ name: "Audio", extensions: ["wav", "flac"] }],
    defaultPath: "mix.wav",
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("out");
    if (el) el.value = selected;
  }
}

async function browseDbOpen() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Select feature database",
    filters: [{ name: "SQLite", extensions: ["sqlite"] }],
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("db");
    if (el) el.value = selected;
  }
}

async function browseDbNew() {
  const current = $<HTMLInputElement>("db")?.value.trim() || "features.sqlite";
  const selected = await save({
    title: "Create feature database",
    filters: [{ name: "SQLite", extensions: ["sqlite"] }],
    defaultPath: current,
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("db");
    if (el) el.value = selected;
  }
}

async function browsePolicyHints() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Select policy hints JSON",
    filters: [{ name: "Policy hints", extensions: ["json"] }],
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("policy-hints");
    if (el) el.value = selected;
  }
}

async function loadDemoPolicyHints() {
  const path = await invoke<string>("resolve_demo_policy_hints");
  const el = $<HTMLInputElement>("policy-hints");
  if (el) el.value = path;
  appendLog(`policy hints: loaded demo ${basename(path)}`);
}

async function browseTransitionModel() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Select transition model JSON",
    filters: [{ name: "Transition model", extensions: ["json"] }],
  });
  if (typeof selected === "string") {
    const el = $<HTMLInputElement>("transition-model");
    if (el) el.value = selected;
  }
}

async function loadCommittedTransitionModel() {
  const path = await invoke<string>("resolve_committed_transition_model");
  const el = $<HTMLInputElement>("transition-model");
  if (el) el.value = path;
  appendLog(`transition model: loaded committed ${basename(path)}`);
}

async function finishMixReport(
  report: MixReport,
  out: string,
  deterministic: boolean,
) {
  lastMixReport = report;
  lastMatchesCueIndex = -1;
  seedEditorDraft(report);
  renderMixReport(report, deterministic);
  const autoplay = $<HTMLInputElement>("autoplay")?.checked ?? true;
  const playPath = report.out?.trim() || out;
  await loadMixPlayer(playPath, autoplay);
  renderMixTimeline();
  appendLog(
    autoplay
      ? `mix complete — playing ${basename(playPath)}`
      : `mix complete — loaded ${basename(playPath)} (autoplay off)`,
  );
  const sidecar = mixReportSidecarPath(playPath);
  try {
    const exists = await invoke<boolean>("path_exists", { path: sidecar });
    if (exists) {
      appendLog(`MixReport sidecar → ${sidecar}`);
    } else {
      appendLog(`MixReport sidecar missing (expected ${sidecar})`);
    }
  } catch (e) {
    appendLog(`MixReport sidecar check failed: ${String(e)}`);
  }
}

/** Apply Automix Editor overrides via mix --preserve-order. */
async function applyEditorRemix() {
  const out = $<HTMLInputElement>("out")?.value.trim() ?? "";
  if (!out || editorDraft.length === 0) {
    showPlainReport("Need a completed mix before Apply & re-mix", true);
    return;
  }
  const genre = $<HTMLSelectElement>("genre")?.value ?? "house";
  const policyHints =
    $<HTMLInputElement>("policy-hints")?.value.trim() || null;
  const { transitionModel, noTransitionModel } = transitionModelInvokeArgs();
  const db = $<HTMLInputElement>("db")?.value.trim() || null;
  const deterministic =
    $<HTMLInputElement>("deterministic")?.checked ?? true;
  const lufs = selectedLufs(deterministic);
  const tracks = editorTracksPayload();
  setBusy(true);
  appendLog("--- Automix Editor Apply & re-mix ---");
  try {
    const report = await invoke<MixReport>("mix_editor_remix", {
      args: {
        out,
        genre,
        tracks,
        policyHints,
        transitionModel,
        noTransitionModel,
        db,
        deterministic,
        lufs,
      },
    });
    lastMatchesCueIndex = -1;
    await finishMixReport(report, out, deterministic);
    const cuePin = report.cue_mix_override_applied ? "cue-override" : "";
    const trPin = report.transition_override_applied
      ? "transition-override"
      : "";
    appendLog(
      `editor remix flags: ${[cuePin, trPin].filter(Boolean).join(", ") || "none"}`,
    );
  } catch (e) {
    showPlainReport(String(e), true);
    appendLog(String(e));
  } finally {
    setBusy(false);
  }
}

/** Save Automix Editor pins as CLI-compatible playlist JSON. */
async function saveEditorPlaylist() {
  if (editorDraft.length === 0) {
    showPlainReport("Need a completed mix before Save playlist", true);
    return;
  }
  const selected = await save({
    title: "Save playlist JSON",
    filters: [{ name: "Playlist", extensions: ["json"] }],
    defaultPath: "playlist.json",
  });
  if (typeof selected !== "string" || !selected.trim()) {
    return;
  }
  const genre = $<HTMLSelectElement>("genre")?.value ?? "house";
  setBusy(true);
  appendLog("--- Automix Editor Save playlist ---");
  try {
    await invoke("export_editor_playlist", {
      args: {
        dest: selected,
        genre,
        tracks: editorTracksPayload(),
      },
    });
    appendLog(`Saved playlist → ${selected}`);
    const playlistEl = $<HTMLInputElement>("playlist");
    if (playlistEl) playlistEl.value = selected;
    const preserveEl = $<HTMLInputElement>("preserve-order");
    if (preserveEl) preserveEl.checked = true;
    const advanced = document.querySelector("details.advanced");
    if (advanced instanceof HTMLDetailsElement) advanced.open = true;
    appendLog(
      `Advanced playlist → ${selected} (preserve order on; Mix playlist ready)`,
    );
    showPlainReport(
      `Saved playlist → ${basename(selected)} · wired Advanced + preserve order`,
    );
  } catch (e) {
    showPlainReport(String(e), true);
    appendLog(String(e));
  } finally {
    setBusy(false);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  await listen<{ line: string }>("autodj-progress", (ev) => {
    appendLog(ev.payload.line);
  });

  await refreshBin();

  const outEl = $<HTMLInputElement>("out");
  if (outEl) outEl.value = "/tmp/autodj-mix.wav";

  syncLufsControl();
  syncTransitionModelControls();
  $<HTMLInputElement>("deterministic")?.addEventListener("change", () => {
    syncLufsControl();
  });
  $<HTMLInputElement>("no-transition-model")?.addEventListener("change", () => {
    syncTransitionModelControls();
  });

  const audio = $<HTMLAudioElement>("mix-player");
  audio?.addEventListener("timeupdate", () => updateNowPlayingHighlight());
  audio?.addEventListener("seeked", () => updateNowPlayingHighlight());
  audio?.addEventListener("loadedmetadata", () => renderMixTimeline());
  audio?.addEventListener("durationchange", () => renderMixTimeline());

  $<HTMLButtonElement>("btn-mix-now")?.addEventListener("click", () => {
    mixNowSeek();
  });

  $<HTMLButtonElement>("btn-editor-apply")?.addEventListener("click", () => {
    void applyEditorRemix();
  });
  $<HTMLButtonElement>("btn-editor-save")?.addEventListener("click", () => {
    void saveEditorPlaylist().catch((e) => {
      showPlainReport(String(e), true);
      appendLog(String(e));
    });
  });
  $<HTMLButtonElement>("btn-editor-reset")?.addEventListener("click", () => {
    resetEditorDraft();
  });
  syncEditorButtons();

  window.addEventListener("keydown", (ev) => {
    if (ev.metaKey || ev.ctrlKey || ev.altKey) return;
    if (isTypingTarget(ev.target)) return;
    if (ev.key !== "n" && ev.key !== "N") return;
    ev.preventDefault();
    if (ev.shiftKey) mixNowSeekPrev();
    else mixNowSeek();
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

  $<HTMLButtonElement>("btn-browse-analyze-folder")?.addEventListener(
    "click",
    async () => {
      try {
        await browseAnalyzeFolder();
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-browse-analyze-file")?.addEventListener(
    "click",
    async () => {
      try {
        await browseAnalyzeFile();
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-browse-db")?.addEventListener("click", async () => {
    try {
      await browseDbOpen();
    } catch (e) {
      showPlainReport(String(e), true);
      appendLog(String(e));
    }
  });

  $<HTMLButtonElement>("btn-new-db")?.addEventListener("click", async () => {
    try {
      await browseDbNew();
    } catch (e) {
      showPlainReport(String(e), true);
      appendLog(String(e));
    }
  });

  $<HTMLButtonElement>("btn-browse-policy-hints")?.addEventListener(
    "click",
    async () => {
      try {
        await browsePolicyHints();
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-load-demo-policy-hints")?.addEventListener(
    "click",
    async () => {
      try {
        await loadDemoPolicyHints();
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-browse-transition-model")?.addEventListener(
    "click",
    async () => {
      try {
        await browseTransitionModel();
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

  $<HTMLButtonElement>("btn-load-committed-transition-model")?.addEventListener(
    "click",
    async () => {
      try {
        await loadCommittedTransitionModel();
      } catch (e) {
        showPlainReport(String(e), true);
        appendLog(String(e));
      }
    },
  );

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
      const { transitionModel, noTransitionModel } = transitionModelInvokeArgs();
      const db = $<HTMLInputElement>("db")?.value.trim() || null;
      const preserveOrder =
        $<HTMLInputElement>("preserve-order")?.checked ?? false;
      const deterministic =
        $<HTMLInputElement>("deterministic")?.checked ?? true;
      const lufs = selectedLufs(deterministic);
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
            noTransitionModel,
            db,
            preserveOrder,
            deterministic,
            lufs,
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
    const { transitionModel, noTransitionModel } = transitionModelInvokeArgs();
    const db = $<HTMLInputElement>("db")?.value.trim() || null;
    const preserveOrder =
      $<HTMLInputElement>("preserve-order")?.checked ?? false;
    const playlistName =
      $<HTMLInputElement>("playlist-name")?.value.trim() || null;
    const deterministic =
      $<HTMLInputElement>("deterministic")?.checked ?? true;
    const lufs = selectedLufs(deterministic);
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
          noTransitionModel,
          db,
          preserveOrder,
          playlistName,
          deterministic,
          lufs,
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
