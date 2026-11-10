//! Phrase-aware mix-in / mix-out cue planning.
//!
//! Offline planner — allocates freely; do not call from a real-time audio thread.

use crate::analysis::{SectionMarks, TrackFeatures};

/// True when section marks are finite and ordered (`intro_end ≤ drop_start ≤ outro_start`).
pub fn sections_drop_ordered(sections: &SectionMarks) -> bool {
    sections.intro_end.is_finite()
        && sections.drop_start.is_finite()
        && sections.outro_start.is_finite()
        && sections.intro_end <= sections.drop_start
        && sections.drop_start <= sections.outro_start
}

/// Planned play window and outgoing crossfade for one mix segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CuePoints {
    /// Absolute start time within the source track (seconds).
    pub play_start_secs: f64,
    /// Absolute end time within the source track (seconds).
    pub play_end_secs: f64,
    /// Crossfade into the next track (seconds). Ignored for the last segment.
    pub crossfade_secs: f32,
}

/// Duration of one 4/4 bar at `bpm` (four quarter notes).
pub fn bar_secs(bpm: f32) -> f64 {
    let bpm = bpm.max(1.0) as f64;
    4.0 * 60.0 / bpm
}

/// Duration of one beat (quarter note) at `bpm`.
pub fn beat_secs(bpm: f32) -> f64 {
    let bpm = bpm.max(1.0) as f64;
    60.0 / bpm
}

/// Snap `t` to the nearest phrase boundary (multiples of `bars_per_phrase` bars).
pub fn snap_to_phrase(t: f64, bpm: f32, bars_per_phrase: u32) -> f64 {
    snap_to_phrase_grid(t, bpm, bars_per_phrase, 0.0)
}

/// Snap `t` to the nearest phrase boundary relative to `beat_offset_secs`.
pub fn snap_to_phrase_grid(t: f64, bpm: f32, bars_per_phrase: u32, beat_offset_secs: f64) -> f64 {
    let phrase = bar_secs(bpm) * bars_per_phrase.max(1) as f64;
    if phrase <= 0.0 || !t.is_finite() {
        return t.max(0.0);
    }
    let off = if beat_offset_secs.is_finite() {
        beat_offset_secs.rem_euclid(phrase)
    } else {
        0.0
    };
    let rel = t - off;
    let n = (rel / phrase).round();
    (n * phrase + off).max(0.0)
}

/// Snap `t` to the nearest beat relative to `beat_offset_secs`.
pub fn snap_to_beat_grid(t: f64, bpm: f32, beat_offset_secs: f64) -> f64 {
    let beat = beat_secs(bpm);
    if beat <= 0.0 || !t.is_finite() {
        return t.max(0.0);
    }
    let off = if beat_offset_secs.is_finite() {
        beat_offset_secs.rem_euclid(beat)
    } else {
        0.0
    };
    let rel = t - off;
    let n = (rel / beat).round();
    (n * beat + off).max(0.0)
}

/// Phrase snap when the track is long enough; otherwise beat-grid snap (short gold loops).
fn snap_cue_time(t: f64, bpm: f32, bars_per_phrase: u32, beat_offset_secs: f64, dur: f64) -> f64 {
    let phrase = bar_secs(bpm) * bars_per_phrase.max(1) as f64;
    if dur < phrase * 0.9 {
        snap_to_beat_grid(t, bpm, beat_offset_secs)
    } else {
        snap_to_phrase_grid(t, bpm, bars_per_phrase, beat_offset_secs)
    }
}

/// Snap a duration so the crossfade lands on an integer number of beats (prefer nearest).
///
/// Zero-length fades (HardCut) stay zero — they must not be inflated to one beat.
pub fn align_crossfade_to_downbeat(crossfade_secs: f32, bpm: f32) -> f32 {
    if !crossfade_secs.is_finite() || crossfade_secs <= 0.0 {
        return 0.0;
    }
    let beat = beat_secs(bpm);
    if beat <= 0.0 {
        return crossfade_secs.max(0.0);
    }
    let beats = (crossfade_secs as f64 / beat).round().max(1.0);
    (beats * beat) as f32
}

