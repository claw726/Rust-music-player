use std::{fs::File, io::BufReader, path::Path, time::Duration, collections::VecDeque};
use rodio::{Decoder, Source};
use anyhow::{Result, anyhow};
use ogg::reading::PacketReader;
use opus::Decoder as OpusDecoder;
use lewton::inside_ogg::OggStreamReader;

/// Normalization factor to convert 16-bit audio samples (-32768 to +32767) to float (-1.0 to +1.0)
const I16_TO_F32_NORM_FACTOR: f32 = 32768.0;

/// Opus buffer size of 60ms
const OPUS_BUFFER_SIZE: usize = 2880;

/// Pre-initialized VecDeque Buffer size
const INITIAL_BUFFER_CAPACITY: usize = 4096;

pub enum AudioDecoder {
    Rodio(Decoder<BufReader<File>>),
    Opus {
        decoder: OpusDecoder,
        packet_reader: PacketReader<BufReader<File>>,
        sample_buffer: VecDeque<f32>,
    },
    Vorbis {
        decoder: Box<OggStreamReader<BufReader<File>>>,
        sample_buffer: VecDeque<f32>,
    }
}

impl Iterator for AudioDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            AudioDecoder::Rodio(decoder) => {
                decoder.next().map(|sample| sample as f32 / I16_TO_F32_NORM_FACTOR)
            }
            AudioDecoder::Opus { decoder, packet_reader, sample_buffer } => {
                // Return buffer sample if available
                if let Some(sample) = sample_buffer.pop_front() {
                    return Some(sample);
                }
                
                // Read and decode the next packet
                while sample_buffer.is_empty() {
                    match packet_reader.read_packet() {
                        Ok(Some(packet)) => {
                            let mut output_buffer = vec![0.0f32; OPUS_BUFFER_SIZE]; // Max frame size for 120ms
                            if let Ok(decoded_samples) = decoder.decode_float(&packet.data, &mut output_buffer, false) {
                                sample_buffer.extend(output_buffer.into_iter().take(decoded_samples * 2));
                            } 
                        }
                        _ => return None, // End of stream error
                    }
                }

                sample_buffer.pop_front()
            },
            AudioDecoder::Vorbis { decoder, sample_buffer} => {
                if let Some(sample) = sample_buffer.pop_front() {
                    return Some(sample);
                }

                while sample_buffer.is_empty() {
                    match decoder.read_dec_packet_itl() {
                        Ok(Some(pck_samples)) => {
                            for sample in pck_samples {
                                sample_buffer.push_back(sample as f32 / I16_TO_F32_NORM_FACTOR);
                            }
                        }
                        Ok(None) => return None, // End of stream
                        Err(_) => return None,   // Error reading packet
                    }
                }
                    

                sample_buffer.pop_front()
            }
        }

    }
}

impl Source for AudioDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.current_frame_len(),
            AudioDecoder::Opus { .. } => Some(960), // 20ms frame size for Opus
            AudioDecoder::Vorbis { .. } => None, // Variable frame size
        }
    }

    fn channels(&self) -> u16 {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.channels(),
            AudioDecoder::Opus { .. } => 2, // Opus decoder is configured for stereo
            AudioDecoder::Vorbis { decoder, .. } => decoder.ident_hdr.audio_channels as u16,
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.sample_rate(),
            AudioDecoder::Opus { .. } => 48000, // Opus always uses 48kHz
            AudioDecoder::Vorbis { decoder, .. } => decoder.ident_hdr.audio_sample_rate,
        }
    }

    fn total_duration(&self) -> Option<Duration> {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.total_duration(),
            _ => None, // Would need to calculate from file size
        }
    }
}

pub fn load_audio_file(path: &Path) -> Result<AudioDecoder> {

    let file = BufReader::new(File::open(path)?);
    
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    match extension.as_deref() {
        Some("opus") => load_opus_file(path),
        Some("ogg") => {
            // For Vorbis files, we'll use rodio's native decoder
            match OggStreamReader::new(file) {
                Ok(decoder) => Ok(AudioDecoder::Vorbis{
                    decoder: Box::new(decoder),
                    sample_buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
                }),
                Err(_) => Err(anyhow!("Failed to decode Vorbis file"))
            }
        }
        _ => {
            // Try regular rodio decoder for other formats
            match Decoder::new(file) {
                Ok(decoder) => Ok(AudioDecoder::Rodio(decoder)),
                Err(_) => Err(anyhow!("Unsupported audio format"))
            }
        }
    } 
}

fn load_opus_file(path: &Path) -> Result<AudioDecoder> {
    let file = BufReader::new(File::open(path)?);
    let mut packet_reader = PacketReader::new(file);
    
    // Read and verify Opus header
    let _header_packet = packet_reader.read_packet()?
        .ok_or_else(|| anyhow!("Missing Opus header"))?;
    
    // Read and verify Opus comments
    let _comments_packet = packet_reader.read_packet()?
        .ok_or_else(|| anyhow!("Missing Opus comments"))?;

    let opus_decoder = OpusDecoder::new(48000, opus::Channels::Stereo)?;

    Ok(AudioDecoder::Opus {
        decoder: opus_decoder,
        packet_reader,
        sample_buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
    })
}