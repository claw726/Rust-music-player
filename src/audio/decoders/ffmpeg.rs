use std::{
    path::Path,
    sync::{Arc, Mutex},
    collections::VecDeque,
};
use std::time::Duration;
use ffmpeg_next::{format, frame, codec, error, util::log::level};
use anyhow::{Result, anyhow};
use rodio::Source;

const INITIAL_BUFFER_CAPACITY: usize = 4096;
const I16_TO_F32_NORM_FACTOR: f32 = 32768.0;
const I32_TO_F32_NORM_FACTOR: f32 = 2147483648.0;

#[derive(Clone)]
pub struct SharedFFmpegDecoder(Arc<Mutex<FFmpegDecoder>>);

impl SharedFFmpegDecoder {
    pub fn new(decoder: FFmpegDecoder) -> Self {
        Self(Arc::new(Mutex::new(decoder)))
    }
}
pub struct FFmpegDecoder {
    decoder: Mutex<codec::decoder::Audio>,
    context: Arc<Mutex<format::context::Input>>,
    frame: Mutex<frame::Audio>,
    sample_buffer: Mutex<VecDeque<f32>>,
}

unsafe impl Send for FFmpegDecoder {}
unsafe impl Sync for FFmpegDecoder {}

impl FFmpegDecoder {
    pub fn load(path: &Path) -> Result<Self> {
        ffmpeg_next::init()
            .map_err(|err| anyhow!("{}", err))?;
        ffmpeg_next::util::log::set_level(level::Level::Warning);

        let input = format::input(path)
            .map_err(|e| anyhow!("FFmpeg input error: {}", e))?;
        let stream = input.streams()
            .best(ffmpeg_next::media::Type::Audio)
            .ok_or_else(|| anyhow!("No audio stream found"))?;

        let mut decoder = codec::Context::from_parameters(stream.parameters())
            .map_err(|e| anyhow!("Codec context error: {}", e))?
            .decoder()
            .audio()
            .map_err(|e| anyhow!("Audio decoder error: {}", e))?;

        decoder.set_parameters(stream.parameters())
            .map_err(|e| anyhow!("Parameter error: {}", e))?;

        Ok(Self {
            decoder: Mutex::new(decoder),
            context: Arc::new(Mutex::new(input)),
            frame: Mutex::new(frame::Audio::empty()),
            sample_buffer: Mutex::new(VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY)),
        })
    }

    pub fn into_shared(self) -> SharedFFmpegDecoder {
        SharedFFmpegDecoder::new(self)
    }

    fn decode_frame(&self) -> Result<()> {
        let mut decoder = self.decoder.lock().unwrap();
        let mut frame = self.frame.lock().unwrap();
        let mut buffer = self.sample_buffer.lock().unwrap();

        loop {
            match decoder.receive_frame(&mut frame) {
                Ok(_) => {
                    let samples = frame.samples();
                    let channels = frame.channels() as usize;

                    match frame.format() {
                        format::Sample::F32(layout) => self.process_f32_frame(&frame, samples, channels, layout, &mut buffer),
                        format::Sample::I16(layout) => self.process_i16_frame(&frame, samples, channels, layout, &mut buffer),
                        format::Sample::I32(layout) => self.process_i32_frame(&frame, samples, channels, layout, &mut buffer),
                        other => return Err(anyhow!("Unsupported sample format: {:?}", other)),
                    }
                    break Ok(());
                }
                Err(error::Error::Other { errno: error::EAGAIN }) => {
                    self.feed_packets()?;
                }
                Err(e) => return Err(anyhow!("Frame error: {}", e)),
            }
        }
    }

    fn process_f32_frame(&self, frame: &frame::Audio, samples: usize, channels: usize, layout: format::sample::Type, buffer: &mut VecDeque<f32>) {
        if layout == format::sample::Type::Planar {
            for i in 0..samples {
                for c in 0..channels {
                    buffer.push_back(frame.plane::<f32>(c)[i]);
                }
            }
        } else {
            buffer.extend(frame.plane::<f32>(0).iter().take(samples * channels).copied());
        }
    }

    fn process_i16_frame(&self, frame: &frame::Audio, samples: usize, channels: usize, layout: format::sample::Type, buffer: &mut VecDeque<f32>) {
        if layout == format::sample::Type::Planar {
            for i in 0..samples {
                for c in 0..channels {
                    buffer.push_back(frame.plane::<i16>(c)[i] as f32 / I16_TO_F32_NORM_FACTOR);
                }
            }
        } else {
            buffer.extend(
                frame.plane::<i16>(0)
                    .iter()
                    .take(samples * channels)
                    .map(|&s| s as f32 / I16_TO_F32_NORM_FACTOR)
            );
        }
    }

    fn process_i32_frame(&self, frame: &frame::Audio, samples: usize, channels: usize, layout: format::sample::Type, buffer: &mut VecDeque<f32>) {
        if layout == format::sample::Type::Planar {
            for i in 0..samples {
                for c in 0..channels {
                    buffer.push_back(frame.plane::<i32>(c)[i] as f32 / I32_TO_F32_NORM_FACTOR);
                }
            }
        } else {
            buffer.extend(
                frame.plane::<i32>(0)
                    .iter()
                    .take(samples * channels)
                    .map(|&s| s as f32 / I32_TO_F32_NORM_FACTOR)
            );
        }
    }

    fn feed_packets(&self) -> Result<()> {
        let mut context = self.context.lock().unwrap();
        let mut decoder = self.decoder.lock().unwrap();

        let stream_index = context.streams()
            .best(ffmpeg_next::media::Type::Audio)
            .map(|s| s.index())
            .unwrap_or(0);

        if let Some((stream, packet)) = context.packets().next() {
            if stream.index() == stream_index {
                decoder.send_packet(&packet)
                    .map_err(|e| anyhow!("Packet error: {}", e))?;
            }
        } else {
            decoder.send_eof()
                .map_err(|e| anyhow!("EOF error: {}", e))?;
        }

        Ok(())
    }
}

impl Iterator for FFmpegDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = self.sample_buffer.lock().unwrap();

        if buffer.is_empty() {
            if let Err(e) = self.decode_frame() {
                eprintln!("Decoding error: {}", e);
                return None;
            }
        }

        buffer.pop_front()
    }
}

impl Source for FFmpegDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None // Variable frame size
    }

    fn channels(&self) -> u16 {
        self.decoder.lock().unwrap().channels()
    }

    fn sample_rate(&self) -> u32 {
        self.decoder.lock().unwrap().rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Would require container duration calculation
    }
}

impl Iterator for SharedFFmpegDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.lock().unwrap().next()
    }
}

impl Source for SharedFFmpegDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        self.0.lock().unwrap().current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.0.lock().unwrap().channels()
    }

    fn sample_rate(&self) -> u32 {
        self.0.lock().unwrap().sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.0.lock().unwrap().total_duration()
    }
}