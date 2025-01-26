use std::{env, io::{stdout, Write}, path::Path, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::{Duration, Instant}};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{enable_raw_mode, disable_raw_mode},
};

use rust_music_player::audio::player::AudioPlayer;
use rust_music_player::utils::metadata::print_song_info;

// Poll keyboard at 60x / s
const POLL_INTERVAL: Duration = Duration::from_millis(60);

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <audio_file_path>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    let mut player = AudioPlayer::new()?;
    let song_duration = print_song_info(path).map_err(|e| anyhow::anyhow!("{}", e))?;
    player.set_metadata_duration(song_duration);

    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_clone = should_stop.clone();

    
    
    // Clear the screen and print controls
    let controls = [
        ("Space", "Pause/Resume"),
        ("Enter", "Stop and exit"),
        ("→", "Skip forward 10 seconds"),
        ("←", "Skip backwards 10 seconds"),
    ];
    
    println!("\n=== Controls ===\r");
    for (key, action) in controls {
        println!("{:<6} : {}\r", key, action);
    }
    println!("===============\r");
    stdout().flush()?;

    player.play(path)?;

    enable_raw_mode()?;

    let mut not_playing_count = 0;
    const MAX_NOT_PLAYING_CHECKS: u32 = 3;

    let mut last_seek = Instant::now();
    let seek_cooldown = Duration::from_millis(100);

    while player.is_playing() {
        if event::poll(POLL_INTERVAL)? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match key_event.code {
                        KeyCode::Char(' ') => {
                            player.toggle_pause();
                        },
                        KeyCode::Enter => {
                            should_stop_clone.store(true, Ordering::SeqCst);
                            break;
                        },
                        KeyCode::Right | KeyCode::Left => {
                            let now = Instant::now();
                            if now.duration_since(last_seek) >= seek_cooldown {
                                let offset = if key_event.code == KeyCode::Right { 10 } else { -10 };
                                let _ = player.seek(offset);
                                last_seek = now;
                            }
                        },
                        _ => {},
                    }
                }
            }
        }

        if should_stop_clone.load(Ordering::SeqCst) {
            println!("\rUser requested stop.");
            break;
        }

        if !player.is_playing() {
            not_playing_count += 1;
            if not_playing_count >= MAX_NOT_PLAYING_CHECKS {
                println!("\rPlayback confirmed finished.\r");
                player.stop();
                disable_raw_mode()?;
                std::process::exit(0);
            }
        } else {
            not_playing_count = 0;
        }
    }

    player.stop();
    disable_raw_mode()?;
    println!("\rProgram exiting.");
    Ok(())
}