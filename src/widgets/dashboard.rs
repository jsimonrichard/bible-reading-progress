use chrono::NaiveDate;
use ratatui::{prelude::*, widgets::*};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::progress::ReadingProgress;
use crate::widgets::tree_builder::{
    build_dashboard_tree_items, collect_recent_reads, RecentReadEntry, TreeId,
};

pub struct DashboardWidget {
    pub tree_items: Vec<TreeItem<'static, TreeId>>,
    pub tree_state: TreeState<TreeId>,
    pub show_only_unread: bool,
    pub recent_reads: Vec<(NaiveDate, Vec<RecentReadEntry>)>,
}

impl DashboardWidget {
    pub fn new(
        bible: &'static crate::bible_structure::BibleStructure,
        progress: &ReadingProgress,
    ) -> Self {
        let tree_items = build_dashboard_tree_items(bible, progress);
        let recent_reads = collect_recent_reads(progress);
        let mut tree_state = TreeState::default();
        tree_state.select_first();

        Self {
            tree_items,
            tree_state,
            show_only_unread: false,
            recent_reads,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        // Calculate recent reads section height (if there are recent reads)
        let recent_reads_height = if self.recent_reads.is_empty() {
            0
        } else {
            // 2 for borders + 1 line per date group (date header + entries on same line)
            (self.recent_reads.len() as u16) + 2
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                   // Header
                Constraint::Length(recent_reads_height), // Recent reads (dynamic)
                Constraint::Min(0),                      // Tree
                Constraint::Length(3),                   // Footer
            ])
            .split(frame.area());

        // Header
        let header_text = "Bible Reading Progress";
        let header = Paragraph::new(header_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(header, chunks[0]);

        // Recent reads section
        if !self.recent_reads.is_empty() {
            let recent_lines = self.format_recent_reads();
            let recent_reads_widget = Paragraph::new(recent_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Recent Reads")
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            frame.render_widget(recent_reads_widget, chunks[1]);
        }

        // Render tree
        let tree = Tree::new(&self.tree_items[..])
            .expect("error rendering tree")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if self.show_only_unread {
                        "Bible Structure (Space/→: expand, ←: collapse, ↑↓: navigate, r: record, m: manual add, q: quit)"
                    } else {
                        "Bible Structure (Space/→: expand, ←: collapse, ↑↓: navigate, r: record, m: manual add, q: quit)"
                    }),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(tree, chunks[2], &mut self.tree_state);

        // Footer
        let footer_text =
            "Space/→: Expand | ←: Collapse | ↑↓: Navigate | r: Record | m: Manual Add | q: Quit";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[3]);
    }

    fn format_recent_reads(&self) -> Vec<Line<'static>> {
        use chrono::Utc;

        let today = Utc::now().date_naive();
        let mut lines = Vec::new();

        for (date, entries) in &self.recent_reads {
            // Format date label
            let days_ago = today.signed_duration_since(*date).num_days();
            let date_label = match days_ago {
                0 => "Today".to_string(),
                1 => "Yesterday".to_string(),
                _ => format!("{} days ago ({})", days_ago, date.format("%Y-%m-%d")),
            };

            // Group entries by book and consolidate contiguous chapters
            let entries_text = Self::format_entries_with_ranges(entries);

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", date_label),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(entries_text),
            ]));
        }

        lines
    }

    /// Format entries by consolidating contiguous chapters into ranges
    /// e.g., "Psalms 23, Psalms 24, Psalms 25" becomes "Psalms 23-25"
    fn format_entries_with_ranges(entries: &[RecentReadEntry]) -> String {
        use std::collections::BTreeMap;

        // Group chapters by book, maintaining order of first appearance
        let mut book_order: Vec<String> = Vec::new();
        let mut book_chapters: BTreeMap<String, Vec<u32>> = BTreeMap::new();

        for entry in entries {
            if !book_chapters.contains_key(&entry.book) {
                book_order.push(entry.book.clone());
            }
            book_chapters
                .entry(entry.book.clone())
                .or_default()
                .push(entry.chapter);
        }

        // Sort chapters within each book and consolidate into ranges
        let mut formatted_parts: Vec<String> = Vec::new();

        for book in &book_order {
            if let Some(chapters) = book_chapters.get_mut(book) {
                chapters.sort();
                chapters.dedup();

                // Find contiguous ranges
                let ranges = Self::find_contiguous_ranges(chapters);

                for (start, end) in ranges {
                    if start == end {
                        formatted_parts.push(format!("{} {}", book, start));
                    } else {
                        formatted_parts.push(format!("{} {}-{}", book, start, end));
                    }
                }
            }
        }

        formatted_parts.join(", ")
    }

    /// Find contiguous ranges in a sorted list of chapters
    /// Returns a list of (start, end) tuples
    fn find_contiguous_ranges(chapters: &[u32]) -> Vec<(u32, u32)> {
        if chapters.is_empty() {
            return Vec::new();
        }

        let mut ranges = Vec::new();
        let mut range_start = chapters[0];
        let mut range_end = chapters[0];

        for &chapter in &chapters[1..] {
            if chapter == range_end + 1 {
                // Extend current range
                range_end = chapter;
            } else {
                // Start new range
                ranges.push((range_start, range_end));
                range_start = chapter;
                range_end = chapter;
            }
        }

        // Don't forget the last range
        ranges.push((range_start, range_end));

        ranges
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DashboardAction {
        match (key.modifiers, key.code) {
            (_, crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc) => {
                DashboardAction::Quit
            }
            (_, crossterm::event::KeyCode::Char('r')) => DashboardAction::StartRecord,
            (_, crossterm::event::KeyCode::Char('m')) => DashboardAction::StartManualAdd,
            (_, crossterm::event::KeyCode::Char('u')) => {
                self.show_only_unread = !self.show_only_unread;
                DashboardAction::None
            }
            (_, crossterm::event::KeyCode::Up) => {
                self.tree_state.key_up();
                DashboardAction::None
            }
            (_, crossterm::event::KeyCode::Down) => {
                self.tree_state.key_down();
                DashboardAction::None
            }
            (_, crossterm::event::KeyCode::Left) => {
                self.tree_state.key_left();
                DashboardAction::None
            }
            (
                _,
                crossterm::event::KeyCode::Right
                | crossterm::event::KeyCode::Char(' ')
                | crossterm::event::KeyCode::Enter,
            ) => {
                self.tree_state.toggle_selected();
                DashboardAction::None
            }
            _ => DashboardAction::None,
        }
    }

    pub fn update_tree(
        &mut self,
        bible: &'static crate::bible_structure::BibleStructure,
        progress: &ReadingProgress,
    ) {
        self.tree_items = build_dashboard_tree_items(bible, progress);
        self.recent_reads = collect_recent_reads(progress);
        self.tree_state = TreeState::default();
        self.tree_state.select_first();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardAction {
    None,
    Quit,
    StartRecord,
    StartManualAdd,
}
