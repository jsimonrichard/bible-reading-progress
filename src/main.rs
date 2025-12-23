use color_eyre::Result;

use bible_reading_progress::config::Config;

struct App {
    config: Config,
    // progress: ReadingProgress,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let config = Config::default();
    Ok(())
}
