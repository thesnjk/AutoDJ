//! AutoDJ analysis and offline DSP.
//!
//! Hot paths intended for future real-time use must avoid heap allocation;
//! the MVP offline renderer may allocate during graph construction only.

mod analysis;
mod camelot;
mod cue;
mod eq;
mod loudness;
mod render;

pub use analysis::{analyze_mono, analyze_path, SectionMarks, TrackFeatures, TransitionClass};
pub use camelot::{camelot_distance, CamelotKey};
pub use cue::{
    align_crossfade_to_downbeat, bar_secs, bars_per_phrase_for_genre, beat_secs, plan_segment_cues,
    snap_to_phrase, CuePoints,
};
pub use eq::{
    apply_echo_out_crossfade, apply_eq_swap_crossfade, apply_filter_sweep_crossfade, band_rms,
    eq_swap_gains, BandGains, ThreeBandSplitter,
};
pub use loudness::{
    measure_loudness, normalize_loudness, within_lufs_tolerance, LoudnessError, LoudnessReport,
    BROADCAST_TARGET_LUFS, DEFAULT_TARGET_LUFS, LUFS_TOLERANCE_LU,
};
pub use render::{mix_buffers, CrossfadeParams, MixPlan, MixSegment};
