//! Phrase-aware mix-in / mix-out cue planning.
//!
//! Offline planner — allocates freely; do not call from a real-time audio thread.

use crate::analysis::TrackFeatures;

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
    let phrase = bar_secs(bpm) * bars_per_phrase.max(1) as f64;
    if phrase <= 0.0 || !t.is_finite() {
        return t.max(0.0);
    }
    let n = (t / phrase).round();
    (n * phrase).max(0.0)
}

/// Snap a duration so the crossfade lands on an integer number of beats (prefer nearest).
pub fn align_crossfade_to_downbeat(crossfade_secs: f32, bpm: f32) -> f32 {
    let beat = beat_secs(bpm);
    if beat <= 0.0 || !crossfade_secs.is_finite() {
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

/// Plan mix-in / mix-out cues for one track in a solved order.
///
/// - Non-first tracks start near a phrase boundary inside the intro region.
/// - Tracks with a successor end near a phrase-snapped outro so the fade covers
///   into the next track's intro head.
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

    let play_start = if is_first {
        0.0
    } else {
        // Prefer the top of the track / early intro, snapped to phrase.
        let raw = feats.sections.intro_end.min(dur * 0.25).max(0.0);
        // For short loops, intro_end may be tiny — stay near 0.
        let candidate = if raw < bar_secs(bpm) {
            0.0
        } else {
            snap_to_phrase(0.0, bpm, bars_per_phrase)
        };
        candidate.clamp(0.0, (dur * 0.5).max(0.0))
    };

    let (play_end, crossfade_secs) = match (is_last, next) {
        (true, _) | (_, None) => (dur, 0.0f32),
        (false, Some(next)) => {
            let next_intro = next.sections.intro_end.max(beat_secs(next.bpm));
            let available_outro = (dur - feats.sections.outro_start).max(0.0);
            let available_intro = next_intro.min(next.duration_secs * 0.35).max(0.0);

            let mut xfade = policy_xfade
                .max(0.0)
                .min(available_outro as f32)
                .min(available_intro as f32)
                .min((dur * 0.35) as f32);

            // Keep a usable fade on short gold loops.
            if xfade < 0.05 && policy_xfade > 0.0 {
                xfade = policy_xfade
                    .min((dur * 0.35) as f32)
                    .min((next.duration_secs * 0.35) as f32)
                    .max(0.05);
            }

            xfade = align_crossfade_to_downbeat(xfade, bpm);

            // End of play window: phrase-snap near outro_start + xfade (or duration).
            let raw_end = (feats.sections.outro_start + xfade as f64).min(dur);
            let mut play_end = snap_to_phrase(raw_end, bpm, bars_per_phrase);
            if play_end <= play_start + xfade as f64 {
                play_end = (play_start + xfade as f64 + bar_secs(bpm)).min(dur);
            }
            if play_end > dur {
                play_end = dur;
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
        TrackFeatures {
            path: None,
            bpm,
            key: CamelotKey::parse("8A").unwrap(),
            loudness_db: -12.0,
            duration_secs: dur,
            sample_rate: 44_100,
            sections: SectionMarks {
                intro_end: intro,
                drop_start: dur * 0.4,
                outro_start: outro,
            },
            spectral_centroid_hz: 1000.0,
            quality: crate::analysis::AnalysisQuality::default(),
        }
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
}
