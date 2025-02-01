use std::{collections::VecDeque, fs::File, io::BufReader, path::Path, time::Duration};
use anyhow::{Result, anyhow};
use ogg::reading::PacketReader;
use opus::Decoder as OpusDecoder;

const INITIAL_BUFFER_CAPACITY: usize = 4096;
const OPUS_BUFFER_SIZE: usize = 2880;

pub struct DecoderOpus {
    decoder: OpusDecoder,
    packet_reader: PacketReader<BufReader<File>>,
    sample_buffer: VecDeque<f32>,
}

impl DecoderOpus {
    pub fn load(path: &Path) -> Result<Self> {
        let file = BufReader::new(File::open(path)?);
        let mut packet_reader = PacketReader::new(file);

        let _header = packet_reader.read_packet()?
            .ok_or_else(|| anyhow!("Missing Opus header"))?;

        let _comments = packet_reader.read_packet()?
            .ok_or_else(|| anyhow!("Missing Opus comments"))?;

        Ok(Self {
            decoder: OpusDecoder::new(48000, opus::Channels::Stereo)?,
            packet_reader,
            sample_buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
        })
    }   
}

impl Iterator for DecoderOpus {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.sample_buffer.pop_front() {
            return Some(sample);
        }
        
        // Read and decode the next packet
        while self.sample_buffer.is_empty() {
            match self.packet_reader.read_packet() {
                Ok(Some(packet)) => {
                    let mut output_buffer = vec![0.0f32; OPUS_BUFFER_SIZE]; // Max frame size for 120ms
                    if let Ok(decoded_samples) = self.decoder.decode_float(&packet.data, &mut output_buffer, false) {
                        self.sample_buffer.extend(output_buffer.into_iter().take(decoded_samples * 2));
                    } 
                }
                _ => return None, // End of stream error
            }
        }
    
        self.sample_buffer.pop_front()
    }
}

impl rodio::Source for DecoderOpus {
    fn current_frame_len(&self) -> Option<usize> {
        Some(960)
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        48000
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}