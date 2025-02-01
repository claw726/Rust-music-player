use std::{collections::VecDeque, fs::File, io::BufReader, path::Path, time::Duration};
use lewton::inside_ogg::OggStreamReader;
use anyhow::{anyhow, Result};

const INITIAL_BUFFER_CAPACITY: usize = 4096;
const I16_TO_F32_NORM_FACTOR: f32 = i16::MAX as f32;

pub struct VorbisDecoder {
    decoder: OggStreamReader<BufReader<File>>,
    sample_buffer: VecDeque<f32>,
}

impl VorbisDecoder {
    pub fn load(path: &Path) -> Result<Self> {
        let file = BufReader::new(File::open(path)?);
        let decoder = OggStreamReader::new(file)
            .map_err(|e| anyhow!("Vorbis decoding error: {:?}", e))?;

        Ok(Self {
            decoder,
            sample_buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
        })
    }
}

impl Iterator for VorbisDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.sample_buffer.pop_front() {
            return Some(sample);
        }

        while self.sample_buffer.is_empty() {
            match self.decoder.read_dec_packet_itl() {
                Ok(Some(pck_samples)) => {
                    for sample in pck_samples {
                        self.sample_buffer.push_back(sample as f32 / I16_TO_F32_NORM_FACTOR);
                    }
                }
                Ok(None) => return None, // End of stream
                Err(e) => {
                    eprintln!("Vorbis decoding error: {:?}", e);
                    return None;
                }
            }
        }

        self.sample_buffer.pop_front()
    }
}

impl rodio::Source for VorbisDecoder {

    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.decoder.ident_hdr.audio_channels as u16
    }

    fn sample_rate(&self) -> u32 {
        self.decoder.ident_hdr.audio_sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

}