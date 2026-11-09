//! AutoDJ analysis and offline DSP.
//!
//! Hot paths intended for future real-time use must avoid heap allocation;
//! the MVP offline renderer may allocate during graph construction only.

mod analysis;
mod camelot;
mod render;

pub use analysis::{analyze_mono, analyze_path, SectionMarks, TrackFeatures, TransitionClass};
pub use camelot::{camelot_distance, CamelotKey};
pub use render::{mix_buffers, CrossfadeParams, MixPlan, MixSegment};
