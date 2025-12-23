use chrono::{Local, NaiveDate};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::{
    prelude::*,
    widgets::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BibleStructure {
    ot: HashMap<String, Vec<u32>>,
    nt: HashMap<String, Vec<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
struct VerseRange {
    start: u32,
    end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReadingEntry {
    date: String, // YYYY-MM-DD format
    passages: Vec<Passage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Passage {
    book: String,
    chapter: u32,
    #[serde(with = "verse_range_format")]
    verses: Vec<VerseRange>,
}

mod verse_range_format {
    use super::VerseRange;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(ranges: &[VerseRange], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = ranges_to_string(ranges);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<VerseRange>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        string_to_ranges(&s).map_err(serde::de::Error::custom)
    }

    fn ranges_to_string(ranges: &[VerseRange]) -> String {
        ranges
            .iter()
            .map(|r| {
                if r.start == r.end {
                    r.start.to_string()
                } else {
                    format!("{}-{}", r.start, r.end)
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn string_to_ranges(s: &str) -> Result<Vec<VerseRange>, String> {
        let mut ranges = Vec::new();
        for part in s.split(',') {
            let part = part.trim();
            if part.contains('-') {
                let parts: Vec<&str> = part.split('-').collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid range format: {}", part));
                }
                let start = parts[0].trim().parse::<u32>()
                    .map_err(|_| format!("Invalid verse number: {}", parts[0]))?;
                let end = parts[1].trim().parse::<u32>()
                    .map_err(|_| format!("Invalid verse number: {}", parts[1]))?;
                ranges.push(VerseRange { start, end });
            } else {
                let verse = part.parse::<u32>()
                    .map_err(|_| format!("Invalid verse number: {}", part))?;
                ranges.push(VerseRange {
                    start: verse,
                    end: verse,
                });
            }
        }
        Ok(ranges)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProgressData {
    readings: Vec<ReadingEntry>,
}

#[derive(Debug, Clone)]
struct PassageStats {
    book: String,
    chapter: u32,
    verse_ranges: Vec<VerseRange>,
    count: u32,
    last_read: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
struct DashboardItem {
    stats: PassageStats,
    expanded: bool,
}

enum AppMode {
    Dashboard,
    Record {
        book_search: String,
        book_matches: Vec<String>,
        selected_book_index: usize,
        passage_input: String,
        passages: Vec<Passage>,
        error_message: Option<String>,
        input_focus: InputFocus, // Which field is being edited
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputFocus {
    Book,
    Passage,
}

struct App {
    running: bool,
    mode: AppMode,
    bible: BibleStructure,
    progress: ProgressData,
    dashboard_items: Vec<DashboardItem>,
    selected_index: usize,
    scroll_offset: usize,
}

impl App {
    fn new() -> Result<Self> {
        let bible = load_bible_structure()?;
        let progress = load_progress()?;
        let dashboard_items = compute_dashboard_stats(&bible, &progress)?;

        Ok(Self {
            running: true,
            mode: AppMode::Dashboard,
            bible,
            progress,
            dashboard_items,
            selected_index: 0,
            scroll_offset: 0,
        })
    }

    fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        match &self.mode {
            AppMode::Dashboard => self.render_dashboard(frame),
            AppMode::Record { .. } => self.render_record(frame),
        }
    }

    fn render_dashboard(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // List
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        // Header
        let header = Paragraph::new("Bible Reading Progress Dashboard")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(header, chunks[0]);

        // List of passages
        if self.dashboard_items.is_empty() {
            let empty_msg = Paragraph::new("No readings recorded yet.\nPress 'r' to record your first reading!")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(empty_msg, chunks[1]);
        } else {
            // Update scroll offset to keep selected item visible
            let visible_height = chunks[1].height as usize - 2; // Account for borders
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            } else if self.selected_index >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected_index - visible_height + 1;
            }

            let items: Vec<ListItem> = self
                .dashboard_items
                .iter()
                .enumerate()
                .skip(self.scroll_offset)
                .take(visible_height)
                .map(|(idx, item)| {
                    let is_selected = idx == self.selected_index;
                    let stats = &item.stats;
                    let verse_str = format_verse_ranges(&stats.verse_ranges, &self.bible, &stats.book, stats.chapter);
                    let last_read_str = format_last_read(stats.last_read);
                    
                    let line = if item.expanded {
                        format!(
                            "▼ {} {}:{} | Read {} time(s) | Last: {}",
                            stats.book, stats.chapter, verse_str, stats.count, last_read_str
                        )
                    } else {
                        format!(
                            "▶ {} {}:{} | Read {} time(s) | Last: {}",
                            stats.book, stats.chapter, verse_str, stats.count, last_read_str
                        )
                    };

                    let style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    ListItem::new(line).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Passages (↑↓: navigate, Enter: expand, r: record, q: quit)"),
                );

            frame.render_widget(list, chunks[1]);

            // Render expanded details if selected item is expanded
            if let Some(item) = self.dashboard_items.get(self.selected_index) {
                if item.expanded {
                    let details = format_passage_details(&item.stats, &self.bible);
                    let detail_paragraph = Paragraph::new(details)
                        .style(Style::default().fg(Color::White))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Details")
                                .border_style(Style::default().fg(Color::Green)),
                        );
                    
                    // Position details below the list
                    let detail_area = Rect {
                        x: chunks[1].x,
                        y: chunks[1].y + chunks[1].height - 10,
                        width: chunks[1].width,
                        height: 10.min(chunks[1].height),
                    };
                    frame.render_widget(detail_paragraph, detail_area);
                }
            }
        }

        // Footer
        let footer_text = if self.dashboard_items.is_empty() {
            "r: Record | q: Quit"
        } else {
            "↑↓: Navigate | Enter: Expand/Collapse | r: Record | q: Quit"
        };
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[2]);
    }

    fn render_record(&mut self, frame: &mut Frame) {
        if let AppMode::Record {
            book_search,
            book_matches,
            selected_book_index,
            passage_input,
            passages,
            error_message,
            input_focus,
        } = &mut self.mode
        {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Book search
                    Constraint::Length(8), // Book matches list
                    Constraint::Length(3), // Passage input
                    Constraint::Min(0),    // Passages list / error / help
                    Constraint::Length(3), // Footer
                ])
                .split(frame.area());

            // Header
            let today = Local::now().date_naive();
            let today_str = today.format("%Y-%m-%d").to_string();
            let header = Paragraph::new(format!("Record Reading - {}", today_str))
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                );
            frame.render_widget(header, chunks[0]);

            // Book search field
            let book_style = if *input_focus == InputFocus::Book {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let book_widget = Paragraph::new(book_search.as_str())
                .style(book_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Book (Tab: switch to passage)")
                        .border_style(if *input_focus == InputFocus::Book {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                );
            frame.render_widget(book_widget, chunks[1]);

            // Book matches list
            if !book_matches.is_empty() {
                let items: Vec<ListItem> = book_matches
                    .iter()
                    .enumerate()
                    .map(|(idx, book)| {
                        let style = if idx == *selected_book_index {
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
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Matches (↑↓: select)"));
                frame.render_widget(list, chunks[2]);
            } else {
                let empty = Paragraph::new("No matches")
                    .style(Style::default().fg(Color::Gray))
                    .block(Block::default().borders(Borders::ALL).title("Matches"));
                frame.render_widget(empty, chunks[2]);
            }

            // Passage input field
            let passage_style = if *input_focus == InputFocus::Passage {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let passage_widget = Paragraph::new(passage_input.as_str())
                .style(passage_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Passage (e.g., 1:1-1:20, 1-5, 1) - Tab: switch to book")
                        .border_style(if *input_focus == InputFocus::Passage {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                );
            frame.render_widget(passage_widget, chunks[3]);

            // Error message or passages list
            if let Some(error) = error_message {
                let error_widget = Paragraph::new(error.clone())
                    .style(Style::default().fg(Color::Red))
                    .block(Block::default().borders(Borders::ALL).title("Error"));
                frame.render_widget(error_widget, chunks[4]);
            } else if !passages.is_empty() {
                // Show added passages
                let passage_items: Vec<ListItem> = passages
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| {
                        let verse_str = format_verse_ranges(&p.verses, &self.bible, &p.book, p.chapter);
                        ListItem::new(format!("{}. {} {}:{}", idx + 1, p.book, p.chapter, verse_str))
                    })
                    .collect();
                let passages_list = List::new(passage_items)
                    .block(Block::default().borders(Borders::ALL).title("Added Passages"));
                frame.render_widget(passages_list, chunks[4]);
            } else {
                let help = Paragraph::new("Examples: '1:1-1:20' (chapter 1, verses 1-20), '1-5' (chapter 1, verses 1-5), '1' (full chapter 1)")
                    .style(Style::default().fg(Color::Gray))
                    .block(Block::default().borders(Borders::ALL).title("Help"));
                frame.render_widget(help, chunks[4]);
            }

            // Footer
            let footer = Paragraph::new("Tab: Switch field | ↑↓: Select book | Enter: Add passage | s: Save | Esc: Cancel")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, chunks[5]);
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                match &mut self.mode {
                    AppMode::Dashboard => self.handle_dashboard_key(key),
                    AppMode::Record { .. } => self.handle_record_key(key)?,
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_dashboard_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q') | KeyCode::Esc) => self.quit(),
            (_, KeyCode::Char('r')) => self.start_record_mode(),
            (_, KeyCode::Up) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            (_, KeyCode::Down) => {
                if self.selected_index < self.dashboard_items.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            (_, KeyCode::Enter) => {
                if let Some(item) = self.dashboard_items.get_mut(self.selected_index) {
                    item.expanded = !item.expanded;
                }
            }
            _ => {}
        }
    }

    fn handle_record_key(&mut self, key: KeyEvent) -> Result<()> {
        if let AppMode::Record {
            book_search,
            book_matches,
            selected_book_index,
            passage_input,
            passages,
            error_message,
            input_focus,
        } = &mut self.mode
        {
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc) => {
                    self.mode = AppMode::Dashboard;
                }
                (_, KeyCode::Tab) => {
                    // Switch focus between book and passage
                    *input_focus = match *input_focus {
                        InputFocus::Book => InputFocus::Passage,
                        InputFocus::Passage => InputFocus::Book,
                    };
                    *error_message = None;
                }
                (_, KeyCode::Up) if *input_focus == InputFocus::Book => {
                    if *selected_book_index > 0 {
                        *selected_book_index -= 1;
                    }
                }
                (_, KeyCode::Down) if *input_focus == InputFocus::Book => {
                    if *selected_book_index < book_matches.len().saturating_sub(1) {
                        *selected_book_index += 1;
                    }
                }
                (_, KeyCode::Enter) => {
                    if *input_focus == InputFocus::Book {
                        // Select the book and switch to passage input
                        if !book_matches.is_empty() {
                            let selected_book = book_matches[*selected_book_index].clone();
                            *book_search = selected_book.clone();
                            *input_focus = InputFocus::Passage;
                            let search_query = book_search.clone();
                            let new_matches = self.compute_book_matches(&search_query);
                            *book_matches = new_matches;
                        }
                    } else {
                        // Add passage
                        if book_matches.is_empty() {
                            *error_message = Some("Please select a book first".to_string());
                        } else {
                            let selected_book = book_matches[*selected_book_index].clone();
                            let passage_input_clone = passage_input.clone();
                            match parse_natural_passage(&passage_input_clone, &selected_book, &self.bible) {
                                Ok(mut new_passages) => {
                                    passages.append(&mut new_passages);
                                    *passage_input = String::new();
                                    *error_message = None;
                                }
                                Err(e) => {
                                    *error_message = Some(e);
                                }
                            }
                        }
                    }
                }
                (_, KeyCode::Backspace) => {
                    if *input_focus == InputFocus::Book {
                        book_search.pop();
                        *selected_book_index = 0;
                        let search_query = book_search.clone();
                        let new_matches = self.compute_book_matches(&search_query);
                        *book_matches = new_matches;
                    } else {
                        passage_input.pop();
                    }
                    *error_message = None;
                }
                (_, KeyCode::Char(c)) if c.is_ascii() && !c.is_control() => {
                    if *input_focus == InputFocus::Book {
                        book_search.push(c);
                        *selected_book_index = 0;
                        let search_query = book_search.clone();
                        let new_matches = self.compute_book_matches(&search_query);
                        *book_matches = new_matches;
                    } else {
                        passage_input.push(c);
                    }
                    *error_message = None;
                }
                (_, KeyCode::Char('s')) => {
                    // Save and return
                    if !passages.is_empty() {
                        let today = Local::now().date_naive();
                        let today_str = today.format("%Y-%m-%d").to_string();
                        self.progress.readings.push(ReadingEntry {
                            date: today_str,
                            passages: passages.clone(),
                        });
                        save_progress(&self.progress)?;
                        self.dashboard_items = compute_dashboard_stats(&self.bible, &self.progress)?;
                        self.mode = AppMode::Dashboard;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn compute_book_matches(&self, search_query: &str) -> Vec<String> {
        let all_books = get_all_books(&self.bible);
        if search_query.is_empty() {
            all_books
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, String)> = all_books
                .into_iter()
                .filter_map(|book| {
                    matcher.fuzzy_match(&book, search_query).map(|score| (score, book))
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by score descending
            scored.into_iter().map(|(_, book)| book).collect()
        }
    }

    fn start_record_mode(&mut self) {
        let books = get_all_books(&self.bible);
        self.mode = AppMode::Record {
            book_search: String::new(),
            book_matches: books,
            selected_book_index: 0,
            passage_input: String::new(),
            passages: Vec::new(),
            error_message: None,
            input_focus: InputFocus::Book,
        };
    }

    fn quit(&mut self) {
        self.running = false;
    }
}

fn format_verse_ranges(ranges: &[VerseRange], bible: &BibleStructure, book: &str, chapter: u32) -> String {
    if ranges.len() == 1
        && ranges[0].start == 1
        && ranges[0].end == get_max_verse(bible, book, chapter)
    {
        "full chapter".to_string()
    } else {
        ranges
            .iter()
            .map(|r| {
                if r.start == r.end {
                    r.start.to_string()
                } else {
                    format!("{}-{}", r.start, r.end)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_last_read(last_read: Option<NaiveDate>) -> String {
    if let Some(date) = last_read {
        let today = Local::now().date_naive();
        let days_ago = (today - date).num_days();
        if days_ago == 0 {
            "today".to_string()
        } else if days_ago == 1 {
            "yesterday".to_string()
        } else if days_ago < 30 {
            format!("{} days ago", days_ago)
        } else if days_ago < 365 {
            format!("{} months ago", days_ago / 30)
        } else {
            format!("{} years ago", days_ago / 365)
        }
    } else {
        "never".to_string()
    }
}

fn format_passage_details(stats: &PassageStats, bible: &BibleStructure) -> String {
    let verse_str = format_verse_ranges(&stats.verse_ranges, bible, &stats.book, stats.chapter);
    let last_read_str = format_last_read(stats.last_read);
    format!(
        "Book: {}\nChapter: {}\nVerses: {}\nTimes read: {}\nLast read: {}",
        stats.book, stats.chapter, verse_str, stats.count, last_read_str
    )
}

fn compute_dashboard_stats(_bible: &BibleStructure, progress: &ProgressData) -> Result<Vec<DashboardItem>> {
    let mut stats_map: HashMap<(String, u32, Vec<VerseRange>), (u32, Option<NaiveDate>)> =
        HashMap::new();

    for entry in &progress.readings {
        let date = NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d")
            .map_err(|e| color_eyre::eyre::eyre!("Invalid date format: {}", e))?;

        for passage in &entry.passages {
            let key = (
                passage.book.clone(),
                passage.chapter,
                passage.verses.clone(),
            );
            let (count, last_read) = stats_map.entry(key).or_insert((0, None));
            *count += 1;
            if last_read.is_none() || date > last_read.unwrap() {
                *last_read = Some(date);
            }
        }
    }

    let mut stats: Vec<PassageStats> = stats_map
        .into_iter()
        .map(|((book, chapter, verse_ranges), (count, last_read))| {
            PassageStats {
                book,
                chapter,
                verse_ranges,
                count,
                last_read,
            }
        })
        .collect();

    stats.sort_by(|a, b| {
        a.book
            .cmp(&b.book)
            .then(a.chapter.cmp(&b.chapter))
            .then(a.verse_ranges[0].start.cmp(&b.verse_ranges[0].start))
    });

    Ok(stats
        .into_iter()
        .map(|stats| DashboardItem {
            stats,
            expanded: false,
        })
        .collect())
}

fn load_bible_structure() -> Result<BibleStructure> {
    let content = fs::read_to_string("bible_structure.json")?;
    let structure: BibleStructure = serde_json::from_str(&content)?;
    Ok(structure)
}

fn get_progress_file_path() -> PathBuf {
    PathBuf::from("reading_progress.yaml")
}

fn load_progress() -> Result<ProgressData> {
    let path = get_progress_file_path();
    if !path.exists() {
        return Ok(ProgressData {
            readings: Vec::new(),
        });
    }
    let content = fs::read_to_string(&path)?;
    let progress: ProgressData = serde_yaml::from_str(&content)?;
    Ok(progress)
}

fn save_progress(progress: &ProgressData) -> Result<()> {
    let path = get_progress_file_path();
    let content = serde_yaml::to_string(progress)?;
    fs::write(&path, content)?;
    Ok(())
}

fn get_all_books(bible: &BibleStructure) -> Vec<String> {
    let mut books: Vec<String> = Vec::new();
    books.extend(bible.ot.keys().cloned());
    books.extend(bible.nt.keys().cloned());
    books.sort();
    books
}

// Parse natural passage formats like:
// - "1:1-1:20" -> chapter 1, verses 1-20
// - "1-5" -> chapter 1, verses 1-5
// - "1-2:20" -> chapter 1 verse 1 to chapter 2 verse 20
// - "1" -> chapter 1, full chapter
// - "1:5" -> chapter 1, verse 5
// - "1:1-20" -> chapter 1, verses 1-20
fn parse_natural_passage(input: &str, book: &str, bible: &BibleStructure) -> Result<Vec<Passage>, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Passage cannot be empty".to_string());
    }

    // Get chapters for this book
    let chapters = bible
        .ot.get(book)
        .or_else(|| bible.nt.get(book))
        .ok_or_else(|| format!("Book '{}' not found", book))?;

    // Split by comma to handle multiple passages
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    let mut passages = Vec::new();

    for part in parts {
        let mut part_passages = parse_single_passage(part, book, chapters)?;
        passages.append(&mut part_passages);
    }

    Ok(passages)
}

fn parse_single_passage(input: &str, book: &str, chapters: &[u32]) -> Result<Vec<Passage>, String> {
    // Format: [chapter][:verse_start][-verse_end] or [chapter_start][-chapter_end][:verse_end]
    // Examples:
    // - "1" -> chapter 1, full chapter
    // - "1:5" -> chapter 1, verse 5
    // - "1:1-20" -> chapter 1, verses 1-20
    // - "1-5" -> chapter 1, verses 1-5 (assume verses, not chapters)
    // - "1:1-1:20" -> chapter 1, verses 1-20
    // - "1-2:20" -> chapter 1 verse 1 to chapter 2 verse 20

    if input.contains(':') {
        // Has verse specification
        if input.contains('-') {
            // Has range
            if input.matches(':').count() == 2 {
                // Format: "1:1-2:20" (chapter:verse-chapter:verse)
                let parts: Vec<&str> = input.split('-').collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid format: {}", input));
                }
                let start_part = parts[0].trim();
                let end_part = parts[1].trim();
                
                let (start_ch, start_v) = parse_chapter_verse(start_part)?;
                let (end_ch, end_v) = parse_chapter_verse(end_part)?;
                
                if start_ch > end_ch || (start_ch == end_ch && start_v > end_v) {
                    return Err(format!("Invalid range: start must be before end"));
                }
                
                if end_ch > chapters.len() as u32 {
                    return Err(format!("Chapter {} doesn't exist (max: {})", end_ch, chapters.len()));
                }

                // Create passages for each chapter
                let mut passages = Vec::new();
                for ch in start_ch..=end_ch {
                    let max_verse = chapters[ch as usize - 1];
                    let verse_start = if ch == start_ch { start_v } else { 1 };
                    let verse_end = if ch == end_ch { end_v } else { max_verse };
                    
                    if verse_start > max_verse || verse_end > max_verse {
                        return Err(format!("Verse out of range for chapter {} (max: {})", ch, max_verse));
                    }
                    
                    passages.push(Passage {
                        book: book.to_string(),
                        chapter: ch,
                        verses: vec![VerseRange { start: verse_start, end: verse_end }],
                    });
                }
                
                // Handle multi-chapter ranges
                if start_ch == end_ch {
                    return Ok(vec![Passage {
                        book: book.to_string(),
                        chapter: start_ch,
                        verses: vec![VerseRange { start: start_v, end: end_v }],
                    }]);
                } else {
                    // Multi-chapter range - create passages for each chapter
                    let mut result = Vec::new();
                    for ch in start_ch..=end_ch {
                        let max_verse = chapters[ch as usize - 1];
                        let verse_start = if ch == start_ch { start_v } else { 1 };
                        let verse_end = if ch == end_ch { end_v } else { max_verse };
                        
                        if verse_start > max_verse || verse_end > max_verse {
                            return Err(format!("Verse out of range for chapter {} (max: {})", ch, max_verse));
                        }
                        
                        result.push(Passage {
                            book: book.to_string(),
                            chapter: ch,
                            verses: vec![VerseRange { start: verse_start, end: verse_end }],
                        });
                    }
                    return Ok(result);
                }
            } else {
                // Format: "1:1-20" or "1-2:20"
                let colon_pos = input.find(':').unwrap();
                let before_colon = &input[..colon_pos];
                let after_colon = &input[colon_pos + 1..];
                
                if after_colon.contains('-') {
                    // Format: "1:1-20"
                    let verse_parts: Vec<&str> = after_colon.split('-').collect();
                    if verse_parts.len() != 2 {
                        return Err(format!("Invalid verse range: {}", after_colon));
                    }
                    let chapter = before_colon.parse::<u32>()
                        .map_err(|_| format!("Invalid chapter: {}", before_colon))?;
                    let verse_start = verse_parts[0].parse::<u32>()
                        .map_err(|_| format!("Invalid verse: {}", verse_parts[0]))?;
                    let verse_end = verse_parts[1].parse::<u32>()
                        .map_err(|_| format!("Invalid verse: {}", verse_parts[1]))?;
                    
                    if chapter > chapters.len() as u32 {
                        return Err(format!("Chapter {} doesn't exist (max: {})", chapter, chapters.len()));
                    }
                    let max_verse = chapters[chapter as usize - 1];
                    if verse_start > verse_end || verse_end > max_verse {
                        return Err(format!("Invalid verse range: {}-{} (max: {})", verse_start, verse_end, max_verse));
                    }
                    
                    return Ok(vec![Passage {
                        book: book.to_string(),
                        chapter,
                        verses: vec![VerseRange { start: verse_start, end: verse_end }],
                    }]);
                } else {
                    // Format: "1:5" (single verse)
                    let chapter = before_colon.parse::<u32>()
                        .map_err(|_| format!("Invalid chapter: {}", before_colon))?;
                    let verse = after_colon.parse::<u32>()
                        .map_err(|_| format!("Invalid verse: {}", after_colon))?;
                    
                    if chapter > chapters.len() as u32 {
                        return Err(format!("Chapter {} doesn't exist (max: {})", chapter, chapters.len()));
                    }
                    let max_verse = chapters[chapter as usize - 1];
                    if verse > max_verse {
                        return Err(format!("Verse {} doesn't exist in chapter {} (max: {})", verse, chapter, max_verse));
                    }
                    
                    return Ok(vec![Passage {
                        book: book.to_string(),
                        chapter,
                        verses: vec![VerseRange { start: verse, end: verse }],
                    }]);
                }
            }
        } else {
            // Format: "1:5" (single verse)
            let parts: Vec<&str> = input.split(':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid format: {}", input));
            }
            let chapter = parts[0].parse::<u32>()
                .map_err(|_| format!("Invalid chapter: {}", parts[0]))?;
            let verse = parts[1].parse::<u32>()
                .map_err(|_| format!("Invalid verse: {}", parts[1]))?;
            
            if chapter > chapters.len() as u32 {
                return Err(format!("Chapter {} doesn't exist (max: {})", chapter, chapters.len()));
            }
            let max_verse = chapters[chapter as usize - 1];
            if verse > max_verse {
                return Err(format!("Verse {} doesn't exist in chapter {} (max: {})", verse, chapter, max_verse));
            }
            
            return Ok(vec![Passage {
                book: book.to_string(),
                chapter,
                verses: vec![VerseRange { start: verse, end: verse }],
            }]);
        }
    } else {
        // No colon - could be "1" (full chapter) or "1-5" (verses 1-5 of chapter 1)
        if input.contains('-') {
            // Format: "1-5" - assume this means chapter 1, verses 1-5
            // But we need to know which chapter... let's assume the most recent chapter or require chapter prefix
            // Actually, let's require explicit chapter: "1:1-5" for clarity
            // But the user said "1-5" should work, so let's assume chapter 1
            // Wait, but we don't know which chapter. Let's require "1:1-5" format.
            // Actually, re-reading the requirement: "1-5" - this could mean verses 1-5 of the current/default chapter
            // For simplicity, let's assume it means chapter 1, verses 1-5
            let parts: Vec<&str> = input.split('-').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid format: {}. Use format like '1:1-5' for verses", input));
            }
            // Assume chapter 1
            let chapter = 1;
            let verse_start = parts[0].parse::<u32>()
                .map_err(|_| format!("Invalid verse: {}", parts[0]))?;
            let verse_end = parts[1].parse::<u32>()
                .map_err(|_| format!("Invalid verse: {}", parts[1]))?;
            
            if chapter > chapters.len() as u32 {
                return Err(format!("Chapter {} doesn't exist", chapter));
            }
            let max_verse = chapters[chapter as usize - 1];
            if verse_start > verse_end || verse_end > max_verse {
                return Err(format!("Invalid verse range: {}-{} (max: {})", verse_start, verse_end, max_verse));
            }
            
            return Ok(vec![Passage {
                book: book.to_string(),
                chapter,
                verses: vec![VerseRange { start: verse_start, end: verse_end }],
            }]);
        } else {
            // Format: "1" -> full chapter
            let chapter = input.parse::<u32>()
                .map_err(|_| format!("Invalid chapter: {}", input))?;
            
            if chapter > chapters.len() as u32 {
                return Err(format!("Chapter {} doesn't exist (max: {})", chapter, chapters.len()));
            }
            let max_verse = chapters[chapter as usize - 1];
            
            return Ok(vec![Passage {
                book: book.to_string(),
                chapter,
                verses: vec![VerseRange { start: 1, end: max_verse }],
            }]);
        }
    }
}

fn parse_chapter_verse(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Expected format 'chapter:verse', got: {}", s));
    }
    let chapter = parts[0].parse::<u32>()
        .map_err(|_| format!("Invalid chapter: {}", parts[0]))?;
    let verse = parts[1].parse::<u32>()
        .map_err(|_| format!("Invalid verse: {}", parts[1]))?;
    Ok((chapter, verse))
}

fn parse_verse_input(input: &str, max_verses: u32) -> Result<Vec<VerseRange>, String> {
    if input.trim().is_empty() {
        return Ok(vec![VerseRange {
            start: 1,
            end: max_verses,
        }]);
    }

    let mut ranges = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid range format: {}", part));
            }
            let start = parts[0].trim().parse::<u32>()
                .map_err(|_| format!("Invalid verse number: {}", parts[0]))?;
            let end = parts[1].trim().parse::<u32>()
                .map_err(|_| format!("Invalid verse number: {}", parts[1]))?;
            if start > end || end > max_verses {
                return Err(format!("Invalid range: {}-{} (max: {})", start, end, max_verses));
            }
            ranges.push(VerseRange { start, end });
        } else {
            let verse = part.parse::<u32>()
                .map_err(|_| format!("Invalid verse number: {}", part))?;
            if verse > max_verses {
                return Err(format!("Invalid verse: {} (max: {})", verse, max_verses));
            }
            ranges.push(VerseRange {
                start: verse,
                end: verse,
            });
        }
    }
    Ok(ranges)
}

fn get_max_verse(bible: &BibleStructure, book: &str, chapter: u32) -> u32 {
    if let Some(chapters) = bible.ot.get(book) {
        if chapter <= chapters.len() as u32 {
            return chapters[chapter as usize - 1];
        }
    }
    if let Some(chapters) = bible.nt.get(book) {
        if chapter <= chapters.len() as u32 {
            return chapters[chapter as usize - 1];
        }
    }
    0
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let mut app = App::new()?;
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
