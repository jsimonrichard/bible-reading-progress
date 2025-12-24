use crate::config::Config;
use crate::progress::ReadingProgress;
use color_eyre::Result;
use std::fs;
use std::path::PathBuf;

pub fn get_all_books(bible: &crate::bible_structure::BibleStructure) -> Vec<String> {
    let mut books: Vec<String> = Vec::new();
    books.extend(bible.ot.keys().cloned());
    books.extend(bible.nt.keys().cloned());
    books
}

pub fn parse_verse_ranges(input: &str, max_verse: u32) -> Result<Vec<(u32, u32)>, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(vec![(1, max_verse)]);
    }

    let mut ranges = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid range format: {}", part));
            }
            let start = parts[0]
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid verse number: {}", parts[0]))?;
            let end = parts[1]
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid verse number: {}", parts[1]))?;
            if start > end || end > max_verse {
                return Err(format!(
                    "Invalid range: {}-{} (max: {})",
                    start, end, max_verse
                ));
            }
            ranges.push((start, end));
        } else {
            let verse = part
                .parse::<u32>()
                .map_err(|_| format!("Invalid verse number: {}", part))?;
            if verse > max_verse {
                return Err(format!("Invalid verse: {} (max: {})", verse, max_verse));
            }
            ranges.push((verse, verse));
        }
    }
    Ok(ranges)
}

pub fn get_progress_file_path(config: &Config) -> PathBuf {
    config.progress_path.clone()
}

pub fn load_progress(config: &Config) -> Result<ReadingProgress> {
    let path = get_progress_file_path(config);
    if !path.exists() {
        return Ok(ReadingProgress::new());
    }
    let content = fs::read_to_string(&path)?;
    let progress: ReadingProgress = serde_yaml::from_str(&content)?;
    Ok(progress)
}

pub fn save_progress(progress: &ReadingProgress, config: &Config) -> Result<()> {
    let path = get_progress_file_path(config);
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(progress)?;
    fs::write(&path, content)?;
    Ok(())
}
