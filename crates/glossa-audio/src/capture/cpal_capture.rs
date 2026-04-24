use std::{
    sync::mpsc::{self, SyncSender},
    thread,
    time::Instant,
};

use async_trait::async_trait;
use camino::Utf8Path;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, StreamConfig,
};

use glossa_app::{ports::AudioCapture, AppError};
use glossa_core::{RecordSpec, SessionId};

use super::{active_recording::CpalActiveRecording, sample_convert};

type WriterHandle = thread::JoinHandle<Result<(), String>>;

/// Audio capture backed by the system default input device through CPAL.
#[derive(Debug, Default, Clone)]
pub struct CpalAudioCapture;

#[async_trait]
impl AudioCapture for CpalAudioCapture {
    async fn start(
        &self,
        session_id: SessionId,
        spec: RecordSpec,
        path: &Utf8Path,
    ) -> Result<Box<dyn glossa_app::ports::ActiveRecording>, AppError> {
        let startup_started_at = Instant::now();
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::message("no default input device is available"))?;
        let default_config = device
            .default_input_config()
            .map_err(|error| AppError::message(format!("failed to read input config: {error}")))?;
        let fallback_config = default_config.config();
        let requested_config = StreamConfig {
            channels: spec.channels,
            sample_rate: SampleRate(spec.sample_rate_hz),
            buffer_size: cpal::BufferSize::Default,
        };

        let writer_started_at = Instant::now();
        let (writer_tx, writer_handle, writer_config) = start_writer(path, &requested_config);
        let writer_setup_ms = writer_started_at.elapsed().as_millis();
        let stream_build_started_at = Instant::now();
        let (stream, used_fallback_config) = match build_stream(
            &device,
            default_config.sample_format(),
            &requested_config,
            writer_tx.clone(),
        ) {
            Ok(stream) => (stream, false),
            Err(error) => {
                tracing::info!(
                    %session_id,
                    requested_sample_rate_hz = requested_config.sample_rate.0,
                    requested_channels = requested_config.channels,
                    fallback_sample_rate_hz = fallback_config.sample_rate.0,
                    fallback_channels = fallback_config.channels,
                    error = %error,
                    "requested input config failed, retrying with default input config"
                );
                (
                    build_stream(
                        &device,
                        default_config.sample_format(),
                        &fallback_config,
                        writer_tx.clone(),
                    )
                    .map_err(AppError::message)?,
                    true,
                )
            }
        };
        let stream_build_ms = stream_build_started_at.elapsed().as_millis();
        let play_started_at = Instant::now();
        stream
            .play()
            .map_err(|error| AppError::message(format!("failed to start audio stream: {error}")))?;
        let stream_play_ms = play_started_at.elapsed().as_millis();
        tracing::info!(
            %session_id,
            requested_sample_rate_hz = spec.sample_rate_hz,
            requested_channels = spec.channels,
            writer_sample_rate_hz = writer_config.sample_rate.0,
            writer_channels = writer_config.channels,
            fallback_sample_rate_hz = fallback_config.sample_rate.0,
            fallback_channels = fallback_config.channels,
            used_fallback_config,
            writer_setup_ms,
            stream_build_ms,
            stream_play_ms,
            total_startup_ms = startup_started_at.elapsed().as_millis(),
            "audio capture initialized"
        );

        Ok(Box::new(CpalActiveRecording {
            session_id,
            path: path.to_owned(),
            sample_rate_hz: writer_config.sample_rate.0,
            channels: writer_config.channels,
            started_at: Instant::now(),
            stream,
            tx: Some(writer_tx),
            writer_handle: Some(writer_handle),
        }))
    }
}

fn start_writer(
    path: &Utf8Path,
    config: &StreamConfig,
) -> (SyncSender<Vec<i16>>, WriterHandle, StreamConfig) {
    let wav_path = path.to_path_buf().into_std_path_buf();
    let wav_spec = hound::WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate.0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let (tx, rx) = mpsc::sync_channel::<Vec<i16>>(8);
    let handle = thread::spawn(move || -> Result<(), String> {
        let mut writer = hound::WavWriter::create(wav_path, wav_spec)
            .map_err(|error| format!("failed to create wav writer: {error}"))?;
        while let Ok(chunk) = rx.recv() {
            for sample in chunk {
                writer
                    .write_sample(sample)
                    .map_err(|error| format!("failed to write wav sample: {error}"))?;
            }
        }
        writer
            .finalize()
            .map_err(|error| format!("failed to finalize wav writer: {error}"))?;
        Ok(())
    });
    (tx, handle, config.clone())
}

fn build_stream(
    device: &cpal::Device,
    sample_format: SampleFormat,
    config: &StreamConfig,
    tx: SyncSender<Vec<i16>>,
) -> Result<cpal::Stream, String> {
    let error_callback = |error| tracing::error!(error = %error, "audio stream error");

    match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| {
                    send_converted(data.iter().copied().map(sample_convert::f32_to_i16), &tx)
                },
                error_callback,
                None,
            )
            .map_err(|error| format!("failed to build f32 input stream: {error}")),
        SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| send_converted(data.iter().copied(), &tx),
                error_callback,
                None,
            )
            .map_err(|error| format!("failed to build i16 input stream: {error}")),
        SampleFormat::U16 => device
            .build_input_stream(
                config,
                move |data: &[u16], _| {
                    send_converted(data.iter().copied().map(sample_convert::u16_to_i16), &tx)
                },
                error_callback,
                None,
            )
            .map_err(|error| format!("failed to build u16 input stream: {error}")),
        _ => Err("unsupported input sample format".into()),
    }
}

fn send_converted<I>(samples: I, tx: &SyncSender<Vec<i16>>)
where
    I: Iterator<Item = i16>,
{
    let chunk: Vec<i16> = samples.collect();
    let _ = tx.send(chunk);
}
