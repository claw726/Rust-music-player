use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SongMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub format: String,
    pub bit_rate: Option<u32>,
}

impl Default for SongMetadata {
    fn default() -> Self {
        SongMetadata {
            title: None,
            artist: None,
            album: None,
            duration: None,
            year: None,
            track_number: None,
            format: String::from("Unknown"),
            bit_rate: None,
        }
    }
}