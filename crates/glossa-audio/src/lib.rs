//! Audio capture, WAV persistence, trimming, and cue playback for Glossa.

pub mod capture;
pub mod cue;
pub mod trim;
pub mod wav;

pub use self::capture::CpalAudioCapture;
pub use self::cue::RodioCuePlayer;
pub use self::trim::WavSilenceTrimmer;
