use std::path::PathBuf;

pub struct Config {
    pub progress_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            progress_path: PathBuf::from("reading_progress.yaml"),
        }
    }
}
