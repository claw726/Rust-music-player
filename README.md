# Rust Audio Player

A command-line audio player built in Rust that supports MP3, FLAC, OPUS, and Vorbis formats.

## Features

- Play MP3 and FLAC audio files
- Progress bar display
- Seeking functionality
- Time display
- Pause/Resume playback

## Dependencies

## Dependencies

### Linux
Required system libraries:

```bash
# Ubuntu/Debian
sudo apt-get install \
  libasound2-dev \
  pkg-config \
  libx11-dev \
  libxext-dev \
  libxft-dev \
  libxinerama-dev \
  libxcursor-dev \
  libxrender-dev \
  libxfixes-dev \
  libpango1.0-dev \
  libgl1-mesa-dev \
  libglu1-mesa-dev

# Fedora
sudo dnf install \
  alsa-lib-devel \
  libX11-devel \
  libXext-devel \
  libXft-devel \
  libXinerama-devel \
  libXcursor-devel \
  libXrender-devel \
  libXfixes-devel \
  pango-devel \
  mesa-libGL-devel \
  mesa-libGLU-devel

# Arch Linux
sudo pacman -S \
  alsa-lib \
  libx11 \
  libxext \
  libxft \
  libxinerama \
  libxcursor \
  libxrender \
  libxfixes \
  pango \
  mesa
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