use rust_music_player::audio::AudioPlayer;
use std::path::PathBuf;

#[test]
fn test_audio_player_creation() {
    let player = AudioPlayer::new();
    assert!(player.is_ok());
}

#[test]
fn test_audio_player_lifecycle() {
    let mut player = AudioPlayer::new().unwrap();

    // Test with a known audio file
    let test_file = PathBuf::from("tests/resources/test.wav");
    assert!(player.play(&test_file).is_ok());

    // Test pause/resume
    player.toggle_pause();
    assert!(player.is_paused());

    player.toggle_pause();
    assert!(!player.is_paused());

    // Test seek
    assert!(player.seek(5).is_ok());

    // Test stop
    player.stop();
    assert!(!player.is_playing());
}