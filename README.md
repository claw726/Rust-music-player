# Rust Audio Player

A command-line audio player built in Rust that supports multiple audio formats through native decoders and FFmpeg as a fallback.

## Features

- Play multiple audio formats
- Progress bar display
- Seeking functionality
- Time display
- Pause/Resume playback
- Vim-style key-bindings

| Category | Format | Extensions | Decoder |
|----------|---------|------------|----------|
| **Lossless** | 
| | FLAC | `.flac` | [Rodio](https://github.com/RustAudio/rodio) |
| | ALAC | `.m4a` | [alac.rs](https://github.com/ebarnard/alac.rs) |
| | WAV | `.wav` | [Rodio](https://github.com/RustAudio/rodio) |
| **Lossy** |
| | MP3 | `.mp3` | [Rodio](https://github.com/RustAudio/rodio) |
| | Opus | `.opus` | [opus-rs](https://github.com/SpaceManiac/opus-rs) |
| | Vorbis | `.ogg` | [ogg](https://github.com/RustAudio/ogg) |
| | AAC | `.m4a`, `.aac` | [Rodio](https://github.com/RustAudio/rodio) | |
| | WMA | `.wma` | [FFmpeg](https://www.ffmpeg.org/) ([rust-bindings](https://github.com/zmwangx/rust-ffmpeg)) |
| **Containers** |
| | OGG | `.ogg` | [ogg](https://github.com/RustAudio/ogg) / [Rodio](https://github.com/RustAudio/rodio) |
| | M4A | `.m4a` | Multiple¹ |

¹ M4A files are automatically detected and decoded using the appropriate decoder (Opus, AAC, or ALAC)

## Playlist Features

When launching the program with a directory instead of a single file:
- Automatically creates a playlist of all supported audio files in the directory
- Files are sorted alphabetically for predictable ordering
- Supports navigation between tracks in the folder
- Retains playlist position when skipping tracks

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
  libglu1-mesa-dev \
  libavcodec-dev \
  libavformat-dev \
  libavutil-dev \
  ffmpeg \
  clang \
  libclang-dev

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
  mesa-libGLU-devel \
  ffmpeg-devel \
  clang \
  clang-devel

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
  mesa \
  ffmpeg \
  clang \
  llvm
```

## Installation

### From Source
```bash
cargo install --path .
```
From Release
Download the latest release for your platform from the releases page.

## Usage
#### Individual Files
```bash
audioplayer <file>
```

#### Play an entire directory
```bash
audioplayer <directory>
```

## Playback Controls

| Key     | Action                                  | Mnemonic               |
|---------|-----------------------------------------|------------------------|
| `SPACE` | Play/Pause current track               | Standard media control|
| `q`     | Stop playback and exit program         | "Quit"                 |
| `k`/`←` | Seek backward 10 seconds               | Vim left / Arrow left  |
| `j`/`→` | Seek forward 10 seconds                | Vim right / Arrow right|
| `l`/`n` | Next track in playlist                  | Vim down/"Next"       |
| `h`/`p` | Previous track in playlist             | Vim up/"Previous"      |
| `?`     | Show help screen                       | Vim help               |

### Playlist Navigation
* In directory mode:
  * Automatically advances to the next track when current song ends
  * Wraps around to the first track when reaching end of playlist
  * Maintains playlist operation when using seek operations
  * Previous track operation wraps to the end when at the first track

### Seek Behavior

## Building

### Development Build
```bash
cargo build
```

### Production Build
For optimal performance, build with specific optimization flags:
```bash
cargo build --release
```

### Cross Compilation
To build for different platforms:

```bash
# For Windows (requires mingw-w64)
cargo build --release --target x86_64-pc-windows-gnu

# For macOS (requires OSX SDK)
cargo build --release --target x86_64-apple-darwin

# For Linux
cargo build --release --target x86_64-unknown-linux-gnu
```

## License
This project is licensed under the MIT License - see the LICENSE file for details.
