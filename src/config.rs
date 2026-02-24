use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Path where the reading progress is stored
    /// Can be absolute or relative to the config directory
    pub progress_path: Option<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            progress_path: None,
        }
    }
}

pub struct Config {
    pub progress_path: PathBuf,
    config_file_path: PathBuf,
    /// True when progress path was overridden in dev mode (in-repo file)
    progress_path_overridden: bool,
}

impl Config {
    /// Loads the config from the standard config directory
    /// Falls back to defaults if the config file doesn't exist
    /// Supports both .yaml and .yml extensions, preferring .yaml
    pub fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get config directory"))?;
        let config_file_yaml = config_dir.join("bible-reading-progress.yaml");
        let config_file_yml = config_dir.join("bible-reading-progress.yml");

        let config_file: ConfigFile = if config_file_yaml.exists() {
            let content = fs::read_to_string(&config_file_yaml)?;
            serde_yaml::from_str(&content).unwrap_or_default()
        } else if config_file_yml.exists() {
            let content = fs::read_to_string(&config_file_yml)?;
            serde_yaml::from_str(&content).unwrap_or_default()
        } else {
            // Create default config file if it doesn't exist (prefer .yaml)
            let default_config = ConfigFile::default();
            if let Some(parent) = config_file_yaml.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = serde_yaml::to_string(&default_config)?;
            fs::write(&config_file_yaml, content)?;
            default_config
        };

        // Determine progress path
        let mut progress_path = if let Some(configured_path) = &config_file.progress_path {
            // Expand tilde if present
            let expanded_path = if configured_path.starts_with("~/") {
                let home = dirs::home_dir()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get home directory"))?;
                home.join(&configured_path[2..])
            } else if configured_path == "~" {
                dirs::home_dir()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get home directory"))?
            } else {
                // If it's an absolute path, use it directly
                // Otherwise, treat it as relative to the config directory
                let path = PathBuf::from(configured_path);
                if path.is_absolute() {
                    path
                } else {
                    config_dir.join(configured_path)
                }
            };
            expanded_path
        } else {
            // Default: use data directory for progress storage
            if cfg!(debug_assertions) {
                // Debug/dev builds: use in-repo file
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reading_progress.yaml")
            } else {
                // Release/production builds: use platform-specific directory
                let data_dir = dirs::data_dir()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get data directory"))?;
                data_dir
                    .join("bible-reading-progress")
                    .join("reading_progress.yaml")
            }
        };

        // In dev mode, always use the in-repo progress file (override config)
        let progress_path_overridden = cfg!(debug_assertions);
        if progress_path_overridden {
            progress_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reading_progress.yaml");
        }

        // Determine which config file was actually used
        let config_file_path = if config_file_yaml.exists() {
            config_file_yaml
        } else if config_file_yml.exists() {
            config_file_yml
        } else {
            config_file_yaml
        };

        Ok(Self {
            progress_path,
            config_file_path,
            progress_path_overridden,
        })
    }
}

impl Config {
    /// Returns the path to the config file that was loaded
    pub fn config_file_path(&self) -> &PathBuf {
        &self.config_file_path
    }

    /// True when the progress path was overridden in dev mode (in-repo file used instead of config)
    pub fn progress_path_overridden(&self) -> bool {
        self.progress_path_overridden
    }

    /// Returns the absolute path to the progress file
    pub fn progress_path_absolute(&self) -> PathBuf {
        if self.progress_path.is_absolute() {
            self.progress_path.clone()
        } else {
            // Resolve relative path to absolute
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let absolute = current_dir.join(&self.progress_path);
            // Try to canonicalize, but if it fails (e.g., file doesn't exist yet), return the joined path
            absolute.canonicalize().unwrap_or(absolute)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::load().unwrap_or_else(|_| {
            // Fallback if loading fails
            let progress_path = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reading_progress.yaml")
            } else {
                dirs::data_dir()
                    .expect("Failed to get data directory")
                    .join("bible-reading-progress")
                    .join("reading_progress.yaml")
            };
            let config_file_path = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("bible-reading-progress.yaml");
            let progress_path_overridden = cfg!(debug_assertions);
            Self {
                progress_path,
                config_file_path,
                progress_path_overridden,
            }
        })
    }
}
