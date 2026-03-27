use std::path::Path;

/// Reads a WAV file as signed 16-bit PCM samples.
pub fn read_wav_i16(path: &Path) -> Result<(hound::WavSpec, Vec<i16>), String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|error| format!("failed to open wav file: {error}"))?;
    let spec = reader.spec();
    let samples = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to decode wav samples: {error}"))?;
    Ok((spec, samples))
}