/// Phrase length in bars for a genre id (4/4 assumed).
pub fn bars_per_phrase_for_genre(genre_id: &str) -> u32 {
    match genre_id {
        "drum_and_bass" | "dnb" | "hip_hop" | "rnb" => 4,
        _ => 8,
    }
}

const PHRASE_CANDIDATES: [u32; 3] = [4, 8, 16];

fn normalize_phrase_bars(bars: u32) -> u32 {
    if PHRASE_CANDIDATES.contains(&bars) {
        bars
    } else {
        8
    }
}

fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

/// Least common multiple of two phrase lengths in `{4, 8, 16}`.
///
/// Invalid inputs are normalized to `8` before the LCM (e.g. `4∧8→8`, `8∧16→16`).
pub fn phrase_bars_lcm(a: u32, b: u32) -> u32 {
    let a = normalize_phrase_bars(a);
    let b = normalize_phrase_bars(b);
    let g = gcd_u32(a, b);
    let lcm = a / g * b;
    if PHRASE_CANDIDATES.contains(&lcm) {
        lcm
    } else {
        // Should not happen for {4,8,16}; fall back to the longer phrase.
        a.max(b)
    }
}

/// True when `t` sits within half a beat of the track's phrase grid (or beat grid on short loops).
fn near_phrase_or_beat_grid(
    t: f64,
    bpm: f32,
    bars_per_phrase: u32,
    beat_offset_secs: f64,
    dur: f64,
) -> bool {
    let bars = normalize_phrase_bars(bars_per_phrase);
    let phrase = bar_secs(bpm) * bars as f64;
    let half_beat = beat_secs(bpm) * 0.5;
    if dur < phrase * 0.9 {
        // Short gold loops use beat-grid snap — skip-with-pass for phrase gate.
        return true;
    }
    let snapped = snap_to_phrase_grid(t, bpm, bars, beat_offset_secs);
    (t - snapped).abs() <= half_beat + 1e-6
}

/// Measured phrase alignment for one mix edge (outgoing `play_end` ↔ incoming `play_start`).
///
/// Short tracks that take the beat-grid planner path always pass. Long tracks must land
/// within ~½ beat of each track's own phrase grid.
pub fn cue_edge_phrase_aligned(
    outgoing: &TrackFeatures,
    incoming: &TrackFeatures,
    out_end: f64,
    in_start: f64,
) -> bool {
    let out_bars = if matches!(outgoing.bars_per_phrase, 4 | 8 | 16) {
        outgoing.bars_per_phrase
    } else {
        8
    };
    let in_bars = if matches!(incoming.bars_per_phrase, 4 | 8 | 16) {
        incoming.bars_per_phrase
    } else {
        8
    };
    near_phrase_or_beat_grid(
        out_end,
        outgoing.bpm,
        out_bars,
        outgoing.beat_offset_secs,
        outgoing.duration_secs,
    ) && near_phrase_or_beat_grid(
        in_start,
        incoming.bpm,
        in_bars,
        incoming.beat_offset_secs,
        incoming.duration_secs,
    )
}

/// Heuristic per-track phrase length from section spacing.
///
/// When marks are ordered and the track is long enough for a phrase, quantizes
/// `(drop_start − intro_end) / bar` to `{4, 8, 16}`. Ties between 4 and 8 prefer
/// 8 (conservative house default). Otherwise returns `genre_fallback`.
pub fn estimate_bars_per_phrase(feats: &TrackFeatures, genre_fallback: u32) -> u32 {
    let fallback = if PHRASE_CANDIDATES.contains(&genre_fallback) {
        genre_fallback
    } else {
        8
    };
    if !sections_drop_ordered(&feats.sections) {
        return fallback;
    }
    let bpm = feats.bpm.max(1.0);
    let bar = bar_secs(bpm);
    if bar <= 0.0 {
        return fallback;
    }
    // Short loops stay on the beat-snap path inside `plan_segment_cues`; do not invent phrase.
    let min_phrase = bar * fallback.max(1) as f64;
    if feats.duration_secs < min_phrase * 0.9 {
        return fallback;
    }
    let span = (feats.sections.drop_start - feats.sections.intro_end).max(0.0);
    if span < bar * 0.5 {
        return fallback;
    }
    let bars = span / bar;
    // Prefer 8 on exact midway between 4 and 8 (distance tie).
    let mut best = 8u32;
    let mut best_dist = f64::MAX;
    for &cand in &PHRASE_CANDIDATES {
        let dist = (bars - cand as f64).abs();
        let better = dist + 1e-9 < best_dist
            || ((dist - best_dist).abs() < 1e-9 && cand == 8 && best != 8);
        if better {
            best_dist = dist;
            best = cand;
        }
    }
    best
}

