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
        let mut book_chapters = Vec::new();

        let book_records = progress.books.get(book);
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

            let chapter_text = format!(
                "Chapter {} ({} / {} verses)",
                chapter, read_verses, total_verses
            );
            let chapter_style = if read_verses == total_verses {
                Style::default().fg(Color::Green)
            } else if read_verses > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };

            book_chapters.push(TreeItem::new_leaf(
                TreeId::Chapter {
                    book: book.clone(),
                    chapter,
                },
                Text::from(chapter_text).style(chapter_style),
            ));
        }

        // Calculate book progress and format label with percentage
        let (total_verses, read_verses) = calculate_book_progress(&book, chapters, book_records);
        let percentage = if total_verses > 0 {
            (read_verses as f64 / total_verses as f64 * 100.0).round()
        } else {
            0.0
        };
        let book_label = format!("{} ({:.0}%)", book, percentage);
        let book_id = book.clone();
        ot_books.push(TreeItem::new(TreeId::Book(book_id), book_label, book_chapters).unwrap());
    }

    tree.push(TreeItem::new(TreeId::OldTestament, "Old Testament", ot_books).unwrap());

    // New Testament
    let mut nt_books = Vec::new();
    for book in bible.nt.keys() {
        let chapters = bible.nt.get(book).unwrap();
        let mut book_chapters = Vec::new();

        let book_records = progress.books.get(book);
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

            let chapter_text = format!(
                "Chapter {} ({} / {} verses)",
                chapter, read_verses, total_verses
            );
            let chapter_style = if read_verses == total_verses {
                Style::default().fg(Color::Green)
            } else if read_verses > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Red)
            };

            book_chapters.push(TreeItem::new_leaf(
                TreeId::Chapter {
                    book: book.clone(),
                    chapter,
                },
                Text::from(chapter_text).style(chapter_style),
            ));
        }

        // Calculate book progress and format label with percentage
        let (total_verses, read_verses) = calculate_book_progress(book, chapters, book_records);
        let percentage = if total_verses > 0 {
            read_verses as f64 / total_verses as f64 * 100.0
        } else {
            0.0
        };
        let book_label = format!("{} ({:.1}%)", book, percentage);
        let book_id = book.clone();
        nt_books.push(TreeItem::new(TreeId::Book(book_id), book_label, book_chapters).unwrap());
    }

    tree.push(TreeItem::new(TreeId::NewTestament, "New Testament", nt_books).unwrap());

    tree
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
