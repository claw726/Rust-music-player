# Rust Audio Player

A command-line audio player built in Rust that supports multiple audio formats through native decoders and FFmpeg as a fallback.

## Features

- Play multiple audio formats
- Progress bar display
- Seeking functionality
- Time display
- Pause/Resume playback

| Category | Format | Extensions | Decoder |
|----------|---------|------------|----------|
| **Lossless** | 
| | FLAC | `.flac` | [Rodio](https://github.com/RustAudio/rodio) |
| | ALAC | `.m4a` | [FFmpeg](https://www.ffmpeg.org/) ([rust-bindings](https://github.com/zmwangx/rust-ffmpeg)) |
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

Usage

```bash
audioplayer <file>
```
Controls
Space: Play/Pause
Left/Right: Seek backward/forward
Enter: Quit

I'll create a more comprehensive build section for the README that includes optimization flags and different build profiles. Here's the improved version:
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
