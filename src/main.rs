use color_eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::prelude::*;

use bible_reading_progress::bible_structure::get_bible_structure;
use bible_reading_progress::config::Config;
use bible_reading_progress::progress::ReadingProgress;
use bible_reading_progress::utils::{load_progress, save_progress};
use bible_reading_progress::widgets::dashboard::{DashboardAction, DashboardWidget};
use bible_reading_progress::widgets::record::{RecordAction, RecordWidget};

enum AppMode {
    Dashboard(DashboardWidget),
    Record(RecordWidget),
}

struct App {
    running: bool,
    mode: AppMode,
    bible: &'static bible_reading_progress::bible_structure::BibleStructure,
    progress: ReadingProgress,
    config: Config,
}

impl App {
    fn new() -> Result<Self> {
        let bible = get_bible_structure();
        let config = Config::default();
        let progress = load_progress(&config)?;
        let dashboard = DashboardWidget::new(bible, &progress);

        Ok(Self {
            running: true,
            mode: AppMode::Dashboard(dashboard),
            bible,
            progress,
            config,
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
        match &mut self.mode {
            AppMode::Dashboard(dashboard) => dashboard.render(frame),
            AppMode::Record(record) => record.render(frame),
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match &mut self.mode {
                AppMode::Dashboard(dashboard) => {
                    let action = dashboard.handle_key(key);
                    self.handle_dashboard_action(action);
                }
                AppMode::Record(record) => {
                    let action = record.handle_key(key, self.bible)?;
                    match action {
                        RecordAction::None => {}
                        RecordAction::Cancel => {
                            self.dashboard_mode();
                        }
                        RecordAction::AddReading => {
                            // Add reading (clears fields), then save and exit
                            if let Err(e) = record.add_reading(&mut self.progress, self.bible) {
                                record.error_message = Some(e);
                            } else {
                                save_progress(&self.progress, &self.config)?;
                                self.dashboard_mode();
                            }
                        }
                    }
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_dashboard_action(&mut self, action: DashboardAction) {
        match action {
            DashboardAction::None => {}
            DashboardAction::Quit => self.quit(),
            DashboardAction::StartRecord => self.start_record_mode(),
        }
    }

    fn start_record_mode(&mut self) {
        let record = RecordWidget::new(self.bible);
        self.mode = AppMode::Record(record);
    }

    fn dashboard_mode(&mut self) {
        let dashboard = DashboardWidget::new(self.bible, &self.progress);
        self.mode = AppMode::Dashboard(dashboard);
    }

    fn quit(&mut self) {
        // Save before quitting
        if let Err(e) = save_progress(&self.progress, &self.config) {
            eprintln!("Error saving progress: {}", e);
        }
        self.running = false;
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let mut app = App::new()?;
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
