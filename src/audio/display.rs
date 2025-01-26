use std::{
    io::{stdout, Write},
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use terminal_size::{terminal_size, Width, Height};

use crate::audio::{TimeFormat, TimeUtils};

// Display rate of 60fps
const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub struct DisplayThread {
    handle: Option<JoinHandle<()>>,
    should_stop: Arc<AtomicBool>,
}

impl DisplayThread {
    pub fn new(
        is_playing: Arc<AtomicBool>,
        is_paused: Arc<AtomicBool>,
        current_position: Arc<Mutex<u64>>,
        total_duration: Option<Duration>,
        playback_start: Arc<Mutex<Option<Instant>>>,
        pause_start: Arc<Mutex<Option<Instant>>>,
        total_pause_duration: Arc<Mutex<Duration>>,
    ) -> Self {
        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = Arc::clone(&should_stop);

        // Clear the line and hide the cursor at the start
        print!("\x1B[?25l"); // Hide cursor
        stdout().flush().unwrap();

        let handle = Some(thread::spawn(move || {
            let mut last_update = Instant::now();

            while !should_stop_clone.load(Ordering::SeqCst) {
                let now = Instant::now();
                if now.duration_since(last_update) >= POLL_INTERVAL {
                    if is_playing.load(Ordering::SeqCst) {
                        if let Some(start_time) = *playback_start.lock().unwrap() {
                            let pause_duration = *total_pause_duration.lock().unwrap();
                            let elapsed = if is_paused.load(Ordering::SeqCst) {
                                if let Some(pause_time) = *pause_start.lock().unwrap() {
                                    start_time.elapsed() - (Instant::now() - pause_time) - pause_duration
                                } else {
                                    start_time.elapsed() - pause_duration
                                }
                            } else {
                                start_time.elapsed() - pause_duration
                            };

                            let position_ms = elapsed.as_millis() as u64;
                            *current_position.lock().unwrap() = position_ms;

                            let total_ms = total_duration.map_or(0, |d| d.as_millis() as u64);
                            let progress_bar = Self::format_progress_bar(
                                position_ms,
                                total_ms,
                                Self::calculate_progress_bar_width()
                            );

                            let status = if is_paused.load(Ordering::SeqCst) {
                                "(Paused)"
                            } else {
                                "(Playing)"
                            };

                            // Move to start of line, clear line, and print update
                            print!("\r\x1B[2K{} / {} {} {}",
                                TimeUtils::format_time(position_ms),
                                TimeUtils::format_time(total_ms),
                                progress_bar,
                                status
                            );
                            stdout().flush().unwrap();

                            if let Some(duration) = total_duration {
                                if position_ms >= duration.as_millis() as u64 {
                                    is_playing.store(false, Ordering::SeqCst);
                                    println!(); // New line at end of playback
                                    print!("\x1B[?25h"); // Show cursor
                                    stdout().flush().unwrap();
                                    break;
                                }
                            }
                        }
                    }
                    last_update = now;
                }
                thread::sleep(Duration::from_millis(1));
            }

            // Show cursor when thread ends
            print!("\x1B[?25h");
            stdout().flush().unwrap();
        }));

        Self {
            handle,
            should_stop,
        }
    }

    pub fn stop(&mut self) {
        self.should_stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.handle.take() {
            let _ = thread.join();
        }
    }

    pub fn format_progress_bar(position: u64, total: u64, width: usize) -> String {
        if total == 0 { return String::new(); }
        let progress = (position as f64 / total as f64 * width as f64) as usize;
        // Pre-allocate the string capacity
        let mut bar = String::with_capacity(width + 2);
        bar.push('[');
        bar.extend(std::iter::repeat('=').take(progress));
        bar.extend(std::iter::repeat('-').take(width - progress));
        bar.push(']');
        bar
    }

    fn get_terminal_width() -> usize {
        if let Some((Width(w), Height(_))) = terminal_size() {
            w as usize
        } else {
            80 // fallback width if terminal size cannot be determined
        }
    }

    pub fn calculate_progress_bar_width() -> usize {
        let term_width = Self::get_terminal_width();
        // Reserve space for "00:00 / 00:00 [] (Playing)    "
        // Which is approximately 35 characters
        let reserved_space = 35;
        if term_width > reserved_space {
            term_width - reserved_space
        } else {
            20 // minimum progress bar width
        }
    }
}

impl Drop for DisplayThread {
    fn drop(&mut self) {
        self.stop();
    }
}