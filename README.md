# Notes

Search and browse inline notes embedded in text files. Built for long-form
writing (books, wikis, drafts) where you want definitions, tags, and errata
next to the prose—without a separate notes database.

## Note format

Notes live in HTML comments. Many editors highlight `NOTE` / `FIXME` inside
comments, so they stay visible while you write.

### Plain note

```markdown
See Spot run. <!-- NOTE Spot is a dog. #character #bio -->
```

- Must start with `NOTE` (after `<!--`).
- Optional `#tags` anywhere in the body.
- **Trailing** `#tags` (a run of tag tokens at the end) are hidden in display
  output but still searchable.
- **In-text** tags (e.g. `the #character arc`) stay in the text: the `#` is
  dropped and the word is underlined.

### Definition

```markdown
<!-- NOTE def spearsheaves a tax taken directly from the crop as it is harvested. -->
```

- After `NOTE`, use `def`, `define`, or `definition`.
- Next token is the **term**; the rest is the **gloss**.
- Space-separated only (no `define:term`).
- Definitions appear in `notes define` and `notes glossary`.

### Errata (FIXME)

```markdown
<!-- FIXME timeline inconsistency in ch. 4 #plot -->
```

- Starts with `FIXME` instead of `NOTE`.
- Listed by `notes errata` and the TUI `e` key.
- Tags on FIXMEs participate in normal tag search.

### What is ignored

Ordinary HTML comments are ignored:

```markdown
<!-- this is not a note -->
```

## Setup

Configuration is **per working directory** (canonical path), stored with
[abseil](https://crates.io/crates/abseil) under your user config dir.

On first run in a directory, you are prompted for a file glob. Or set it
explicitly (quote globs in the shell):

```bash
notes config "**/*.md"
notes config "src/chapter.*.md"
```

Show the current directory’s config:

```bash
notes config
```

Surrounding quotes typed into the interactive prompt are stripped automatically.

There is **no on-disk note cache**. Each command rescans matching files and
rebuilds an in-memory index.

## Commands

| Command | Description |
|---------|-------------|
| `notes` / `notes tui` | Interactive tag browser |
| `notes search <tag>` | Notes with the given tag |
| `notes search -f <query>` | Full-text search over note bodies |
| `notes define <term>` | Look up a definition |
| `notes glossary` | Pretty-print all definitions (sorted) |
| `notes all` | Pretty-print every note |
| `notes errata` | List all FIXME notes |
| `notes config [glob]` | Show or set the scan glob |

```bash
notes search character
notes search -f "tax harvested"
notes define spearsheaves
notes glossary
notes all
notes errata
```

### Full-text search

`memory-indexer` is built only when needed:

- Opening the TUI, or
- `notes search -f …`

It indexes note bodies (and definition terms), not full source documents.

## TUI

```bash
notes        # or: notes tui
```

| Key | Action |
|-----|--------|
| `tab` | Focus tags ↔ notes |
| `j` / `k` | Move in the focused list |
| `enter` | Expand selected note (dialog) |
| `y` | Yank note text to clipboard |
| `/` | Filter tags |
| `f` | Full-text search |
| `e` | Show errata (FIXMEs) |
| `q` / `esc` | Quit (or close dialog / clear override) |

In the detail dialog: `j`/`k` scroll, `y` yank, `enter`/`esc`/`q` close.

## Display rules

- Tag lists and titles never show a leading `#`.
- Trailing hashtag runs are omitted from rendered bodies.
- In-text tags render without `#`, underlined when the terminal supports it.

## Development

```bash
cargo build
cargo test
cargo run -- search character
```

See [AGENT.md](AGENT.md) for architecture notes aimed at automated agents.
