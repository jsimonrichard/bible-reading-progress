use chrono::{NaiveDate, Utc};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Text;
use tui_tree_widget::TreeItem;

use crate::progress::{InsideBookBibleReference, ReadingProgress, ReadingRecord};
use crate::range_query::RangeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TreeId {
    OldTestament,
    NewTestament,
    Book(String),
    Chapter {
        book: String,
        chapter: u32,
    },
    Passage {
        book: String,
        chapter: u32,
        verse_start: u32,
        verse_end: u32,
    },
}

#[derive(Debug, Clone)]
pub struct DashboardItem {
    pub book: String,
    pub chapter: u32,
    pub verse_start: u32,
    pub verse_end: u32,
    pub read_count: u32,
    pub last_read: Option<chrono::NaiveDate>,
    pub is_read: bool,
}

pub fn build_dashboard_tree_items(
    bible: &'static crate::bible_structure::BibleStructure,
    progress: &ReadingProgress,
) -> Vec<TreeItem<'static, TreeId>> {
    let mut tree = Vec::new();

    // Old Testament
    let mut ot_books = Vec::new();
    for book in bible.ot.keys() {
        let chapters = bible.ot.get(book).unwrap();
        let book_records = progress.books.get(book);
        let book_chapters = build_chapter_items(book, chapters, book_records);
        let book_label = build_book_label(book, chapters, book_records);
        let book_id = book.clone();
        ot_books.push(TreeItem::new(TreeId::Book(book_id), book_label, book_chapters).unwrap());
    }

    tree.push(TreeItem::new(TreeId::OldTestament, "Old Testament", ot_books).unwrap());

    // New Testament
    let mut nt_books = Vec::new();
    for book in bible.nt.keys() {
        let chapters = bible.nt.get(book).unwrap();
        let book_records = progress.books.get(book);
        let book_chapters = build_chapter_items(book, chapters, book_records);
        let book_label = build_book_label(book, chapters, book_records);
        let book_id = book.clone();
        nt_books.push(TreeItem::new(TreeId::Book(book_id), book_label, book_chapters).unwrap());
    }

    tree.push(TreeItem::new(TreeId::NewTestament, "New Testament", nt_books).unwrap());

    tree
}

/// Build chapter tree items for a book
fn build_chapter_items(
    book: &str,
    chapters: &[u32],
    book_records: Option<&RangeMap<InsideBookBibleReference, ReadingRecord>>,
) -> Vec<TreeItem<'static, TreeId>> {
    let mut book_chapters = Vec::new();

    for (chapter_idx, &max_verse) in chapters.iter().enumerate() {
        let chapter = (chapter_idx + 1) as u32;
        let verse_items = compute_chapter_items(book, chapter, max_verse, book_records);

        let total_verses: u32 = verse_items
            .iter()
            .map(|item| item.verse_end - item.verse_start + 1)
            .sum();
        let read_verses: u32 = verse_items
            .iter()
            .filter(|item| item.is_read)
            .map(|item| item.verse_end - item.verse_start + 1)
            .sum();

        // Calculate read count statistics for this chapter
        let (min_read_count, verses_read_more, total_verses_for_stats) =
            calculate_chapter_read_stats(chapter, max_verse, book_records);

        // Find the most recent last_read date for this chapter
        let last_read_date = verse_items.iter().filter_map(|item| item.last_read).max();

        let last_read_text = if let Some(date) = last_read_date {
            format!(" | Last read: {}", format_last_read_date(date))
        } else {
            String::new()
        };

        let read_count_text =
            format_read_count_text(min_read_count, verses_read_more, total_verses_for_stats);

        // Special case: if all verses are read at least one more time (100%), add parenthetical with verse count
        let read_count_display = if verses_read_more == total_verses_for_stats
            && total_verses_for_stats > 0
            && min_read_count > 0
        {
            format!("{}x ({} verses)", min_read_count, total_verses_for_stats)
        } else {
            read_count_text
        };

        let chapter_text = if !read_count_display.is_empty() {
            format!(
                "Chapter {} ({}){}",
                chapter, read_count_display, last_read_text
            )
        } else {
            format!(
                "Chapter {} ({} / {} verses){}",
                chapter, read_verses, total_verses, last_read_text
            )
        };
        let chapter_style = if read_verses == total_verses {
            Style::default().fg(Color::Green)
        } else if read_verses > 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Red)
        };

        book_chapters.push(TreeItem::new_leaf(
            TreeId::Chapter {
                book: book.to_string(),
                chapter,
            },
            Text::from(chapter_text).style(chapter_style),
        ));
    }

    book_chapters
}

