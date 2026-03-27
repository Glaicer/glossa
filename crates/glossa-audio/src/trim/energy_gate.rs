use async_trait::async_trait;
use camino::Utf8PathBuf;

use glossa_app::{ports::SilenceTrimmer, AppError};
use glossa_core::CapturedAudio;

use crate::wav::read_wav_i16;

/// Simple absolute-amplitude silence trimmer for WAV recordings.
#[derive(Debug, Clone)]
pub struct WavSilenceTrimmer {
    threshold: i16,
}

impl Default for WavSilenceTrimmer {
    fn default() -> Self {
        Self { threshold: 500 }
    }
}

#[async_trait]
impl SilenceTrimmer for WavSilenceTrimmer {
    async fn trim(&self, input: &CapturedAudio) -> Result<CapturedAudio, AppError> {
        if input.path.extension() != Some("wav") {
            return Ok(input.clone());
        }

        let path = input.path.clone();
        let captured = input.clone();
        let threshold = self.threshold;
        tokio::task::spawn_blocking(move || trim_wav(path, captured, threshold))
            .await
            .map_err(|error| AppError::message(format!("failed to join trim task: {error}")))?
    }
}

fn trim_wav(
    path: Utf8PathBuf,
    input: CapturedAudio,
    threshold: i16,
) -> Result<CapturedAudio, AppError> {
    let (spec, samples) = read_wav_i16(path.as_std_path()).map_err(AppError::message)?;
    let channels = spec.channels as usize;
    let frame_count = samples.len() / channels;

    let first = (0..frame_count).find(|frame_index| {
        let start = frame_index * channels;
        samples[start..start + channels]
            .iter()
            .any(|sample| sample.abs() > threshold)
    });
    let last = (0..frame_count).rfind(|frame_index| {
        let start = frame_index * channels;
        samples[start..start + channels]
            .iter()
            .any(|sample| sample.abs() > threshold)
    });

    let Some(first_frame) = first else {
        return Ok(CapturedAudio {
            duration_ms: 0,
            ..input.clone()
        });
    };
    let Some(last_frame) = last else {
        return Ok(input.clone());
    };

    let trimmed_path = path.with_file_name(format!(
        "{}-trimmed.wav",
        path.file_stem().unwrap_or("capture")
    ));
    let mut writer = hound::WavWriter::create(trimmed_path.as_std_path(), spec)
        .map_err(|error| AppError::message(format!("failed to create trimmed wav: {error}")))?;
    let sample_start = first_frame * channels;
    let sample_end = (last_frame + 1) * channels;
    for sample in &samples[sample_start..sample_end] {
        writer.write_sample(*sample).map_err(|error| {
            AppError::message(format!("failed to write trimmed sample: {error}"))
        })?;
    }
    writer
        .finalize()
        .map_err(|error| AppError::message(format!("failed to finalize trimmed wav: {error}")))?;
    let trimmed_duration_ms =
        (((sample_end - sample_start) / channels) as f64 / spec.sample_rate as f64 * 1000.0) as u64;

    Ok(CapturedAudio {
        path: trimmed_path,
        duration_ms: trimmed_duration_ms,
        ..input
    })
}
