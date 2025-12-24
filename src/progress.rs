use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::range_query::RangeMap;

/// Represents the start position of a range for comparison purposes.
/// Used as a key in RangeQueryMap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InsideBookBibleReference {
    pub chapter: u32,
    pub verse: u32,
}

/// Tracks reading statistics for a bible passage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadingRecord {
    /// Number of times this passage has been read
    pub read_count: u32,
    /// Most recent date this passage was read
    pub last_read: NaiveDate,
}

impl Default for ReadingRecord {
    fn default() -> Self {
        Self {
            read_count: 1,
            last_read: Utc::now().date_naive(),
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
        // For a single verse, use exclusive end (verse + 1)
        let next_reference = InsideBookBibleReference {
            chapter: reference.chapter,
            verse: reference.verse + 1,
        };
        records.insert_with(
            reference..next_reference,
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
        last_read: Option<NaiveDate>,
    ) {
        let records: &mut RangeMap<InsideBookBibleReference, ReadingRecord> =
            self.books.entry(book).or_insert_with(RangeMap::new);
        records.insert_replace(
            reference..reference,
            ReadingRecord {
                read_count,
                last_read: last_read.unwrap_or_else(|| Utc::now().date_naive()),
            },
        );
    }

    /// Marks a range as read, overwriting any overlapping ranges instead of adding them together.
    pub fn mark_read_overwrite(
        &mut self,
        book: String,
        reference: InsideBookBibleReference,
        read_count: u32,
        last_read: Option<NaiveDate>,
    ) {
        let records: &mut RangeMap<InsideBookBibleReference, ReadingRecord> =
            self.books.entry(book).or_insert_with(RangeMap::new);
        // For a single verse, use exclusive end (verse + 1)
        let next_reference = InsideBookBibleReference {
            chapter: reference.chapter,
            verse: reference.verse + 1,
        };
        records.insert_replace(
            reference..next_reference,
            ReadingRecord {
                read_count,
                last_read: last_read.unwrap_or_else(|| Utc::now().date_naive()),
            },
        );
    }
}

impl Default for ReadingProgress {
    fn default() -> Self {
        Self::new()
    }
}
