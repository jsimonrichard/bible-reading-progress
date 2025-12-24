use std::path::PathBuf;

pub struct Config {
    pub progress_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let progress_path = if cfg!(debug_assertions) {
            // Debug/dev builds: use current directory
            PathBuf::from("reading_progress.yaml")
        } else {
            // Release/production builds: use platform-specific directory
            let data_dir = dirs::data_dir().expect("Failed to get data directory");
            data_dir.join("bible-reading-progress.yaml")
        };

        Self { progress_path }
    }
}
