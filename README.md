# Bible Reading Progress Tracker

A command-line tool to track your Bible reading progress with a YAML-based backend for easy reading and version control.

## Features

- **Record Mode**: Interactive interface to record what you read today, supporting sub-chapter ranges
- **Dashboard Mode**: View statistics showing how many times you've read each passage and when you last read it
- **YAML Storage**: All data is stored in readable, diffable YAML format
- **Sub-chapter Ranges**: Support for recording specific verse ranges (e.g., "1-10" or "1-5,10-15")

## Usage

Simply run the application:

```bash
cargo run
```

The application starts in **Dashboard mode** by default, showing all your reading progress.

### Dashboard Mode

- **↑/↓**: Navigate through passages
- **Enter**: Expand/collapse a passage to see details
- **r**: Switch to Record mode
- **q/Esc**: Quit

The dashboard displays:
- Each passage you've read
- How many times you've read it
- How long ago you last read it (e.g., "today", "3 days ago", "2 months ago")

### Record Mode

Press **r** from the dashboard to record what you read today.

- **←/→**: Navigate between books
- **Shift+←/→**: Navigate between chapters
- **Type**: Enter verse ranges (e.g., `1-10` or `1-5,10-15`)
- **Enter**: Add the current passage
- **a**: Clear input to add another passage
- **s**: Save all passages and return to dashboard
- **Esc**: Cancel and return to dashboard

Verse range examples:
- `1-10` for verses 1 through 10
- `1-5,10-15` for verses 1-5 and 10-15
- `1` for just verse 1
- (empty) for the entire chapter

## Data Storage

Your reading progress is stored in `reading_progress.yaml` in the project directory. The format is human-readable and version-control friendly:

```yaml
readings:
  - date: "2024-01-15"
    passages:
      - book: "Genesis"
        chapter: 1
        verses: "1-10"
      - book: "Genesis"
        chapter: 2
        verses: "1"  # full chapter
```

## Building

```bash
cargo build --release
```

The binary will be in `target/release/bible-reading-progress`.

## License

Copyright (c) J. Simon Richard <jsimonrichard@gmail.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
