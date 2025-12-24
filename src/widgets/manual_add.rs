use chrono::NaiveDate;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{prelude::*, widgets::*};

use crate::progress::{InsideBookBibleReference, ReadingProgress};
use crate::utils::{get_all_books, parse_verse_ranges};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputFocus {
    Book,
    Chapter,
    Verse,
    VerseEnd,
    ReadCount,
    Date,
}

pub struct ManualAddWidget {
    pub book_search: String,
    pub book_matches: Vec<String>,
    pub selected_book_index: usize,
    pub chapter_input: String,
    pub verse_input: String,
    pub verse_end_input: String,
    pub read_count_input: String,
    pub date_input: String,
    pub error_message: Option<String>,
    pub input_focus: InputFocus,
    pub show_confirmation: bool,
}

impl ManualAddWidget {
    pub fn new(bible: &'static crate::bible_structure::BibleStructure) -> Self {
        let books = get_all_books(bible);
        Self {
            book_search: String::new(),
            book_matches: books,
            selected_book_index: 0,
            chapter_input: String::new(),
            verse_input: String::new(),
            verse_end_input: String::new(),
            read_count_input: String::new(),
            date_input: String::new(),
            error_message: None,
            input_focus: InputFocus::Book,
            show_confirmation: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Book search
                Constraint::Length(8), // Book matches list
                Constraint::Length(3), // Chapter input
                Constraint::Length(3), // Verse input(s)
                Constraint::Length(3), // Read count input
                Constraint::Length(3), // Date input
                Constraint::Min(0),    // Error / help
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        // Header
        let header = Paragraph::new("Manual Add (Overwrite)")
            .style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            );
        frame.render_widget(header, chunks[0]);

        // Book search field
        let book_style = if self.input_focus == InputFocus::Book {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let book_widget = Paragraph::new(self.book_search.as_str())
            .style(book_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Book")
                    .border_style(if self.input_focus == InputFocus::Book {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );
        frame.render_widget(book_widget, chunks[1]);

        // Book matches list
        if !self.book_matches.is_empty() {
            let items: Vec<ListItem> = self
                .book_matches
                .iter()
                .enumerate()
                .map(|(idx, book)| {
                    let style = if idx == self.selected_book_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(book.as_str()).style(style)
                })
                .collect();
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Matches (↑↓: select)"),
            );
            frame.render_widget(list, chunks[2]);
        } else {
            let empty = Paragraph::new("No matches")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("Matches"));
            frame.render_widget(empty, chunks[2]);
        }

        // Chapter input field
        let chapter_style = if self.input_focus == InputFocus::Chapter {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let chapter_widget = Paragraph::new(self.chapter_input.as_str())
            .style(chapter_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chapter (e.g., 1, 1-5, or leave empty for entire book)")
                    .border_style(if self.input_focus == InputFocus::Chapter {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );
        frame.render_widget(chapter_widget, chunks[3]);

        // Verse input field(s) - show two columns if chapter range is detected
        let has_chapter_range = self.chapter_input.contains('-');
        if has_chapter_range {
            let verse_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[4]);

            // Start chapter verse input
            let verse_style = if self.input_focus == InputFocus::Verse {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let verse_widget = Paragraph::new(self.verse_input.as_str())
                .style(verse_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Start Chapter Verses (e.g., 1, 1-5, or leave empty)")
                        .border_style(if self.input_focus == InputFocus::Verse {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                );
            frame.render_widget(verse_widget, verse_chunks[0]);

            // End chapter verse input
            let verse_end_style = if self.input_focus == InputFocus::VerseEnd {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let verse_end_widget = Paragraph::new(self.verse_end_input.as_str())
                .style(verse_end_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("End Chapter Verses (e.g., 1, 1-5, or leave empty)")
                        .border_style(if self.input_focus == InputFocus::VerseEnd {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                );
            frame.render_widget(verse_end_widget, verse_chunks[1]);
        } else {
            // Single verse input field
            let verse_style = if self.input_focus == InputFocus::Verse {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let verse_widget = Paragraph::new(self.verse_input.as_str())
                .style(verse_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Verse (e.g., 1, 1-5, or leave empty for full chapter)")
                        .border_style(if self.input_focus == InputFocus::Verse {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                );
            frame.render_widget(verse_widget, chunks[4]);
        }

        // Read count input field
        let read_count_style = if self.input_focus == InputFocus::ReadCount {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let read_count_widget = Paragraph::new(self.read_count_input.as_str())
            .style(read_count_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Read Count (e.g., 1, 5, or leave empty for 1)")
                    .border_style(if self.input_focus == InputFocus::ReadCount {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );
        frame.render_widget(read_count_widget, chunks[5]);

        // Date input field
        let date_style = if self.input_focus == InputFocus::Date {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let date_widget = Paragraph::new(self.date_input.as_str())
            .style(date_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Date (YYYY-MM-DD, or leave empty for today)")
                    .border_style(if self.input_focus == InputFocus::Date {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );
        frame.render_widget(date_widget, chunks[6]);

        // Error message or help
        if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(error.clone())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(error_widget, chunks[5]);
        } else {
            let has_chapter_range = self.chapter_input.contains('-');
            let chapter_empty = self.chapter_input.trim().is_empty();
            let help_text = if chapter_empty {
                "Leave chapter empty to mark entire book as read (confirmation required). Overwrites overlapping ranges."
            } else if has_chapter_range {
                "Chapter range detected: Enter verses for start and end chapters. Middle chapters will be fully read. Overwrites overlapping ranges."
            } else {
                "Enter a verse number (e.g., 1), a range (e.g., 1-5), or leave empty for the full chapter. Overwrites overlapping ranges."
            };
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("Help"));
            frame.render_widget(help, chunks[7]);
        }

        // Footer
        let footer = Paragraph::new(
            "Tab: Next field | Shift+Tab: Previous field | ↑↓: Select book | Enter: Add | s: Save | Esc: Cancel",
        )
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[8]);

        // Show confirmation popup if needed
        if self.show_confirmation {
            let popup_area = Self::centered_rect(60, 25, frame.area());
            frame.render_widget(Clear, popup_area);
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title("Confirm"),
                popup_area,
            );

            let popup_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .margin(1)
                .split(popup_area);

            let message = Paragraph::new("Are you sure you want to mark the entire book as read? (This will overwrite overlapping ranges)")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(message, popup_chunks[0]);

            let instruction = Paragraph::new("Press Enter to confirm, Esc to cancel")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            frame.render_widget(instruction, popup_chunks[1]);
        }
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        bible: &'static crate::bible_structure::BibleStructure,
    ) -> Result<ManualAddAction> {
        // Handle confirmation popup
        if self.show_confirmation {
            match key.code {
                KeyCode::Enter => {
                    self.show_confirmation = false;
                    // Proceed with adding reading (chapter is empty, so entire book)
                    if self.book_matches.is_empty() {
                        self.error_message = Some("Please select a book first".to_string());
                        Ok(ManualAddAction::None)
                    } else {
                        Ok(ManualAddAction::AddReading)
                    }
                }
                KeyCode::Esc => {
                    self.show_confirmation = false;
                    Ok(ManualAddAction::None)
                }
                _ => Ok(ManualAddAction::None),
            }
        } else {
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => Ok(ManualAddAction::Cancel),
                (_, KeyCode::Tab) => {
                    // Navigate forward through input fields
                    let has_chapter_range = self.chapter_input.contains('-');
                    self.input_focus = match self.input_focus {
                        InputFocus::Book => InputFocus::Chapter,
                        InputFocus::Chapter => InputFocus::Verse,
                        InputFocus::Verse => {
                            if has_chapter_range {
                                InputFocus::VerseEnd
                            } else {
                                InputFocus::ReadCount
                            }
                        }
                        InputFocus::VerseEnd => InputFocus::ReadCount,
                        InputFocus::ReadCount => InputFocus::Date,
                        InputFocus::Date => InputFocus::Book,
                    };
                    self.error_message = None;
                    Ok(ManualAddAction::None)
                }
                (_, KeyCode::BackTab) => {
                    // Navigate backward through input fields
                    let has_chapter_range = self.chapter_input.contains('-');
                    self.input_focus = match self.input_focus {
                        InputFocus::Book => InputFocus::Date,
                        InputFocus::Chapter => InputFocus::Book,
                        InputFocus::Verse => InputFocus::Chapter,
                        InputFocus::VerseEnd => InputFocus::Verse,
                        InputFocus::ReadCount => {
                            if has_chapter_range {
                                InputFocus::VerseEnd
                            } else {
                                InputFocus::Verse
                            }
                        }
                        InputFocus::Date => InputFocus::ReadCount,
                    };
                    self.error_message = None;
                    Ok(ManualAddAction::None)
                }
                (_, KeyCode::Up) if self.input_focus == InputFocus::Book => {
                    if self.selected_book_index > 0 {
                        self.selected_book_index -= 1;
                    }
                    Ok(ManualAddAction::None)
                }
                (_, KeyCode::Down) if self.input_focus == InputFocus::Book => {
                    if self.selected_book_index < self.book_matches.len().saturating_sub(1) {
                        self.selected_book_index += 1;
                    }
                    Ok(ManualAddAction::None)
                }
                (_, KeyCode::Enter) => {
                    if self.input_focus == InputFocus::Book {
                        // Select the book and move to chapter
                        if !self.book_matches.is_empty() {
                            let selected_book = self.book_matches[self.selected_book_index].clone();
                            self.book_search = selected_book.clone();
                            self.input_focus = InputFocus::Chapter;
                            let search_query = self.book_search.clone();
                            let new_matches = Self::compute_book_matches(bible, &search_query);
                            self.book_matches = new_matches;
                        }
                        Ok(ManualAddAction::None)
                    } else if self.input_focus == InputFocus::Chapter {
                        // Move to verse input
                        self.input_focus = InputFocus::Verse;
                        Ok(ManualAddAction::None)
                    } else if self.input_focus == InputFocus::Verse {
                        // If chapter range, move to verse end, otherwise move to read count
                        let has_chapter_range = self.chapter_input.contains('-');
                        if has_chapter_range {
                            self.input_focus = InputFocus::VerseEnd;
                            Ok(ManualAddAction::None)
                        } else {
                            self.input_focus = InputFocus::ReadCount;
                            Ok(ManualAddAction::None)
                        }
                    } else if self.input_focus == InputFocus::VerseEnd {
                        // Move to read count
                        self.input_focus = InputFocus::ReadCount;
                        Ok(ManualAddAction::None)
                    } else if self.input_focus == InputFocus::ReadCount {
                        // Move to date
                        self.input_focus = InputFocus::Date;
                        Ok(ManualAddAction::None)
                    } else {
                        // Add the reading (from Date field)
                        // Check if chapter is empty - show confirmation if so
                        if self.chapter_input.trim().is_empty() {
                            self.show_confirmation = true;
                            Ok(ManualAddAction::None)
                        } else {
                            if self.book_matches.is_empty() {
                                self.error_message = Some("Please select a book first".to_string());
                                Ok(ManualAddAction::None)
                            } else {
                                Ok(ManualAddAction::AddReading)
                            }
                        }
                    }
                }
                (_, KeyCode::Backspace) => {
                    match self.input_focus {
                        InputFocus::Book => {
                            self.book_search.pop();
                            self.selected_book_index = 0;
                            let search_query = self.book_search.clone();
                            let new_matches = Self::compute_book_matches(bible, &search_query);
                            self.book_matches = new_matches;
                        }
                        InputFocus::Chapter => {
                            self.chapter_input.pop();
                        }
                        InputFocus::Verse => {
                            self.verse_input.pop();
                        }
                        InputFocus::VerseEnd => {
                            self.verse_end_input.pop();
                        }
                        InputFocus::ReadCount => {
                            self.read_count_input.pop();
                        }
                        InputFocus::Date => {
                            self.date_input.pop();
                        }
                    }
                    self.error_message = None;
                    Ok(ManualAddAction::None)
                }
                (_, KeyCode::Char(c)) if c.is_ascii() && !c.is_control() => {
                    match self.input_focus {
                        InputFocus::Book => {
                            self.book_search.push(c);
                            self.selected_book_index = 0;
                            let search_query = self.book_search.clone();
                            let new_matches = Self::compute_book_matches(bible, &search_query);
                            self.book_matches = new_matches;
                        }
                        InputFocus::Chapter => {
                            if c.is_ascii_digit() || c == '-' {
                                self.chapter_input.push(c);
                            }
                        }
                        InputFocus::Verse => {
                            if c.is_ascii_digit() || c == '-' || c == ',' {
                                self.verse_input.push(c);
                            }
                        }
                        InputFocus::VerseEnd => {
                            if c.is_ascii_digit() || c == '-' || c == ',' {
                                self.verse_end_input.push(c);
                            }
                        }
                        InputFocus::ReadCount => {
                            if c.is_ascii_digit() {
                                self.read_count_input.push(c);
                            }
                        }
                        InputFocus::Date => {
                            if c.is_ascii_digit() || c == '-' {
                                self.date_input.push(c);
                            }
                        }
                    }
                    self.error_message = None;
                    Ok(ManualAddAction::None)
                }
                _ => Ok(ManualAddAction::None),
            }
        }
    }

    pub fn add_reading(
        &mut self,
        progress: &mut ReadingProgress,
        bible: &'static crate::bible_structure::BibleStructure,
    ) -> Result<(), String> {
        if self.book_matches.is_empty() {
            return Err("Please select a book first".to_string());
        }

        let selected_book = self.book_matches[self.selected_book_index].clone();
        let chapter_str = self.chapter_input.clone();
        let verse_str = self.verse_input.clone();
        let verse_end_str = self.verse_end_input.clone();
        let read_count_str = self.read_count_input.clone();
        let date_str = self.date_input.clone();

        // Parse read count
        let read_count = if read_count_str.trim().is_empty() {
            1
        } else {
            read_count_str
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid read count: {}", read_count_str))?
        };

        // Parse date
        let last_read = if date_str.trim().is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d")
                    .map_err(|_| format!("Invalid date format: {}. Expected YYYY-MM-DD", date_str))?,
            )
        };

        // Get chapters for this book
        let chapters = bible
            .ot
            .get(&selected_book)
            .or_else(|| bible.nt.get(&selected_book))
            .ok_or_else(|| format!("Book '{}' not found", selected_book))?;

        // Handle empty chapter input (entire book)
        if chapter_str.trim().is_empty() {
            // Mark entire book as read
            for (chapter_idx, &max_verse) in chapters.iter().enumerate() {
                let chapter = (chapter_idx + 1) as u32;
                for verse in 1..=max_verse {
                    progress.mark_read_overwrite(
                        selected_book.clone(),
                        InsideBookBibleReference { chapter, verse },
                        read_count,
                        last_read,
                    );
                }
            }

            // Clear inputs and reset
            self.chapter_input = String::new();
            self.verse_input = String::new();
            self.verse_end_input = String::new();
            self.read_count_input = String::new();
            self.date_input = String::new();
            self.error_message = None;
            self.show_confirmation = false;
            self.input_focus = InputFocus::Chapter;

            return Ok(());
        }

        // Parse chapter(s) - handle ranges
        let (chapter_start, chapter_end) = if chapter_str.contains('-') {
            let parts: Vec<&str> = chapter_str.split('-').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid chapter range format: {}", chapter_str));
            }
            let start = parts[0]
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid chapter number: {}", parts[0]))?;
            let end = parts[1]
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid chapter number: {}", parts[1]))?;

            if start == 0 || start > chapters.len() as u32 {
                return Err(format!(
                    "Start chapter {} doesn't exist (max: {})",
                    start,
                    chapters.len()
                ));
            }
            if end == 0 || end > chapters.len() as u32 {
                return Err(format!(
                    "End chapter {} doesn't exist (max: {})",
                    end,
                    chapters.len()
                ));
            }
            if start > end {
                return Err(format!(
                    "Start chapter ({}) must be <= end chapter ({})",
                    start, end
                ));
            }

            (start, end)
        } else {
            let chapter = chapter_str
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("Invalid chapter: {}", chapter_str))?;

            if chapter == 0 || chapter > chapters.len() as u32 {
                return Err(format!(
                    "Chapter {} doesn't exist (max: {})",
                    chapter,
                    chapters.len()
                ));
            }

            (chapter, chapter)
        };

        // Process each chapter in the range
        for chapter in chapter_start..=chapter_end {
            let max_verse = chapters[chapter as usize - 1];

            // Determine which verse input to use
            let verse_input = if chapter == chapter_start {
                // Use start chapter verse input
                &verse_str
            } else if chapter == chapter_end && chapter_start != chapter_end {
                // Use end chapter verse input
                &verse_end_str
            } else {
                // Middle chapters: use empty (full chapter)
                ""
            };

            // Parse verses
            let verse_ranges = if verse_input.trim().is_empty() {
                // Full chapter
                vec![(1, max_verse)]
            } else {
                parse_verse_ranges(verse_input, max_verse)?
            };

            // Mark each verse as read (overwriting overlapping ranges)
            for (verse_start, verse_end) in verse_ranges {
                for verse in verse_start..=verse_end {
                    progress.mark_read_overwrite(
                        selected_book.clone(),
                        InsideBookBibleReference { chapter, verse },
                        read_count,
                        last_read,
                    );
                }
            }
        }

        // Clear inputs and reset
        self.chapter_input = String::new();
        self.verse_input = String::new();
        self.verse_end_input = String::new();
        self.read_count_input = String::new();
        self.date_input = String::new();
        self.error_message = None;
        self.show_confirmation = false;
        self.input_focus = InputFocus::Chapter;

        Ok(())
    }

    fn compute_book_matches(
        bible: &'static crate::bible_structure::BibleStructure,
        search_query: &str,
    ) -> Vec<String> {
        let all_books = get_all_books(bible);
        if search_query.is_empty() {
            all_books
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, String)> = all_books
                .into_iter()
                .filter_map(|book| {
                    matcher
                        .fuzzy_match(&book, search_query)
                        .map(|score| (score, book))
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by score descending
            scored.into_iter().map(|(_, book)| book).collect()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualAddAction {
    None,
    Cancel,
    AddReading,
}

