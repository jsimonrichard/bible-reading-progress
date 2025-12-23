use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::range_query::{CanCoalesce, RangeMap};

/// Represents the start position of a range for comparison purposes.
/// Used as a key in RangeQueryMap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InsideBookBibleReference {
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

impl Default for ReadingRecord {
    fn default() -> Self {
        Self {
            read_count: 1,
            last_read: Utc::now(),
        }
    }
}

impl CanCoalesce for ReadingRecord {
    fn coalesce(&self, other: &Self) -> Option<Self> {
        if self.read_count == other.read_count {
            Some(ReadingRecord {
                read_count: self.read_count,
                last_read: self.last_read.max(other.last_read),
            })
        } else {
            None
        }
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

    pub fn mark_read(&mut self, book: String, reference: InsideBookBibleReference) {
        let records: &mut RangeMap<InsideBookBibleReference, ReadingRecord> =
            self.books.entry(book).or_insert_with(RangeMap::new);
        records.insert_with(
            reference..reference,
            ReadingRecord::default(),
            |old, new| ReadingRecord {
                read_count: old.read_count + new.read_count,
                last_read: new.last_read,
            },
        );
    }

    pub fn set_read_count(
        &mut self,
        book: String,
        reference: InsideBookBibleReference,
        read_count: u32,
        last_read: Option<DateTime<Utc>>,
    ) {
        let records: &mut RangeMap<InsideBookBibleReference, ReadingRecord> =
            self.books.entry(book).or_insert_with(RangeMap::new);
        records.insert_replace(
            reference..reference,
            ReadingRecord {
                read_count,
                last_read: last_read.unwrap_or(Utc::now()),
            },
        );
    }
}

impl Default for ReadingProgress {
    fn default() -> Self {
        Self::new()
    }
}