/// Build book label text
fn build_book_label(
    book: &str,
    chapters: &[u32],
    book_records: Option<&RangeMap<InsideBookBibleReference, ReadingRecord>>,
) -> String {
    // Calculate read count statistics for this book
    let (min_read_count, verses_read_more, total_verses_for_stats) =
        calculate_book_read_stats(chapters, book_records);

    // Find the most recent last_read date across all chapters in this book
    let book_last_read = if let Some(records) = book_records {
        records.iter().map(|(_, record)| record.last_read).max()
    } else {
        None
    };

    let last_read_text = if let Some(date) = book_last_read {
        format!(" | Last read: {}", format_last_read_date(date))
    } else {
        String::new()
    };

    let read_count_text =
        format_read_count_text(min_read_count, verses_read_more, total_verses_for_stats);

    if !read_count_text.is_empty() {
        format!("{} ({}){}", book, read_count_text, last_read_text)
    } else {
        format!("{} ({})", book, last_read_text)
    }
}

/// Format read count display text: "2x" or "2x + 2%" or "2x + 20/30"
/// If all verses are read at least one more time (verses_read_more == total_verses), don't show the extra part
fn format_read_count_text(min_read_count: u32, verses_read_more: u32, total_verses: u32) -> String {
    if min_read_count == 0 {
        return String::from("0%");
    }

    // If no verses are read more, just show the base count
    if verses_read_more == 0 {
        return format!("{}x", min_read_count);
    }

    // If all verses are read at least one more time (100%), don't show the extra part
    // Check both exact equality and if the fraction is effectively 1.0
    if verses_read_more == total_verses && total_verses > 0 {
        return format!("{}x", min_read_count);
    }

    // Use percentage if total is >= 100, otherwise use fraction
    if total_verses >= 100 {
        let percentage = (verses_read_more as f64 / total_verses as f64 * 100.0).round();
        // Don't show if it's 100% (check both exact and rounded percentage)
        if verses_read_more == total_verses || percentage >= 100.0 {
            format!("{}x", min_read_count)
        } else {
            format!("{}x + {:.0}%", min_read_count, percentage)
        }
    } else {
        // For fractions, only hide if it's exactly all verses
        if verses_read_more == total_verses {
            format!("{}x", min_read_count)
        } else {
            format!(
                "{}x + {}/{} verses",
                min_read_count, verses_read_more, total_verses
            )
        }
    }
}

/// Get the maximum read count for each verse in a chapter
fn get_verse_read_counts(
    chapter: u32,
    max_verse: u32,
    book_records: &RangeMap<InsideBookBibleReference, ReadingRecord>,
) -> std::collections::HashMap<u32, u32> {
    let mut verse_read_counts = std::collections::HashMap::new();

    let chapter_start = InsideBookBibleReference { chapter, verse: 1 };
    let chapter_end_exclusive = InsideBookBibleReference {
        chapter,
        verse: max_verse + 1,
    };

    for (range, record) in book_records.range(chapter_start..chapter_end_exclusive) {
        if range.start.chapter == chapter && range.end.chapter == chapter {
            for verse in range.start.verse..range.end.verse {
                let current_max = verse_read_counts.get(&verse).copied().unwrap_or(0);
                if record.read_count > current_max {
                    verse_read_counts.insert(verse, record.read_count);
                }
            }
        }
    }

    verse_read_counts
}