/// Plan mix-in / mix-out cues for one track in a solved order.
///
/// - Non-first tracks start near a phrase boundary inside the intro region,
///   clamped at/before `drop_start` when section marks are ordered.
/// - Tracks with a successor end near a phrase-snapped outro so the fade covers
///   into the next track's intro head (xfade capped by next `drop_start`).
/// - When both tracks are long enough for phrase snap, outgoing `play_end` uses
///   [`phrase_bars_lcm`] of A and B so the join lands on a shared phrase period;
///   drop-ordered caps still win over LCM when they conflict.
/// - On long tracks, `play_end` stays at/after `drop_start` so mixes don't cut
///   before the drop; short gold loops keep the existing collapse rescue.
/// - Crossfade length is policy-driven, capped by available outro/intro, then
///   snapped to whole beats.
pub fn plan_segment_cues(
    feats: &TrackFeatures,
    next: Option<&TrackFeatures>,
    policy_xfade: f32,
    bars_per_phrase: u32,
    is_first: bool,
    is_last: bool,
) -> CuePoints {
    let dur = feats.duration_secs.max(0.0);
    let bpm = feats.bpm.max(1.0);
    let drop_ordered = sections_drop_ordered(&feats.sections);
    let own_bars = normalize_phrase_bars(bars_per_phrase);
    let phrase = bar_secs(bpm) * own_bars as f64;
    let long_enough = dur >= phrase * 0.9;

    let offset = feats.beat_offset_secs;
    let play_start = if is_first {
        // Keep the head of the first track (short gold loops need the full buffer).
        0.0
    } else {
        // Prefer early intro, snapped to phrase/beat grid; never start after the drop.
        let raw = feats.sections.intro_end.min(dur * 0.25).max(0.0);
        let candidate = if raw < bar_secs(bpm) {
            0.0
        } else {
            snap_cue_time(raw, bpm, own_bars, offset, dur)
        };
        let mut capped = candidate.clamp(0.0, (dur * 0.5).max(0.0));
        if drop_ordered {
            capped = capped.min(feats.sections.drop_start.max(0.0));
        }
        capped
    };

    let (play_end, crossfade_secs) = match (is_last, next) {
        (true, _) | (_, None) => (dur, 0.0f32),
        (false, Some(next)) => {
            let next_bars = if matches!(next.bars_per_phrase, 4 | 8 | 16) {
                next.bars_per_phrase
            } else {
                own_bars
            };
            let next_phrase = bar_secs(next.bpm.max(1.0)) * next_bars as f64;
            let next_long = next.duration_secs >= next_phrase * 0.9;
            // Shared phrase period for the outgoing join when both tracks phrase-snap.
            let end_bars = if long_enough && next_long {
                phrase_bars_lcm(own_bars, next_bars)
            } else {
                own_bars
            };

            let next_intro = next.sections.intro_end.max(beat_secs(next.bpm));
            let available_outro = (dur - feats.sections.outro_start).max(0.0);
            let mut available_intro = next_intro.min(next.duration_secs * 0.35).max(0.0);
            // Fade must finish before the next drop when marks are ordered.
            if sections_drop_ordered(&next.sections) {
                available_intro = available_intro.min(next.sections.drop_start.max(0.0));
            }

            let mut xfade = policy_xfade
                .max(0.0)
                .min(available_outro as f32)
                .min(available_intro as f32)
                .min((dur * 0.35) as f32);

            // Keep a usable fade on short gold loops.
            if xfade < 0.05 && policy_xfade > 0.0 {
                let mut rescued = policy_xfade
                    .min((dur * 0.35) as f32)
                    .min((next.duration_secs * 0.35) as f32)
                    .max(0.05);
                if sections_drop_ordered(&next.sections) {
                    let drop_cap = next.sections.drop_start.max(0.0) as f32;
                    if drop_cap > 0.0 {
                        rescued = rescued.min(drop_cap);
                    }
                }
                xfade = rescued;
            }

            xfade = align_crossfade_to_downbeat(xfade, bpm);

            // End of play window: LCM (or own) phrase snap near outro_start + xfade.
            let raw_end = (feats.sections.outro_start + xfade as f64).min(dur);
            let mut play_end = snap_cue_time(raw_end, bpm, end_bars, offset, dur);
            if play_end <= play_start + xfade as f64 {
                // Rescue without re-collapsing onto the first grid point.
                play_end = (play_start + xfade as f64 + bar_secs(bpm)).min(dur);
                play_end = snap_to_beat_grid(play_end, bpm, offset).clamp(play_start + xfade as f64, dur);
            }
            if play_end > dur {
                play_end = dur;
            }
            // Long tracks: do not cut the play window before the drop (caps beat LCM).
            if long_enough && drop_ordered && play_end < feats.sections.drop_start {
                let floored = snap_cue_time(
                    feats.sections.drop_start,
                    bpm,
                    end_bars,
                    offset,
                    dur,
                )
                .max(feats.sections.drop_start)
                .min(dur);
                play_end = floored.max(play_end);
            }
            // Ensure we still have room for the crossfade inside the window.
            if play_end - play_start < xfade as f64 {
                xfade = ((play_end - play_start) * 0.5).max(0.0) as f32;
            }

            (play_end, xfade)
        }
    };

    CuePoints {
        play_start_secs: play_start.clamp(0.0, dur),
        play_end_secs: play_end.clamp(play_start, dur).max(play_start),
        crossfade_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::SectionMarks;
    use crate::camelot::CamelotKey;

    fn feats(bpm: f32, dur: f64, intro: f64, outro: f64) -> TrackFeatures {
        feats_with_drop(bpm, dur, intro, dur * 0.4, outro)
    }

    fn feats_with_drop(bpm: f32, dur: f64, intro: f64, drop: f64, outro: f64) -> TrackFeatures {
        TrackFeatures {
            path: None,
            bpm,
            key: CamelotKey::parse("8A").unwrap(),
            loudness_db: -12.0,
            duration_secs: dur,
            sample_rate: 44_100,
            sections: SectionMarks {
                intro_end: intro,
                drop_start: drop,
                outro_start: outro,
            },
            spectral_centroid_hz: 1000.0,
            beat_offset_secs: 0.0,
            quality: crate::analysis::AnalysisQuality::default(),
            bars_per_phrase: 0,
        }
    }

    #[test]
    fn estimate_phrase_from_section_span() {
        let bpm = 128.0;
        let bar = bar_secs(bpm);
        // Exactly 16 bars intro→drop → candidate 16.
        let a = feats_with_drop(bpm, 120.0, 0.0, bar * 16.0, 90.0);
        assert_eq!(estimate_bars_per_phrase(&a, 8), 16);
        // Exactly 8 bars → 8.
        let b = feats_with_drop(bpm, 120.0, 0.0, bar * 8.0, 90.0);
        assert_eq!(estimate_bars_per_phrase(&b, 4), 8);
        // Exactly 4 bars → 4.
        let c = feats_with_drop(bpm, 120.0, 0.0, bar * 4.0, 90.0);
        assert_eq!(estimate_bars_per_phrase(&c, 8), 4);
        // ~16s span @ 128 BPM ≈ 8.53 bars → nearest 8 (plan smoke case).
        let s = feats_with_drop(bpm, 120.0, 0.0, 16.0, 90.0);
        assert_eq!(estimate_bars_per_phrase(&s, 4), 8);
        // Unordered sections → genre fallback.
        let d = feats_with_drop(bpm, 120.0, 40.0, 10.0, 90.0);
        assert!(!sections_drop_ordered(&d.sections));
        assert_eq!(estimate_bars_per_phrase(&d, 4), 4);
        // Short loop → fallback.
        let e = feats_with_drop(bpm, 8.0, 1.0, 3.0, 6.0);
        assert_eq!(estimate_bars_per_phrase(&e, 8), 8);
    }

    #[test]
    fn mix_in_clamped_at_or_before_drop() {
        // 4-bar phrases @ 120 BPM = 8s; snap(7.5) → 8.0 which would pass drop 7.8 without clamp.
        let a = feats_with_drop(120.0, 64.0, 7.5, 7.8, 48.0);
        let b = feats_with_drop(120.0, 64.0, 4.0, 16.0, 48.0);
        assert!(sections_drop_ordered(&a.sections));
        let cues = plan_segment_cues(&a, Some(&b), 8.0, 4, false, false);
        assert!(
            cues.play_start_secs <= a.sections.drop_start + 1e-9,
            "play_start {} > drop {}",
            cues.play_start_secs,
            a.sections.drop_start
        );
    }

    #[test]
    fn xfade_head_capped_by_next_drop() {
        let a = feats_with_drop(120.0, 64.0, 4.0, 16.0, 48.0);
        // Intro longer than drop window would be unordered; keep intro ≤ drop and
        // make drop the binding cap vs a large policy xfade.
        let b = feats_with_drop(120.0, 64.0, 2.0, 3.0, 48.0);
        assert!(sections_drop_ordered(&b.sections));
        let cues = plan_segment_cues(&a, Some(&b), 16.0, 8, true, false);
        assert!(
            cues.crossfade_secs as f64 <= b.sections.drop_start + beat_secs(120.0) + 1e-3,
            "xfade {} exceeds next drop {}",
            cues.crossfade_secs,
            b.sections.drop_start
        );
    }

    #[test]
    fn long_track_play_end_reaches_drop() {
        // Outro before drop is repaired by analysis in CLI; here force a late drop
        // with an early outro-like end by placing outro just after drop on a long track.
        let a = feats_with_drop(120.0, 64.0, 4.0, 32.0, 33.0);
        let b = feats_with_drop(120.0, 64.0, 4.0, 16.0, 48.0);
        let cues = plan_segment_cues(&a, Some(&b), 1.0, 8, true, false);
        assert!(
            cues.play_end_secs + 1e-6 >= a.sections.drop_start,
            "play_end {} < drop {}",
            cues.play_end_secs,
            a.sections.drop_start
        );
    }

    #[test]
    fn snap_grid_respects_offset() {
        let bpm = 120.0;
        // 8 bars = 16 s; offset 0.5 s → nearest phrase to 16.4 is 16.5
        let snapped = snap_to_phrase_grid(16.4, bpm, 8, 0.5);
        assert!((snapped - 16.5).abs() < 1e-6, "got {snapped}");
    }

    #[test]
    fn snap_lands_on_phrase_grid() {
        let bpm = 120.0;
        // 8 bars at 120 BPM = 16 seconds.
        let snapped = snap_to_phrase(17.0, bpm, 8);
        assert!((snapped - 16.0).abs() < 1e-6);
        let rem = snapped % (bar_secs(bpm) * 8.0);
        assert!(rem < 1e-6 || (bar_secs(bpm) * 8.0 - rem) < 1e-6);
    }

    #[test]
    fn crossfade_snaps_to_whole_beats() {
        let bpm = 120.0;
        let aligned = align_crossfade_to_downbeat(1.7, bpm);
        let beat = beat_secs(bpm);
        // 1.7s @ 120 BPM → nearest 3 beats = 1.5s
        assert!((aligned as f64 - 1.5).abs() < 1e-3);
        let rem = (aligned as f64) % beat;
        assert!(rem < 1e-3 || (beat - rem) < 1e-3);
    }

    #[test]
    fn hard_cut_zero_crossfade_stays_zero() {
        assert_eq!(align_crossfade_to_downbeat(0.0, 128.0), 0.0);
        let a = feats_with_drop(128.0, 8.0, 1.0, 3.0, 6.0);
        let b = feats_with_drop(128.0, 8.0, 1.0, 3.0, 6.0);
        let cues = plan_segment_cues(&a, Some(&b), 0.0, 8, true, false);
        assert_eq!(cues.crossfade_secs, 0.0);
    }

    #[test]
    fn plans_outgoing_near_outro() {
        let a = feats(128.0, 32.0, 4.0, 24.0);
        let b = feats(128.0, 32.0, 4.0, 24.0);
        let cues = plan_segment_cues(&a, Some(&b), 8.0, 8, true, false);
        assert!(cues.play_start_secs < 1.0);
        assert!(cues.play_end_secs >= a.sections.outro_start - 1.0);
        assert!(cues.crossfade_secs > 0.0);
        // Phrase-aligned end should be near a phrase boundary.
        let phrase = bar_secs(128.0) * 8.0;
        let rem = cues.play_end_secs % phrase;
        assert!(rem < 0.05 || (phrase - rem) < 0.05 || cues.play_end_secs >= 31.9);
    }

    #[test]
    fn genre_phrase_lengths() {
        assert_eq!(bars_per_phrase_for_genre("house"), 8);
        assert_eq!(bars_per_phrase_for_genre("drum_and_bass"), 4);
        assert_eq!(bars_per_phrase_for_genre("hip_hop"), 4);
    }

    #[test]
    fn phrase_bars_lcm_for_common_pairs() {
        assert_eq!(phrase_bars_lcm(4, 8), 8);
        assert_eq!(phrase_bars_lcm(8, 16), 16);
        assert_eq!(phrase_bars_lcm(4, 16), 16);
        assert_eq!(phrase_bars_lcm(8, 8), 8);
        assert_eq!(phrase_bars_lcm(0, 4), 8); // invalid → 8, lcm(8,4)=8
    }

    #[test]
    fn lcm_snap_aligns_outgoing_play_end() {
        let bpm = 120.0;
        let bar = bar_secs(bpm);
        // Long tracks: 4-bar A → 8-bar B; LCM=8 bars = 16s.
        let mut a = feats_with_drop(bpm, 64.0, 4.0, 16.0, 40.0);
        a.bars_per_phrase = 4;
        let mut b = feats_with_drop(bpm, 64.0, 4.0, 16.0, 48.0);
        b.bars_per_phrase = 8;
        let cues = plan_segment_cues(&a, Some(&b), 8.0, 4, true, false);
        let shared = bar * 8.0;
        let rem = cues.play_end_secs % shared;
        assert!(
            rem < 0.05 || (shared - rem) < 0.05 || cues.play_end_secs >= 63.9,
            "play_end {} not on LCM-8 grid (rem {rem})",
            cues.play_end_secs
        );
        assert!(cue_edge_phrase_aligned(
            &a,
            &b,
            cues.play_end_secs,
            0.0 // first-segment next start is planned separately; 0 is on grid
        ));
    }

    #[test]
    fn short_loop_phrase_gate_skip_with_pass() {
        let a = feats_with_drop(128.0, 8.0, 1.0, 3.0, 6.0);
        let b = feats_with_drop(128.0, 8.0, 1.0, 3.0, 6.0);
        // Mid-phrase times would fail on long tracks; shorts skip-with-pass.
        assert!(cue_edge_phrase_aligned(&a, &b, 5.3, 2.7));
    }

    #[test]
    fn long_edge_misaligned_fails_gate() {
        let bpm = 120.0;
        let mut a = feats_with_drop(bpm, 64.0, 4.0, 16.0, 48.0);
        a.bars_per_phrase = 8;
        let mut b = feats_with_drop(bpm, 64.0, 4.0, 16.0, 48.0);
        b.bars_per_phrase = 8;
        // 1.0s is not on an 8-bar (16s) phrase grid at 120 BPM.
        assert!(!cue_edge_phrase_aligned(&a, &b, 1.0, 1.0));
        // Exact phrase boundary passes.
        assert!(cue_edge_phrase_aligned(&a, &b, 16.0, 16.0));
    }
}
