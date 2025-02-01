use std::{collections::VecDeque, fs::File, io::BufReader, path::Path};
use alac::{Packets, StreamInfo};
use anyhow::{Result, anyhow};
use rodio::Source;

const INITIAL_BUFFER_CAPACITY: usize = 4096;
const I16_TO_F32_NORM_FACTOR: f32 = 32768.0;
const I32_TO_F32_NORM_FACTOR: f32 = 2147483648.0;

pub struct AlacDecoder {
    packets: Packets<BufReader<File>, i32>,
    buffer: VecDeque<f32>,
    config: StreamInfo,
    current_shift: u32,
    current_norm_factor: f32,
}

impl AlacDecoder {
    pub fn load(path: &Path) -> Result<Self> {
        let file = BufReader::new(File::open(path)?);
        let reader = alac::Reader::new(file)
            .map_err(|e| anyhow!("Failed to create ALAC reader: {:?}", e))?;

        let stream_info = reader.stream_info().clone();
        let packets = reader.into_packets();

        Ok(Self {
            packets,
            buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
            config: stream_info,
            current_shift: 0,
            current_norm_factor: I32_TO_F32_NORM_FACTOR,
        })
    }

    fn determine_normalization(&mut self, decoded: &[i32]) {
        let max_abs = decoded.iter()
            .map(|&s| s.abs())
            .max()
            .unwrap_or(0);

        let (shift, norm_factor) = if max_abs > 0 {
            let bits_needed = 32 - max_abs.leading_zeros();

            match bits_needed {
                0..=16 => (0, I16_TO_F32_NORM_FACTOR),
                17..=24 => (8, 8388608.0),
                _ => (0, I32_TO_F32_NORM_FACTOR)
            }
        } else {
            (0, I32_TO_F32_NORM_FACTOR)
        };

        self.current_shift = shift;
        self.current_norm_factor = norm_factor;
    }
}

impl Iterator for AlacDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.buffer.pop_front() {
            return Some(sample);
        }

        let max_samples = self.config.max_samples_per_packet() as usize
            * self.config.channels() as usize;
        let mut output = vec![0i32; max_samples];

        // Fixed: Properly handle the returned slice
        match self.packets.next_into(&mut output) {
            Ok(Some(decoded)) => {
                if self.buffer.is_empty() {
                    self.determine_normalization(decoded);
                }

                for &sample in decoded {
                    let shifted = if self.current_shift > 0 {
                        sample >> self.current_shift
                    } else {
                        sample
                    };

                    let normalized = (shifted as f32) / self.current_norm_factor;
                    let clamped = normalized.clamp(-1.0, 1.0);
                    self.buffer.push_back(clamped);
                }

                self.buffer.pop_front()
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!("ALAC decoding error: {:?}", e);
                None
            }
        }
    }
}

impl Source for AlacDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.config.channels() as u16
    }

    fn sample_rate(&self) -> u32 {
        self.config.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}