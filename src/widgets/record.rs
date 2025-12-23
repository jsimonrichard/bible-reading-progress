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
}

pub struct RecordWidget {
    pub book_search: String,
    pub book_matches: Vec<String>,
    pub selected_book_index: usize,
    pub chapter_input: String,
    pub verse_input: String,
    pub error_message: Option<String>,
    pub input_focus: InputFocus,
}

impl RecordWidget {
    pub fn new(bible: &'static crate::bible_structure::BibleStructure) -> Self {
        let books = get_all_books(bible);
        Self {
            book_search: String::new(),
            book_matches: books,
            selected_book_index: 0,
            chapter_input: String::new(),
            verse_input: String::new(),
            error_message: None,
            input_focus: InputFocus::Book,
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
                Constraint::Length(3), // Verse input
                Constraint::Min(0),    // Error / help
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        // Header
        let header = Paragraph::new("Record Reading")
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
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
                    .title("Chapter")
                    .border_style(if self.input_focus == InputFocus::Chapter {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );
        frame.render_widget(chapter_widget, chunks[3]);

        // Verse input field
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

        // Error message or help
        if let Some(error) = &self.error_message {
            let error_widget = Paragraph::new(error.clone())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(error_widget, chunks[5]);
        } else {
            let help = Paragraph::new("Enter a verse number (e.g., 1), a range (e.g., 1-5), or leave empty for the full chapter")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("Help"));
            frame.render_widget(help, chunks[5]);
        }

        // Footer
        let footer = Paragraph::new(
            "Tab: Next field | Shift+Tab: Previous field | ↑↓: Select book | Enter: Add | s: Save | Esc: Cancel",
        )
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[6]);
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        bible: &'static crate::bible_structure::BibleStructure,
    ) -> Result<RecordAction> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => Ok(RecordAction::Cancel),
            (_, KeyCode::Tab) => {
                // Navigate forward through input fields
                self.input_focus = match self.input_focus {
                    InputFocus::Book => InputFocus::Chapter,
                    InputFocus::Chapter => InputFocus::Verse,
                    InputFocus::Verse => InputFocus::Book,
                };
                self.error_message = None;
                Ok(RecordAction::None)
            }
            (_, KeyCode::BackTab) => {
                // Navigate backward through input fields
                self.input_focus = match self.input_focus {
                    InputFocus::Book => InputFocus::Verse,
                    InputFocus::Chapter => InputFocus::Book,
                    InputFocus::Verse => InputFocus::Chapter,
                };
                self.error_message = None;
                Ok(RecordAction::None)
            }
            (_, KeyCode::Up) if self.input_focus == InputFocus::Book => {
                if self.selected_book_index > 0 {
                    self.selected_book_index -= 1;
                }
                Ok(RecordAction::None)
            }
            (_, KeyCode::Down) if self.input_focus == InputFocus::Book => {
                if self.selected_book_index < self.book_matches.len().saturating_sub(1) {
                    self.selected_book_index += 1;
                }
                Ok(RecordAction::None)
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
                    Ok(RecordAction::None)
                } else if self.input_focus == InputFocus::Chapter {
                    // Move to verse input
                    self.input_focus = InputFocus::Verse;
                    Ok(RecordAction::None)
                } else {
                    // Add the reading
                    if self.book_matches.is_empty() {
                        self.error_message = Some("Please select a book first".to_string());
                        Ok(RecordAction::None)
                    } else {
                        Ok(RecordAction::AddReading)
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
                }
                self.error_message = None;
                Ok(RecordAction::None)
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
                        if c.is_ascii_digit() {
                            self.chapter_input.push(c);
                        }
                    }
                    InputFocus::Verse => {
                        if c.is_ascii_digit() || c == '-' || c == ',' {
                            self.verse_input.push(c);
                        }
                    }
                }
                self.error_message = None;
                Ok(RecordAction::None)
            }
            _ => Ok(RecordAction::None),
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

        // Parse chapter
        let chapter = chapter_str
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("Invalid chapter: {}", chapter_str))?;

        // Get chapters for this book
        let chapters = bible
            .ot
            .get(&selected_book)
            .or_else(|| bible.nt.get(&selected_book))
            .ok_or_else(|| format!("Book '{}' not found", selected_book))?;

        if chapter == 0 || chapter > chapters.len() as u32 {
            return Err(format!(
                "Chapter {} doesn't exist (max: {})",
                chapter,
                chapters.len()
            ));
        }

        let max_verse = chapters[chapter as usize - 1];

        // Parse verses
        let verse_ranges = if verse_str.trim().is_empty() {
            // Full chapter
            vec![(1, max_verse)]
        } else {
            parse_verse_ranges(&verse_str, max_verse)?
        };

        // Mark each verse as read
        for (verse_start, verse_end) in verse_ranges {
            for verse in verse_start..=verse_end {
                progress.mark_read(
                    selected_book.clone(),
                    InsideBookBibleReference { chapter, verse },
                );
            }
        }

        // Clear inputs and reset
        self.chapter_input = String::new();
        self.verse_input = String::new();
        self.error_message = None;
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
pub enum RecordAction {
    None,
    Cancel,
    AddReading,
}
