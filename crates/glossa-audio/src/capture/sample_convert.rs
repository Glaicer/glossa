/// Converts a normalized sample into `i16` PCM.
#[must_use]
pub fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// Converts an unsigned 16-bit sample into signed `i16` PCM.
#[must_use]
pub fn u16_to_i16(sample: u16) -> i16 {
    (sample as i32 - i16::MAX as i32 - 1) as i16
}
