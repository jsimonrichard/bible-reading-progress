# Bible Reading Progress Tracker

A command-line tool to track your Bible reading progress with a YAML-based backend for easy reading and version control.

## Features

- **Record Mode**: Interactive interface to record what you read today, supporting sub-chapter ranges
- **Dashboard Mode**: View statistics showing how many times you've read each passage and when you last read it
- **YAML Storage**: All data is stored in readable(ish), diffable YAML format
- **Sub-chapter Ranges**: Support for recording specific verse ranges (e.g., "1-10")

## Usage

Install the client:

```bash
cargo install --path .
```

Run:

```bash
brp
```

The application starts in **Dashboard mode** by default, showing all your reading progress.

### Dashboard Mode

- **↑/↓**: Navigate through passages
- **Space/→/Enter**: Expand/collapse a passage to see details
- **←**: Collapse a passage
- **r**: Switch to Record mode
- **m**: Switch to Manual Add mode
- **q/Esc**: Quit

The dashboard displays:
- Each passage you've read
- How many times you've read it
- How long ago you last read it (e.g., "today", "3 days ago", "2 months ago")

### Record Mode

Press **r** from the dashboard to record what you read today. This mode automatically saves and returns to the dashboard after adding a reading.

The record widget has multiple input fields that you navigate between:

- **Tab**: Move to the next field
- **Shift+Tab**: Move to the previous field
- **↑/↓**: When in the Book field, navigate through book matches
- **Type**: Enter text in the current field
  - **Book field**: Type to search for a book (fuzzy matching)
  - **Chapter field**: Enter chapter number (e.g., `1`, `1-5` for range, or leave empty for entire book)
  - **Verse field**: Enter verse ranges (e.g., `1-10`, or leave empty for full chapter)
- **Enter**: 
  - In Book field: Select the book and move to Chapter field
  - In Chapter field: Move to Verse field
  - In Verse field: Add the reading (saves automatically and returns to dashboard)
- **Esc**: Cancel and return to dashboard

### Manual Add Mode

Press **m** from the dashboard to manually add readings with custom read counts and dates. This mode works similarly to Record mode but includes additional fields:

- **Read Count field**: Enter how many times you've read the passage (defaults to 1)
- **Date field**: Enter the date in YYYY-MM-DD format (defaults to today)

This mode overwrites any existing readings for overlapping verse ranges.

## Data Storage

Your reading progress is stored in `.local/share/bible-reading-progress.yaml`, or the equivalent. The format is human-readable-ish and version-control friendly:

```yaml
books:
  Psalms:
    map:
      ? chapter: 1 # start
        verse: 1
      : - chapter: 1 # end
          verse: 7
        - read_count: 1
          last_read: 2025-12-24
```

## Building

```bash
cargo build --release
```

The binary will be in `target/release/brp`.

## License

Copyright (c) J. Simon Richard <jsimonrichard@gmail.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
