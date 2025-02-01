pub use rodio::{Source, Sample, Decoder};
use std::{fs::File, io::BufReader, path::Path, time::Duration};
use anyhow::{Result, anyhow};

const I16_TO_F32_NORM_FACTOR: f32 = i16::MAX as f32;


pub struct RodioDecoder {
    decoder: Decoder<BufReader<File>>
}

impl RodioDecoder {
    pub fn load(path: &Path) -> Result<Self> {
        let file = BufReader::new(File::open(path)?);
        let decoder = Decoder::new(file)
            .map_err(|e| anyhow!("Rodio decoder error: {:?}", e))?;

        Ok(Self {
            decoder
        })

    }
}

impl Iterator for RodioDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.next().map(|sample| sample as f32 / I16_TO_F32_NORM_FACTOR)
    }
}

impl Source for RodioDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        self.decoder.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.decoder.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.decoder.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.decoder.total_duration()
    }
}