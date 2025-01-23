use rodio::{Decoder, OutputStream, Sink, Source};
use anyhow::Result;
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    time::{Duration, Instant},
};

use super::display::DisplayThread;
use super::utils::{TimeFormat, TimeUtils};
use std::io::{stdout, Write};

pub struct AudioPlayer {
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    sink: Arc<Sink>,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    current_position: Arc<Mutex<u64>>,
    file_path: Option<PathBuf>,
    total_duration: Option<Duration>,
    display_thread: Option<DisplayThread>,
    playback_start: Arc<Mutex<Option<Instant>>>,
    pause_start: Arc<Mutex<Option<Instant>>>,
    total_pause_duration: Arc<Mutex<Duration>>,
    metadata_duration: Option<Duration>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        Ok(Self { 
            _stream, 
            stream_handle,
            sink: Arc::new(sink),
            is_playing: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            current_position: Arc::new(Mutex::new(0)),
            file_path: None,
            metadata_duration: None,
            total_duration: None,
            display_thread: None,
            playback_start: Arc::new(Mutex::new(None)),
            pause_start: Arc::new(Mutex::new(None)),
            total_pause_duration: Arc::new(Mutex::new(Duration::from_secs(0))),
        })
    }

    pub fn set_metadata_duration(&mut self, duration_seconds: u64) {
        self.metadata_duration = Some(Duration::from_secs(duration_seconds));
        // Also set total_duration if it's not available from the decoder
        if self.total_duration.is_none() {
            self.total_duration = self.metadata_duration;
        }
    }

    pub fn play<P: AsRef<Path>>(&mut self, source: Decoder<BufReader<File>>, path: P) {
        // Stop any existing display thread
        if let Some(mut display_thread) = self.display_thread.take() {
            display_thread.stop();
        }

        self.file_path = Some(path.as_ref().to_path_buf());
        // Try to get duration from decoder first, fall back to metadata duration
        self.total_duration = source.total_duration().or(self.metadata_duration);

        let new_sink = Sink::try_new(&self.stream_handle).unwrap();
        new_sink.append(source);
        self.sink = Arc::new(new_sink);
        
        // Reset state
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        *self.current_position.lock().unwrap() = 0;
        *self.pause_start.lock().unwrap() = None;
        *self.total_pause_duration.lock().unwrap() = Duration::from_secs(0);
        *self.playback_start.lock().unwrap() = Some(Instant::now());

        // Create and start new display thread
        self.display_thread = Some(DisplayThread::new(
            Arc::clone(&self.is_playing),
            Arc::clone(&self.is_paused),
            Arc::clone(&self.current_position),
            self.total_duration,
            Arc::clone(&self.playback_start),
            Arc::clone(&self.pause_start),
            Arc::clone(&self.total_pause_duration),
        ));
    }

    fn create_decoder(&self) -> Result<Decoder<BufReader<File>>, String> {
        let path = self.file_path.as_ref()
            .ok_or_else(|| "No file path set".to_string())?;

        let file = File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let reader = BufReader::new(file);
        Decoder::new(reader)
            .map_err(|e| format!("Failed to create decoder: {}", e))
    }

    fn play_from_position(&mut self, position_ms: u64) -> Result<(), String> {
        // Check if position is within bounds
        if let Some(total_duration) = self.total_duration {
            if position_ms >= total_duration.as_millis() as u64 {
                self.is_playing.store(false, Ordering::SeqCst);
                return Err("Cannot seek beyond end of track".to_string());
            }
        }

        // Create decoder and skip to position
        let decoder = self.create_decoder()?;
        let skip_duration = Duration::from_millis(position_ms);
        let skipped_source = decoder.skip_duration(skip_duration);

        // Create new sink and play
        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create sink: {}", e))?;
        
        new_sink.append(skipped_source);
        
        // Stop old sink and replace with new one
        self.sink.stop();
        self.sink = Arc::new(new_sink);
        
        // Reset all timing-related state
        *self.playback_start.lock().unwrap() = Some(Instant::now());
        *self.pause_start.lock().unwrap() = None;
        *self.total_pause_duration.lock().unwrap() = Duration::from_secs(0);
        
        // Adjust playback start time to account for the seek position
        if let Ok(mut start_time) = self.playback_start.lock() {
            *start_time = Some(Instant::now() - Duration::from_millis(position_ms));
        }

        *self.current_position.lock().unwrap() = position_ms;
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        
        Ok(())
    }

    pub fn seek(&mut self, offset_seconds: i64) -> Result<(), String> {
        // Get current position with mutex lock
        let current_pos = *self.current_position.lock().unwrap();
        
        // Calculate new position with saturation arithmetic
        let new_pos = if offset_seconds.is_negative() {
            current_pos.saturating_sub(offset_seconds.unsigned_abs() * 1000)
        } else {
            current_pos.saturating_add(offset_seconds as u64 * 1000)
        };
        
        // Try to play from new position
        self.play_from_position(new_pos)?;
        
        // Update display if total duration is available
        if let Some(total_duration) = self.total_duration {
            let total_ms = total_duration.as_millis() as u64;
            
            // Get progress bar from display module
            let progress_bar = super::display::DisplayThread::format_progress_bar(
                new_pos,
                total_ms,
                super::display::DisplayThread::calculate_progress_bar_width()
            );
            
            // Format times using TimeUtils
            print!("\r\x1B[2K{} / {} {} (Playing)", 
                TimeUtils::format_time(new_pos),
                TimeUtils::format_time(total_ms),
                progress_bar
            );
            stdout().flush().unwrap();
        }
        
        Ok(())
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.is_playing.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        
        // Stop the display thread
        if let Some(mut display_thread) = self.display_thread.take() {
            display_thread.stop();
        }
    }

    pub fn toggle_pause(&self) {
        if self.is_paused.load(Ordering::SeqCst) {
            // Resuming playback
            if let Some(pause_time) = *self.pause_start.lock().unwrap() {
                let pause_duration = Instant::now() - pause_time;
                let mut total_pause = self.total_pause_duration.lock().unwrap();
                *total_pause += pause_duration;
            }
            *self.pause_start.lock().unwrap() = None;
            
            self.sink.play();
            self.is_paused.store(false, Ordering::SeqCst);
        } else {
            // Pausing playback
            *self.pause_start.lock().unwrap() = Some(Instant::now());
            self.sink.pause();
            self.is_paused.store(true, Ordering::SeqCst);
        }
    }

    pub fn is_playing(&self) -> bool {
        if self.is_paused.load(Ordering::SeqCst) {
            return true;
        }
        
        let sink_active = !self.sink.empty() && self.sink.len() > 0;
        let currently_playing = self.is_playing.load(Ordering::SeqCst);
        
        let playing = sink_active && currently_playing;
        
        if !playing && currently_playing {
            self.is_playing.store(false, Ordering::SeqCst);
        }
        
        playing
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        if let Some(mut display_thread) = self.display_thread.take() {
            display_thread.stop();
        }
    }
}