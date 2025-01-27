use std::{collections::VecDeque, fs::File, io::BufReader, path::Path, time::Duration};
use rodio::{Decoder, Sample, Source};
use anyhow::{Result, anyhow};
use ogg::reading::PacketReader;
use opus::Decoder as OpusDecoder;
use lewton::inside_ogg::OggStreamReader;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::{format, frame, codec};
use std::sync::{Arc, Mutex};
use alac;

/// Normalization factor to convert 16-bit audio samples (-32768 to +32767) to float (-1.0 to +1.0)
const I16_TO_F32_NORM_FACTOR: f32 = 32768.0;

/// Normalization factor to convert 32-bit interger audio samples to float32
const I32_TO_F32_NORM_FACTOR: f32 = 2147483648.0;

/// Opus buffer size of 60ms
const OPUS_BUFFER_SIZE: usize = 2880;

/// Pre-initialized VecDeque Buffer size
const INITIAL_BUFFER_CAPACITY: usize = 4096;

pub struct ThreadSafeFFmpeg {
    decoder: Mutex<codec::decoder::Audio>,
    context: Arc<Mutex<format::context::Input>>,
    frame: Mutex<frame::Audio>,
    sample_buffer: Mutex<VecDeque<f32>>,
}

unsafe impl Send for ThreadSafeFFmpeg {}
unsafe impl Sync for ThreadSafeFFmpeg {}

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
    },
    Alac {
        packets: alac::Packets<BufReader<File>, i32>,
        buffer: VecDeque<f32>,
        config: alac::StreamInfo,
    },
    FFmpeg (Arc<ThreadSafeFFmpeg>),

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
            },
            AudioDecoder::Alac { packets, buffer, config } => {
                if let Some(sample) = buffer.pop_front() {
                    return Some(sample);
                }
            
                let max_samples = config.max_samples_per_packet() as usize * config.channels() as usize;
                let mut output = vec![0i32; max_samples];
            
                match packets.next_into(&mut output) {
                    Ok(Some(decoded)) => {
                        // If buffer is empty, analyze the first packet to determine actual bit depth
                        if buffer.is_empty() {
                            // Find the maximum absolute value in the first packet
                            let max_abs = decoded.iter()
                                .map(|&s| s.abs())
                                .max()
                                .unwrap_or(0);
            
                            // Determine bit depth based on the maximum value
                            let (shift, norm_factor) = if max_abs > 0 {
                                let bits_needed = 32 - max_abs.leading_zeros();
                                
                                match bits_needed {
                                    0..=16 => (0, I16_TO_F32_NORM_FACTOR), // 16-bit
                                    17..=24 => (8, 8388608.0), // 24-bit shifted left by 8
                                    _ => (0, I32_TO_F32_NORM_FACTOR) // 32-bit
                                }
                            } else {
                                (0, I32_TO_F32_NORM_FACTOR) // Default to 32-bit if no signal
                            };
            
                            // Convert samples to f32 and add to buffer
                            for &sample in decoded {
                                let shifted = if shift > 0 { sample >> shift } else { sample };
                                let normalized = (shifted as f32) / norm_factor;
                                let clamped = normalized.clamp(-1.0, 1.0);
                                buffer.push_back(clamped);
                            }
                        } else {
                            // Use the same normalization for subsequent packets
                            let (shift, norm_factor) = match config.bit_depth() {
                                16 => (0, I16_TO_F32_NORM_FACTOR),
                                24 => (8, 8388608.0), // 24-bit shifted left by 8
                                _ => (0, I32_TO_F32_NORM_FACTOR)
                            };
            
                            for &sample in decoded {
                                let shifted = if shift > 0 { sample >> shift } else { sample };
                                let normalized = (shifted as f32) / norm_factor;
                                let clamped = normalized.clamp(-1.0, 1.0);
                                buffer.push_back(clamped);
                            }
                        }
                        buffer.pop_front()
                    }
                    Ok(None) => None, // End of stream
                    Err(e) => {
                        println!("Error decoding ALAC packet: {:?}", e);
                        None
                    }
                }
            }
            AudioDecoder::FFmpeg(ffmpeg) => {
                let mut buffer = ffmpeg.sample_buffer.lock().unwrap();
                if let Some(sample) = buffer.pop_front() {
                    return Some(sample);
                }
            
                let mut decoder = ffmpeg.decoder.lock().unwrap();
                let mut frame = ffmpeg.frame.lock().unwrap();
                let mut context = ffmpeg.context.lock().unwrap();
            
                while buffer.is_empty() {
                    match decoder.receive_frame(&mut frame) {
                        Ok(_) => {
                            let nb_samples = frame.samples();
                            let nb_channels = frame.channels() as usize;
                            
                            match frame.format() {
                                format::Sample::F32(layout) => {
                                    if layout == format::sample::Type::Planar {
                                        // Handle planar format (channels are separate)
                                        for i in 0..nb_samples {
                                            for c in 0..nb_channels {
                                                let sample = frame.plane::<f32>(c)[i];
                                                buffer.push_back(sample);
                                            }
                                        }
                                    } else {
                                        // Handle packed format (channels are interleaved)
                                        for &sample in frame.plane::<f32>(0).iter().take(nb_samples * nb_channels) {
                                            buffer.push_back(sample);
                                        }
                                    }
                                }
                                format::Sample::I16(layout) => {
                                    if layout == format::sample::Type::Planar {
                                        for i in 0..nb_samples {
                                            for c in 0..nb_channels {
                                                let sample = frame.plane::<i16>(c)[i] as f32 / I16_TO_F32_NORM_FACTOR;
                                                buffer.push_back(sample);
                                            }
                                        }
                                    } else {
                                        for &sample in frame.plane::<i16>(0).iter().take(nb_samples * nb_channels) {
                                            buffer.push_back(sample as f32 / I16_TO_F32_NORM_FACTOR);
                                        }
                                    }
                                }
                                format::Sample::I32(layout) => {
                                    if layout == format::sample::Type::Planar {
                                        for i in 0..nb_samples {
                                            for c in 0..nb_channels {
                                                let sample = frame.plane::<i32>(c)[i] as f32 / I32_TO_F32_NORM_FACTOR;
                                                buffer.push_back(sample);
                                            }
                                        }
                                    } else {
                                        for &sample in frame.plane::<i32>(0).iter().take(nb_samples * nb_channels) {
                                            buffer.push_back(sample as f32 / I32_TO_F32_NORM_FACTOR);
                                        }
                                    }
                                }
                                other => {
                                    println!("Unsupported sample format: {:?}", other);
                                    return None;
                                }
                            }
                        }
                        Err(ffmpeg::Error::Other { errno: ffmpeg::error::EAGAIN }) => {
                            // Need more packets
                            let stream_index = context.streams().best(ffmpeg::media::Type::Audio)
                                .map(|s| s.index())
                                .unwrap_or(0);
            
                            match context.packets().next() {
                                Some((stream, packet)) if stream.index() == stream_index => {
                                    if decoder.send_packet(&packet).is_err() {
                                        println!("Error sending packet to decoder");
                                        return None;
                                    }
                                }
                                Some(_) => continue, // wrong stream, skip
                                None => {
                                    println!("End of stream reached");
                                    return None;
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error receiving frame: {:?}", e);
                            return None;
                        }
                    }
                }
            
                buffer.pop_front()
            }
        }

    }
}

