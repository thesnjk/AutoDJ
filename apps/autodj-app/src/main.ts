import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

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
  document
    .querySelectorAll<HTMLButtonElement>(".matches-list button")
    .forEach((btn) => {
      btn.disabled = busy;
    });
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
  summary.textContent = [
    `genre: ${r.genre ?? "?"}`,
    `preserve_order: ${String(r.preserve_order ?? false)}`,
    `tracks: ${(r.order ?? []).length}`,
    `cost: ${fmt(r.total_cost, 2)}`,
    `out: ${basename(r.out)}`,
  ].join(" · ");

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
    edges.forEach((e, i) => {
      const li = document.createElement("li");
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
  renderJoinMarkers();
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
    btn.style.left = `${(t / duration) * 100}%`;
    btn.title = transitionTooltip(edges[i], t);
    btn.setAttribute("aria-label", `Seek to join ${i + 1} at ${fmt(t, 2)} seconds`);
    btn.addEventListener("click", () => seekToJoin(i));
    rail.appendChild(btn);
  });
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
  const cues = lastMixReport?.cues;
  if (!cues || cues.length === 0) return;
  const idx = currentCueIndex();
  highlightPlayingTrack(idx);
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
  const transitionModel =
    $<HTMLInputElement>("transition-model")?.value.trim() || null;
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
  const transitionModel =
    $<HTMLInputElement>("transition-model")?.value.trim() || null;
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
  lastMatchesCueIndex = -1;
  renderMixReport(report, deterministic);
  const autoplay = $<HTMLInputElement>("autoplay")?.checked ?? true;
  const playPath = report.out?.trim() || out;
  await loadMixPlayer(playPath, autoplay);
  renderJoinMarkers();
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

  syncLufsControl();
  $<HTMLInputElement>("deterministic")?.addEventListener("change", () => {
    syncLufsControl();
  });

  const audio = $<HTMLAudioElement>("mix-player");
  audio?.addEventListener("timeupdate", () => updateNowPlayingHighlight());
  audio?.addEventListener("seeked", () => updateNowPlayingHighlight());
  audio?.addEventListener("loadedmetadata", () => renderJoinMarkers());
  audio?.addEventListener("durationchange", () => renderJoinMarkers());

  $<HTMLButtonElement>("btn-mix-now")?.addEventListener("click", () => {
    mixNowSeek();
  });

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
    const transitionModel =
      $<HTMLInputElement>("transition-model")?.value.trim() || null;
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
