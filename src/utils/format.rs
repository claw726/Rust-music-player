use lofty::file::FileType;
use std::time::Duration;

pub fn format_to_string(file_type: FileType) -> String {
    match file_type {
        FileType::Flac => "FLAC".to_string(),
        FileType::Opus => "Opus".to_string(),
        FileType::Vorbis => "Vorbis".to_string(),
        FileType::Mpeg => "MP3".to_string(),
        FileType::Wav => "WAV".to_string(),
        FileType::Aiff => "AIFF".to_string(),
        FileType::Mp4 => "MP4".to_string(),
        FileType::Aac => "AAC".to_string(),
        FileType::Ape => "APE".to_string(),
        FileType::Mpc => "MPC".to_string(),
        FileType::WavPack => "WavPack".to_string(),
        FileType::Speex => "Speex".to_string(),
        FileType::Custom(format) => format.to_string(),
        _ => "Unknown".to_string(),
    }
}
pub fn format_bitrate(bitrate: u32) -> String {
    if bitrate == 0 {
        return "Unknown".to_string();
    }
    format!("{} kbps", bitrate)
}

pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / (60 * 60);
    let minutes = (total_seconds % (60 * 60)) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}