impl Source for AudioDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.current_frame_len(),
            AudioDecoder::Opus { .. } => Some(960), // 20ms frame size for Opus
            AudioDecoder::FFmpeg(_) => None,
            _ => None, // Variable frame size
        }
    }

    fn channels(&self) -> u16 {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.channels(),
            AudioDecoder::Opus { .. } => 2, // Opus decoder is configured for stereo
            AudioDecoder::Vorbis { decoder, .. } => decoder.ident_hdr.audio_channels as u16,
            AudioDecoder::Alac { config, .. } => config.channels() as u16,
            AudioDecoder::FFmpeg(ffmpeg) => ffmpeg.decoder.lock().unwrap().channels(),
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            AudioDecoder::Rodio(decoder) => decoder.sample_rate(),
            AudioDecoder::Opus { .. } => 48000, // Opus always uses 48kHz
            AudioDecoder::Vorbis { decoder, .. } => decoder.ident_hdr.audio_sample_rate,
            AudioDecoder::Alac {  config, .. } => config.sample_rate(),
            AudioDecoder::FFmpeg (ffmpeg) => ffmpeg.decoder.lock().unwrap().rate(),
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
        Some("m4a") => {
            match load_alac_file(path) {
                Ok(decoder) => {
                    Ok(decoder)
                },
                Err(_) => {
                    match load_opus_file(path) {
                        Ok(decoder) => {
                            Ok(decoder)
                        },
                        Err(_) => {
                            let file = BufReader::new(File::open(path)?);
                            match Decoder::new(file) {
                                Ok(decoder) => {
                                    Ok(AudioDecoder::Rodio(decoder))
                                },
                                Err(_) => {
                                    load_ffmpeg_file(path)
                                }
                            }
                        }
                    }
                }
            }
        }
        Some("opus") => load_opus_file(path),
        Some("ogg") => {
            // For Vorbis files, we'll use rodio's native decoder
            match OggStreamReader::new(file) {
                Ok(decoder) => Ok(AudioDecoder::Vorbis{
                    decoder: Box::new(decoder),
                    sample_buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
                }),
                Err(_) => load_ffmpeg_file(path) // Fallback to FFmpeg
            }
        }
        _ => {
            match Decoder::new(file) {
                Ok(decoder) => Ok(AudioDecoder::Rodio(decoder)),
                Err(_) => load_ffmpeg_file(path)
            }
        }

    } 
}