/// Calculate min read count and count of verses read at least one more time for a chapter
/// Returns (min_read_count, verses_read_more, total_verses)
fn calculate_chapter_read_stats(
    chapter: u32,
    max_verse: u32,
    book_records: Option<&RangeMap<InsideBookBibleReference, ReadingRecord>>,
) -> (u32, u32, u32) {
    if book_records.is_none() {
        return (0, 0, 0);
    }

    let records = book_records.unwrap();
    let verse_read_counts = get_verse_read_counts(chapter, max_verse, records);

    // Find minimum read count across all verses in this chapter
    // Include verses that haven't been read (read_count = 0)
    let mut min_read_count = u32::MAX;
    for verse in 1..=max_verse {
        let verse_read_count = verse_read_counts.get(&verse).copied().unwrap_or(0);
        if verse_read_count < min_read_count {
            min_read_count = verse_read_count;
        }
    }

    // If no verses have been read, min_read_count will be MAX, so set it to 0
    if min_read_count == u32::MAX {
        return (0, 0, 0);
    }

    // Count verses that have been read at least one more time than the minimum
    let mut verses_read_more = 0u32;
    for verse in 1..=max_verse {
        let verse_read_count = verse_read_counts.get(&verse).copied().unwrap_or(0);
        if verse_read_count > min_read_count {
            verses_read_more += 1;
        }
    }

    (min_read_count, verses_read_more, max_verse)
}

/// Calculate min read count and count of verses read at least one more time for a book
/// Returns (min_read_count, verses_read_more, total_verses)
fn calculate_book_read_stats(
    chapters: &[u32],
    book_records: Option<&RangeMap<InsideBookBibleReference, ReadingRecord>>,
) -> (u32, u32, u32) {
    if book_records.is_none() {
        return (0, 0, 0);
    }

    let records = book_records.unwrap();
    let mut all_verse_read_counts = Vec::new();

    // Collect read counts for all verses in the book
    for (chapter_idx, &max_verse) in chapters.iter().enumerate() {
        let chapter = (chapter_idx + 1) as u32;
        let verse_read_counts = get_verse_read_counts(chapter, max_verse, records);

        for verse in 1..=max_verse {
            let read_count = verse_read_counts.get(&verse).copied().unwrap_or(0);
            all_verse_read_counts.push(read_count);
        }
    }

    if all_verse_read_counts.is_empty() {
        return (0, 0, 0);
    }

    // Find minimum read count across all verses in the book
    // This will be 0 if any verse hasn't been read
    let min_read_count = all_verse_read_counts.iter().min().copied().unwrap_or(0);

    // Count verses that have been read at least one more time than the minimum
    let verses_read_more = all_verse_read_counts
        .iter()
        .filter(|&&count| count > min_read_count)
        .count() as u32;

    let total_verses = all_verse_read_counts.len() as u32;

    (min_read_count, verses_read_more, total_verses)
}

/// Format a date in natural language (e.g., "today", "yesterday", "last week")
fn format_last_read_date(date: NaiveDate) -> String {
    let today = Utc::now().date_naive();
    let days_ago = today.signed_duration_since(date).num_days();

    match days_ago {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        2..=7 => format!("{} days ago", days_ago),
        8..=14 => "last week".to_string(),
        15..=30 => {
            let weeks = days_ago / 7;
            if weeks == 1 {
                "1 week ago".to_string()
            } else {
                format!("{} weeks ago", weeks)
            }
        }
        31..=60 => {
            let months = days_ago / 30;
            if months == 1 {
                "1 month ago".to_string()
            } else {
                format!("{} months ago", months)
            }
        }
        _ => date.format("%Y-%m-%d").to_string(),
    }
}

