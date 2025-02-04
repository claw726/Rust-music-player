use std::{env, io::{stdout, Write}, path::Path, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::{Duration, Instant}};
use std::path::PathBuf;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{enable_raw_mode, disable_raw_mode},
};

use rust_music_player::audio::player::AudioPlayer;
use rust_music_player::utils::metadata::print_song_info;

mod playlist;
use playlist::{Playlist, get_supported_files};

// Poll keyboard at 60x / s
const POLL_INTERVAL: Duration = Duration::from_millis(60);

fn main() -> Result<()> {
    let args = parse_args()?;
    let mut player = AudioPlayer::new()?;
    let (mut playlist, is_directory) = setup_playlist(&args)?;

    print_controls()?;
    enable_raw_mode()?;

    let should_stop = Arc::new(AtomicBool::new(false));
    let mut last_seek = Instant::now();
    let seek_cooldown = Duration::from_millis(100);

    while let Some(current_path) = playlist.current() {
        handle_track_start(current_path, &mut player)?;

        let original_index = playlist.current_index();

        let exit_program = handle_playback_loop(
            &mut player,
            &mut playlist,
            &should_stop,
            &mut last_seek,
            seek_cooldown,
            is_directory,
        )?;

        if exit_program {
            break;
        }

        if playlist.current_index() == original_index {
            playlist.next();
        }

        if playlist.current().is_none() {
            break;
        }
    }

    cleanup(player)
}

fn parse_args() -> anyhow::Result<PathBuf> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        anyhow::bail!("Usage: {} <audio_file_or_directory>", args[0]);
    }
    Ok(PathBuf::from(&args[1]))
}

fn setup_playlist(path: &Path) -> anyhow::Result<(Playlist, bool)> {
    let (files, is_directory) = if path.is_dir() {
        (get_supported_files(path)?, true)
    } else {
        (vec![path.to_path_buf()], false)
    };

    if files.is_empty() {
        anyhow::bail!("No supported audio files found");
    }

    Ok((Playlist::new(files), is_directory))
}

fn print_controls() -> anyhow::Result<()> {
    let controls = [
        ("SPACE",   "Play/Pause"),
        ("q/ENTER", "Quit program"),
        ("→/k",     "Seek forward 10s"),
        ("←/j",     "Seek backward 10s"),
        ("n/l",     "Next track (playlist)"),
        ("p/h",     "Previous track (playlist)"),
        ("?",       "Show this help"),
    ];

    println!("\r\n\n\n=== Controls ===\n");
    for (key, action) in controls {
        println!("\r{:<8} : {}", key, action);
    }
    println!("\r==================");
    stdout().flush()?;
    Ok(())
}

fn handle_track_start(path: &Path, player: &mut AudioPlayer) -> anyhow::Result<()> {
    let duration = print_song_info(path)?;
    player.set_metadata_duration(duration);
    player.play(path)?;
    Ok(())
}

fn handle_playback_loop(
    player: &mut AudioPlayer,
    playlist: &mut Playlist,
    should_stop: &AtomicBool,
    last_seek: &mut Instant,
    seek_cooldown: Duration,
    is_directory: bool,
) -> anyhow::Result<bool> {
    let mut not_playing_count = 0;
    const MAX_NOT_PLAYING: u32 = 3;

    while player.is_playing() {
        if should_stop.load(Ordering::SeqCst) {
            return Ok(true);
        }

        handle_user_input(
            player,
            playlist,
            should_stop,
            last_seek,
            seek_cooldown,
            is_directory,
        )?;

        if !check_playback_status(player, &mut not_playing_count, MAX_NOT_PLAYING) {
            break;
        }
    }

    Ok(false)
}

fn handle_user_input(
    player: &mut AudioPlayer,
    playlist: &mut Playlist,
    should_stop: &AtomicBool,
    last_seek: &mut Instant,
    seek_cooldown: Duration,
    is_directory: bool,
) -> anyhow::Result<()> {
    if event::poll(POLL_INTERVAL)? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char(' ') => player.toggle_pause(),
                KeyCode::Enter | KeyCode::Char('q') => should_stop.store(true, Ordering::SeqCst),
                KeyCode::Right | KeyCode::Char('k') => handle_seek(player, 10, last_seek, seek_cooldown),
                KeyCode::Left  | KeyCode::Char('j') => handle_seek(player, -10, last_seek, seek_cooldown),
                KeyCode::Char('n') | KeyCode::Char('l') if is_directory => handle_next_track(player, playlist),
                KeyCode::Char('p') | KeyCode::Char('h') if is_directory => handle_prev_track(player, playlist),
                KeyCode::Char('?') => print_controls()?,
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_seek(
    player: &mut AudioPlayer,
    offset: i64,
    last_seek: &mut Instant,
    cooldown: Duration,
) {
    let now = Instant::now();
    if now.duration_since(*last_seek) >= cooldown {
        let _ = player.seek(offset);
        *last_seek = now;
    }
}

fn handle_next_track(player: &mut AudioPlayer, playlist: &mut Playlist) {
    player.stop();
    playlist.next();
}

fn handle_prev_track(player: &mut AudioPlayer, playlist: &mut Playlist) {
    player.stop();
    playlist.previous();
}

fn check_playback_status(
    player: &AudioPlayer,
    not_playing_count: &mut u32,
    max_count: u32,
) -> bool {
    if !player.is_playing() {
        *not_playing_count += 1;
        if *not_playing_count >= max_count {
            return false;
        }
    } else {
        *not_playing_count = 0;
    }
    true
}

fn cleanup(mut player: AudioPlayer) -> anyhow::Result<()> {
    player.stop();
    disable_raw_mode()?;
    println!("\rProgram exiting.");
    Ok(())
}