fn load_opus_file(path: &Path) -> Result<AudioDecoder> {
    let file = BufReader::new(File::open(path)?);
    let mut packet_reader = PacketReader::new(file);
    
    // Read and verify Opus header
    let _header = packet_reader.read_packet()?.ok_or_else(|| anyhow!("Missing Opus header"))?;
    let _comments = packet_reader.read_packet()?.ok_or_else(|| anyhow!("Missing Opus comments"))?;

    let opus_decoder = OpusDecoder::new(48000, opus::Channels::Stereo)?;

    Ok(AudioDecoder::Opus {
        decoder: opus_decoder,
        packet_reader,
        sample_buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
    })
}


fn load_alac_file(path: &Path) -> Result<AudioDecoder> {
    
    let file = BufReader::new(File::open(path)?);

    let reader = alac::Reader::new(file)
        .map_err(|e| anyhow!("Failed to create ALAC reader: {:?}", e))?;
    
    let stream_info = reader.stream_info().clone();
    
    // Convert reader into packets
    let packets = reader.into_packets();
    
    Ok(AudioDecoder::Alac {
        packets,  // Store packets instead of reader
        buffer: VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY),
        config: stream_info,
    })
}

fn load_ffmpeg_file(path: &Path) -> Result<AudioDecoder> {
    ffmpeg::init().map_err(|e| anyhow!("Failed to initialize FFmpeg: {}", e))?;

    let input = format::input(&path)?;

    let stream = input
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .ok_or_else(|| anyhow!("No audio stream found"))?;

    let decoder = codec::Context::from_parameters(stream.parameters())?
        .decoder()
        .audio()?;
    // Print debug info
    println!("\r\nPlaying song with FFmpeg\r");

    // Create FFmpeg wrapper
    let ffmpeg = ThreadSafeFFmpeg {
        decoder: Mutex::new(decoder),
        context: Arc::new(Mutex::new(input)),
        frame: Mutex::new(frame::Audio::empty()),
        sample_buffer: Mutex::new(VecDeque::with_capacity(INITIAL_BUFFER_CAPACITY)),
    };

    Ok(AudioDecoder::FFmpeg(Arc::new(ffmpeg)))
}

pub struct SkipDuration<S> {
    source: S,
    samples_to_skip: usize,
    samples_skipped: usize,
}

impl<S> SkipDuration<S>
where
    S: Source,
    S::Item: Sample,
{
    fn new(source: S, duration: Duration) -> Self {
        let samples_to_skip = (duration.as_secs_f32() * source.sample_rate() as f32) as usize
            * source.channels() as usize;
        Self {
            source,
            samples_to_skip,
            samples_skipped: 0,
        }
    }
}

impl<S> Iterator for SkipDuration<S>
where
    S: Source,
    S::Item: Sample,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        while self.samples_skipped < self.samples_to_skip {
            self.source.next()?;
            self.samples_skipped += 1;
        }
        self.source.next()
    }
}

impl<S> Source for SkipDuration<S>
where
    S: Source,
    S::Item: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

// Add this method to AudioDecoder
impl AudioDecoder {
    pub fn skip_duration(self, duration: Duration) -> SkipDuration<Self> {
        SkipDuration::new(self, duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_audio_file() {
        let test_files = vec![
            ("test.mp3", true),
            ("test.opus", true),
            ("test.ogg", true),
            ("test.aac", true),
            ("test.flac", true),
            ("test.m4a", true),
            ("test.wav", true),
            ("invalid.xyz", false),
        ];

        for (file, should_succeed) in test_files {
            let path = PathBuf::from(format!("tests/resources/{}", file));
            let result = load_audio_file(&path);
            
            if should_succeed {
                assert!(result.is_ok(), "Failed to load valid audio file: {}", file);
            } else {
                assert!(result.is_err(), "Should fail for invalid file: {}", file);
            }
        }
    }

    #[test]
    fn test_skip_duration() {
        let test_file = PathBuf::from("tests/resources/test.mp3");
        let decoder = load_audio_file(&test_file).unwrap();
        
        let skip_duration = Duration::from_secs(1);
        let skipped_source = decoder.skip_duration(skip_duration);
        
        // Verify sample rate and channels remain unchanged
        assert_eq!(skipped_source.sample_rate(), 44100); // Adjust based on your test file
        assert_eq!(skipped_source.channels(), 2);        // Adjust based on your test file
    }
}