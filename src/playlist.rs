use std::path::{Path, PathBuf};

pub struct Playlist {
    files: Vec<PathBuf>,
    current_index: usize,
}

impl Playlist {
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files,
            current_index: 0,
        }
    }

    pub fn current(&self) -> Option<&Path> {
        self.files.get(self.current_index).map(|p| p.as_path())
    }

    pub fn next(&mut self) {
        self.current_index = (self.current_index + 1) % self.files.len();
    }

    pub fn previous(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        } else {
            self.current_index = self.files.len() - 1;
        }
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }
}

pub fn get_supported_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                if is_supported_extension(&ext) {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn is_supported_extension(ext: &str) -> bool {
    matches!(
        ext,
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "opus" | "aac"
    )
}