/// Calculate the total and read verses for a book, taking into account read counts
/// and using a target based on the minimum read count across all passages
fn calculate_book_progress(
    _book: &str,
    chapters: &[u32],
    book_records: Option<&RangeMap<InsideBookBibleReference, ReadingRecord>>,
) -> (u32, u32) {
    if book_records.is_none() {
        // No records, calculate total verses only
        let total_verses: u32 = chapters.iter().sum();
        return (total_verses, 0);
    }

    let records = book_records.unwrap();

    // First, find the minimum and maximum read_count across all passages in the book
    let read_counts: Vec<u32> = records
        .iter()
        .map(|(_, record)| record.read_count)
        .collect();

    let min_read_count = read_counts.iter().min().copied().unwrap_or(0);
    let max_read_count = read_counts.iter().max().copied().unwrap_or(0);

    // Calculate target: if there are different read counts, use floor(min) + 1
    // Otherwise, if all passages have the same read_count, use that read_count as target
    // Special case: if min = 0, we want target = 1 (at least one read)
    let target_read_count = if min_read_count == 0 {
        1
    } else if min_read_count != max_read_count {
        // Different read counts: use floor(min) + 1
        min_read_count + 1
    } else {
        // All passages have the same read_count: use that as target
        min_read_count
    };

    let mut total_verses = 0u32;
    let mut read_verses = 0u32;

    // For each chapter, calculate verse-level read counts
    for (chapter_idx, &max_verse) in chapters.iter().enumerate() {
        let chapter = (chapter_idx + 1) as u32;

        // Get all read ranges for this chapter
        let chapter_start = InsideBookBibleReference { chapter, verse: 1 };
        let chapter_end_exclusive = InsideBookBibleReference {
            chapter,
            verse: max_verse + 1,
        };

        // Track the maximum read_count for each verse
        let mut verse_read_counts: std::collections::HashMap<u32, u32> =
            std::collections::HashMap::new();

        for (range, record) in records.range(chapter_start..chapter_end_exclusive) {
            if range.start.chapter == chapter && range.end.chapter == chapter {
                // This range is within this chapter
                for verse in range.start.verse..range.end.verse {
                    let current_max = verse_read_counts.get(&verse).copied().unwrap_or(0);
                    if record.read_count > current_max {
                        verse_read_counts.insert(verse, record.read_count);
                    }
                }
            }
        }

        // Count verses that meet the target
        total_verses += max_verse;
        for verse in 1..=max_verse {
            let verse_read_count = verse_read_counts.get(&verse).copied().unwrap_or(0);
            if verse_read_count >= target_read_count {
                read_verses += 1;
            }
        }
    }

    (total_verses, read_verses)
}

fn compute_chapter_items(
    book: &str,
    chapter: u32,
    max_verse: u32,
    book_records: Option<&RangeMap<InsideBookBibleReference, ReadingRecord>>,
) -> Vec<DashboardItem> {
    let mut items = Vec::new();

    if let Some(records) = book_records {
        // Get all read ranges for this chapter
        let chapter_start = InsideBookBibleReference { chapter, verse: 1 };
        let chapter_end_exclusive = InsideBookBibleReference {
            chapter,
            verse: max_verse + 1,
        };
        let read_ranges: Vec<_> = records
            .range(chapter_start..chapter_end_exclusive)
            .map(|(range, record)| {
                (
                    *range.start,
                    *range.end,
                    record.read_count,
                    record.last_read,
                )
            })
            .collect();

        // Find missing verses - collect read verses first
        let mut read_verses = std::collections::BTreeSet::new();
        for (start_ref, end_ref, _, _) in &read_ranges {
            if start_ref.chapter == chapter && end_ref.chapter == chapter {
                for verse in start_ref.verse..end_ref.verse {
                    read_verses.insert(verse);
                }
            }
        }

        // Create items for read verses
        for (start_ref, end_ref, read_count, last_read) in &read_ranges {
            if start_ref.chapter == chapter && end_ref.chapter == chapter {
                let verse_end = end_ref.verse.saturating_sub(1);
                if verse_end >= start_ref.verse {
                    items.push(DashboardItem {
                        book: book.to_string(),
                        chapter,
                        verse_start: start_ref.verse,
                        verse_end,
                        read_count: *read_count,
                        last_read: Some(*last_read),
                        is_read: true,
                    });
                }
            }
        }

        // Add missing verse ranges
        let mut current_start = None;
        for verse in 1..=max_verse {
            if !read_verses.contains(&verse) {
                if current_start.is_none() {
                    current_start = Some(verse);
                }
            } else {
                if let Some(start) = current_start {
                    items.push(DashboardItem {
                        book: book.to_string(),
                        chapter,
                        verse_start: start,
                        verse_end: verse - 1,
                        read_count: 0,
                        last_read: None,
                        is_read: false,
                    });
                    current_start = None;
                }
            }
        }
        if let Some(start) = current_start {
            items.push(DashboardItem {
                book: book.to_string(),
                chapter,
                verse_start: start,
                verse_end: max_verse,
                read_count: 0,
                last_read: None,
                is_read: false,
            });
        }
    } else {
        // No records for this book, all verses are unread
        items.push(DashboardItem {
            book: book.to_string(),
            chapter,
            verse_start: 1,
            verse_end: max_verse,
            read_count: 0,
            last_read: None,
            is_read: false,
        });
    }

    items
}
