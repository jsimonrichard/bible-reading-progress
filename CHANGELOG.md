## 0.1.0 (2026-03-30)

### Breaking Changes

#### Initial release of Bible Reading Progress Tracker (`brp`), a terminal UI for tracking Bible reading progress.

- **Dashboard**: Tree-based view of all Bible books, chapters, and verses with read counts and last-read dates
- **Record mode**: Interactive interface to record today's reading with fuzzy book search, chapter/verse range support
- **Manual add mode**: Add readings with custom read counts and dates, with support for overwriting existing records
- **Sub-chapter ranges**: Track specific verse ranges within chapters (e.g. verses 1–10)
- **Color-coded progress**: Visual indicators for read vs. unread passages
- **YAML storage**: Human-readable, version-control-friendly data format
- **Configurable data path**: Override via `~/.config/bible-reading-progress.yaml`
- **Cross-platform**: Works on Linux, macOS, and Windows
