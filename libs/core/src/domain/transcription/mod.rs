pub mod ports;

use bytes::Bytes;
use capitainerie_common::CoreError;

use crate::domain::provider::ModelRef;

pub const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioMime {
    Mpeg,
    Mp4,
    Wav,
    Webm,
    Ogg,
}

#[derive(Debug, Clone)]
pub struct AudioClip {
    pub file_name: String,
    pub mime: AudioMime,
    pub bytes: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionText(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcription {
    pub text: String,
    pub model: ModelRef,
}

#[derive(Debug, thiserror::Error)]
pub enum TranscribeError {
    #[error("no transcriber is assigned for this organization")]
    NotConfigured,
    #[error("{0}")]
    Provider(String),
    #[error(transparent)]
    Core(#[from] CoreError),
}

impl AudioMime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mpeg => "audio/mpeg",
            Self::Mp4 => "audio/mp4",
            Self::Wav => "audio/wav",
            Self::Webm => "audio/webm",
            Self::Ogg => "audio/ogg",
        }
    }

    pub fn from_file_name(name: &str) -> Option<Self> {
        let (_, extension) = name.rsplit_once('.')?;
        match extension.to_ascii_lowercase().as_str() {
            "mp3" => Some(Self::Mpeg),
            "m4a" | "mp4" => Some(Self::Mp4),
            "wav" => Some(Self::Wav),
            "webm" => Some(Self::Webm),
            "ogg" => Some(Self::Ogg),
            _ => None,
        }
    }

    pub fn from_content_type(content_type: &str) -> Option<Self> {
        let essence = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        match essence.as_str() {
            "audio/mpeg" | "audio/mp3" => Some(Self::Mpeg),
            "audio/mp4" | "audio/m4a" | "audio/x-m4a" => Some(Self::Mp4),
            "audio/wav" | "audio/x-wav" | "audio/wave" => Some(Self::Wav),
            "audio/webm" | "video/webm" => Some(Self::Webm),
            "audio/ogg" | "application/ogg" => Some(Self::Ogg),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_names_the_format() {
        let cases = [
            ("a.mp3", AudioMime::Mpeg),
            ("a.M4A", AudioMime::Mp4),
            ("a.wav", AudioMime::Wav),
            ("réunion.final.webm", AudioMime::Webm),
            ("a.ogg", AudioMime::Ogg),
        ];
        for (name, mime) in cases {
            assert_eq!(AudioMime::from_file_name(name), Some(mime), "{name}");
        }
        assert_eq!(AudioMime::from_file_name("a.txt"), None);
        assert_eq!(AudioMime::from_file_name("mp3"), None);
    }

    #[test]
    fn the_content_type_names_the_format_without_its_parameters() {
        assert_eq!(
            AudioMime::from_content_type("audio/webm; codecs=opus"),
            Some(AudioMime::Webm)
        );
        assert_eq!(
            AudioMime::from_content_type("Audio/X-M4A"),
            Some(AudioMime::Mp4)
        );
        assert_eq!(AudioMime::from_content_type("text/plain"), None);
    }
}
