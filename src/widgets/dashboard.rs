use ratatui::{prelude::*, widgets::*};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::progress::ReadingProgress;
use crate::widgets::tree_builder::{build_dashboard_tree_items, TreeId};

pub struct DashboardWidget {
    pub tree_items: Vec<TreeItem<'static, TreeId>>,
    pub tree_state: TreeState<TreeId>,
    pub show_only_unread: bool,
}

impl DashboardWidget {
    pub fn new(
        bible: &'static crate::bible_structure::BibleStructure,
        progress: &ReadingProgress,
    ) -> Self {
        let tree_items = build_dashboard_tree_items(bible, progress);
        let mut tree_state = TreeState::default();
        tree_state.select_first();

        Self {
            tree_items,
            tree_state,
            show_only_unread: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Tree
                Constraint::Length(3), // Footer
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

        // Render tree
        let tree = Tree::new(&self.tree_items[..])
            .expect("error rendering tree")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if self.show_only_unread {
                        "Bible Structure (Space/→: expand, ←: collapse, ↑↓: navigate, r: record, q: quit)"
                    } else {
                        "Bible Structure (Space/→: expand, ←: collapse, ↑↓: navigate, r: record, q: quit)"
                    }),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(tree, chunks[1], &mut self.tree_state);

        // Footer
        let footer_text = "Space/→: Expand | ←: Collapse | ↑↓: Navigate | r: Record | q: Quit";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[2]);
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> DashboardAction {
        match (key.modifiers, key.code) {
            (_, crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc) => {
                DashboardAction::Quit
            }
            (_, crossterm::event::KeyCode::Char('r')) => DashboardAction::StartRecord,
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
        self.tree_state = TreeState::default();
        self.tree_state.select_first();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardAction {
    None,
    Quit,
    StartRecord,
}
