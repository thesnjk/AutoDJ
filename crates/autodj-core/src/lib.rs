//! AutoDJ analysis and offline DSP.
//!
//! Hot paths intended for future real-time use must avoid heap allocation;
//! the MVP offline renderer may allocate during graph construction only.

mod analysis;
mod camelot;
mod cue;
mod eq;
mod eval;
mod loudness;
mod rekordbox;
mod render;
mod stretch;

pub use analysis::{
    analyze_mono, analyze_path, AnalysisQuality, SectionMarks, TrackFeatures, TransitionClass,
};
pub use camelot::{camelot_distance, CamelotKey};
pub use cue::{
    align_crossfade_to_downbeat, bar_secs, bars_per_phrase_for_genre, beat_secs,
    cue_edge_phrase_aligned, estimate_bars_per_phrase, phrase_bars_lcm, plan_segment_cues,
    sections_drop_ordered, snap_to_beat_grid, snap_to_phrase, snap_to_phrase_grid, CuePoints,
};
pub use eq::{
    apply_echo_out_crossfade, apply_eq_swap_crossfade, apply_filter_sweep_crossfade,
    apply_riser_out_crossfade, apply_tremolo_out_crossfade, band_rms, bass_conflict_pass,
    eq_swap_gains, max_eq_swap_bass_overlap, BandGains, ThreeBandSplitter,
    DEFAULT_BASS_OVERLAP_LIMIT,
};
pub use eval::{
    adjacent_flux_ratio, beat_phase_gate_pass, join_continuity_pass, max_join_flux_ratio,
    mean_beat_phase_offset_ms, peak_gate_pass, spectral_continuity_pass, spectral_flux_envelope,
    BeatPhaseCue, DEFAULT_BEAT_PHASE_LIMIT_MS, DEFAULT_JOIN_FLUX_RATIO, DEFAULT_MAX_FLUX_RATIO,
    DEFAULT_TRUE_PEAK_LIMIT,
};
pub use loudness::{
    measure_loudness, normalize_loudness, within_lufs_tolerance, LoudnessError, LoudnessReport,
    BROADCAST_TARGET_LUFS, DEFAULT_TARGET_LUFS, LUFS_TOLERANCE_LU,
};
pub use rekordbox::{
    parse_rekordbox_bytes, parse_rekordbox_xml, resolve_location, RekordboxPlaylist, RekordboxTrack,
};
pub use render::{mix_buffers, mix_buffers_with_meta, CrossfadeParams, MixPlan, MixRenderResult, MixSegment};
pub use stretch::{
    fit_length, should_tempo_blend, stretch_clamp_excess, tempo_blend_head, tempo_ratio,
    tempo_stretch_gate_pass, wsola_stretch, STRETCH_MAX, STRETCH_MIN, TEMPO_BLEND_EPSILON_BPM,
};
