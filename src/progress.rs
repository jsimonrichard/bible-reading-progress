use chrono::{DateTime, Utc};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::range_query::RangeMap;

/// Represents the start position of a range for comparison purposes.
/// Used as a key in RangeQueryMap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct InsideBookBibleReference {
    chapter: u32,
    verse: u32,
}

/// Tracks reading statistics for a bible passage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingRecord {
    /// Number of times this passage has been read
    pub read_count: u32,
    /// Most recent time this passage was read
    pub last_read: DateTime<Utc>,
}

impl ReadingRecord {
    pub fn new() -> Self {
        Self {
            read_count: 1,
            last_read: Utc::now(),
        }
    }

    /// Increments the read count and updates the last read time.
    pub fn mark_read(&mut self) {
        self.read_count += 1;
        self.last_read = Utc::now();
    }
}

impl Default for ReadingRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// Main data structure for tracking bible reading progress.
/// Organized by book for efficient querying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingProgress {
    /// Maps each book to its reading records.
    /// Within each book, ranges are stored in a RangeQueryMap for efficient overlap queries.
    #[serde(default)]
    pub books: HashMap<String, RangeMap<InsideBookBibleReference, ReadingRecord>>,
}

impl ReadingProgress {
    /// Creates a new empty ReadingProgress.
    pub fn new() -> Self {
        Self {
            books: HashMap::new(),
        }
    }
}

impl Default for ReadingProgress {
    fn default() -> Self {
        Self::new()
    }
}
