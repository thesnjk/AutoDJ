//! Decode and encode mono/stereo PCM for AutoDJ offline rendering.

use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use thiserror::Error;

/// Errors from audio I/O.
#[derive(Debug, Error)]
pub enum IoError {
    /// Underlying hound / filesystem failure.
    #[error("audio io: {0}")]
    Hound(#[from] hound::Error),
    /// Unsupported sample format.
    #[error("unsupported format: {0}")]
    Unsupported(String),
}

/// Interleaved floating-point PCM in range approximately `[-1.0, 1.0]`.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count (1 or 2 for MVP).
    pub channels: u16,
    /// Interleaved samples.
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    /// Number of frames (samples / channels).
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    /// Downmix to mono (average channels).
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            return self.samples.clone();
        }
        let ch = self.channels as usize;
        let mut mono = Vec::with_capacity(self.frames());
        for frame in self.samples.chunks_exact(ch) {
            let sum: f32 = frame.iter().sum();
            mono.push(sum / ch as f32);
        }
        mono
    }

    /// Return a copy of frames in `[start_secs, end_secs)` (clamped to the buffer).
    pub fn slice_secs(&self, start_secs: f64, end_secs: f64) -> AudioBuffer {
        let ch = self.channels.max(1) as usize;
        let total = self.frames();
        let start = ((start_secs.max(0.0) * self.sample_rate as f64) as usize).min(total);
        let end = ((end_secs.max(0.0) * self.sample_rate as f64) as usize)
            .max(start)
            .min(total);
        let byte_start = start * ch;
        let byte_end = end * ch;
        AudioBuffer {
            sample_rate: self.sample_rate,
            channels: self.channels,
            samples: self.samples[byte_start..byte_end].to_vec(),
        }
    }
}

/// Read a WAV file into an [`AudioBuffer`].
pub fn read_wav(path: impl AsRef<Path>) -> Result<AudioBuffer, IoError> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.channels > 2 {
        return Err(IoError::Unsupported(format!(
            "{} channels",
            spec.channels
        )));
    }
    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => match spec.bits_per_sample {
            16 => reader
                .samples::<i16>()
                .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
                .collect::<Result<_, _>>()?,
            24 | 32 => reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / i32::MAX as f32))
                .collect::<Result<_, _>>()?,
            other => {
                return Err(IoError::Unsupported(format!("{other}-bit int")));
            }
        },
        SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(AudioBuffer {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        samples,
    })
}

/// Write mono or stereo f32 samples as 16-bit PCM WAV.
pub fn write_wav(path: impl AsRef<Path>, buffer: &AudioBuffer) -> Result<(), IoError> {
    let spec = WavSpec {
        channels: buffer.channels,
        sample_rate: buffer.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &s in &buffer.samples {
        let clipped = s.clamp(-1.0, 1.0);
        writer.write_sample((clipped * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_mono_wav() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wav");
        let buf = AudioBuffer {
            sample_rate: 44100,
            channels: 1,
            samples: vec![0.0, 0.5, -0.5, 0.25],
        };
        write_wav(&path, &buf).unwrap();
        let got = read_wav(&path).unwrap();
        assert_eq!(got.sample_rate, 44100);
        assert_eq!(got.channels, 1);
        assert_eq!(got.samples.len(), 4);
    }

    #[test]
    fn slice_secs_extracts_frames() {
        let buf = AudioBuffer {
            sample_rate: 10,
            channels: 1,
            samples: (0..20).map(|i| i as f32).collect(),
        };
        let sliced = buf.slice_secs(0.5, 1.2);
        assert_eq!(sliced.samples, vec![5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]);
    }
}
