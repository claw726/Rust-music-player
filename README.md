# Rust Audio Player

A command-line audio player built in Rust that supports MP3 and FLAC formats.

## Features

- Play MP3 and FLAC audio files
- Progress bar display
- Seeking functionality
- Time display
- Pause/Resume playback

## Dependencies

### Linux
Required system libraries:
```bash
# Ubuntu/Debian
sudo apt-get install libasound2-dev pkg-config

# Fedora
sudo dnf install alsa-lib-devel

# Arch Linux
sudo pacman -S alsa-lib
```

## Installation

### From Source
```bash
cargo install --path .
```
From Release
Download the latest release for your platform from the releases page.

Usage

```bash
audioplayer <file>
```
Controls
Space: Play/Pause
Left/Right: Seek backward/forward
Enter: Quit

## Building
``` bash
# Debug build
cargo build

# Release build
cargo build --release
```
## License
This project is licensed under the MIT License - see the LICENSE file for details.