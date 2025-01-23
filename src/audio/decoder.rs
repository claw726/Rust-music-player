use std::{fs::File, io::BufReader, path::Path, time::Duration, collections::VecDeque};
use rodio::{Decoder, Source};
use anyhow::{Result, anyhow};
use ogg::reading::PacketReader;
use opus::Decoder as OpusDecoder;

pub enum AudioDecoder {
    Rodio(Decoder<BufReader<File>>),
    Opus {
        decoder: OpusDecoder,
        packet_reader: PacketReader<BufReader<File>>,
        sample_buffer: VecDeque<f32>,
    },
}

impl Iterator for AudioDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            AudioDecoder::Rodio(decoder) => {
                decoder.next().map(|sample| sample as f32 / 32768.0)
            },
            AudioDecoder::Opus { decoder, packet_reader, sample_buffer } => {
                // If buffer is empty, decode next packet
                if sample_buffer.is_empty() {
                    if let Ok(Some(packet)) = packet_reader.read_packet() {
                        // Get number of samples in this packet
                        if let Ok(nb_samples) = decoder.get_nb_samples(&packet.data) {
                            let mut output_buffer = vec![0.0f32; nb_samples * 2]; // * 2 for stereo
                            if let Ok(decoded_samples) = decoder.decode_float(&packet.data, &mut output_buffer, false) {
                                // Push all decoded samples to the buffer
                                sample_buffer.extend(output_buffer.into_iter().take(decoded_samples * 2));
                            }
                        }
                    }
                }
                
                // Return next sample from buffer
                sample_buffer.pop_front()
            },
        }
        
    }
}

impl Source for AudioDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.current_frame_len(),
            AudioDecoder::Opus { .. } => Some(960), // 20ms frame size for Opus
        }
    }

    fn channels(&self) -> u16 {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.channels(),
            AudioDecoder::Opus { .. } => 2, // Opus decoder is configured for stereo
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.sample_rate(),
            AudioDecoder::Opus { .. } => 48000, // Opus always uses 48kHz
        }
    }

    fn total_duration(&self) -> Option<Duration> {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.total_duration(),
            _ => None, // Would need to calculate from file size
        }
    }
}

pub fn load_sound_file(path: &Path) -> Result<AudioDecoder> {

    // For other formats, try Rodio first
    if let Ok(decoder) = Decoder::new(BufReader::new(File::open(path)?)) {
        return Ok(AudioDecoder::Rodio(decoder));
    }

    // Try Opus for .opus files
    if path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("opus"))
        .unwrap_or(false)
    {
        return load_opus_file(path);
    }

    Err(anyhow!("Unsupported audio format.\nProgram only supports MP3, Vorbis, Opus, WAV, and FLAC files."))
}

fn load_opus_file(path: &Path) -> Result<AudioDecoder> {
    let file = BufReader::new(File::open(path)?);
    let mut packet_reader = PacketReader::new(file);
    
    // Read and verify Opus header
    let _header_packet = packet_reader.read_packet()?
        .ok_or_else(|| anyhow!("Failed to read Opus header"))?;
    
    // Read and verify Opus comments
    let _comments_packet = packet_reader.read_packet()?
        .ok_or_else(|| anyhow!("Failed to read Opus comments"))?;

    let opus_decoder = OpusDecoder::new(48000, opus::Channels::Stereo)?;

    Ok(AudioDecoder::Opus {
        decoder: opus_decoder,
        packet_reader,
        sample_buffer: VecDeque::new(),
    })
}