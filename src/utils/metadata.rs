use std::path::Path;
use lofty::{
    prelude::*,
    probe::Probe,
    tag::Tag,
    file::FileType,
};
use crate::models::song_metadata::SongMetadata;
use crate::utils::format::{format_to_string, format_bitrate, format_duration};

use std::time::Duration;

fn get_default_tag(file_type: FileType) -> Tag {
    let tag_type = file_type.primary_tag_type();
    Tag::new(tag_type)
}

pub fn read_metadata(path: &Path) -> Result<SongMetadata, Box<dyn std::error::Error>> {
    let tagged_file = Probe::open(path)?.read()?;

    let file_type = tagged_file.file_type();
    let default_tag = get_default_tag(file_type);

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
        .unwrap_or(&default_tag);

    let properties = tagged_file.properties();

    let bit_rate = properties.audio_bitrate().or_else(|| properties.overall_bitrate());

    let metadata = SongMetadata {
        title: tag.title().map(|s| s.to_string()),
        artist: tag.artist().map(|s| s.to_string()),
        album: tag.album().map(|s| s.to_string()),
        duration: Some(Duration::from_secs(properties.duration().as_secs())),
        year: tag.year(),
        track_number: tag.track(),
        format: format_to_string(file_type),
        bit_rate,
    };

    Ok(metadata)
}

pub fn print_song_info(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let metadata = read_metadata(path)?;
    let mut return_duration: u64 = 0;

    println!("\n=== Song Information ===");
    println!("Title: {}", metadata.title.as_deref().unwrap_or("Unknown"));
    println!("Artist: {}", metadata.artist.as_deref().unwrap_or("Unknown"));
    println!("Album: {}", metadata.album.as_deref().unwrap_or("Unknown"));
    println!("Format: {}", metadata.format);

    if let Some(bit_rate) = metadata.bit_rate {
        println!("Bit Rate: {}", format_bitrate(bit_rate));
    }

    if let Some(duration) = metadata.duration {
        println!("Duration: {}", format_duration(duration));
        return_duration = duration.as_secs();
    }

    if let Some(year) = metadata.year {
        println!("Year: {}", year);
    }

    if let Some(track) = metadata.track_number {
        println!("Track Number: {}", track);
    }

    Ok(return_duration)
}