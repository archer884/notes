# Agent guide — notes

Rust CLI + TUI for inline notes in text files. Version **0.3**.

## Layout

| Path | Role |
|------|------|
| `src/main.rs` | Command dispatch |
| `src/cli.rs` | clap args / help text |
| `src/note.rs` + `note/parser.rs` | `Note` model + HTML-comment parser |
| `src/store.rs` | Glob scan → in-memory indexes |
| `src/configuration.rs` | Per-cwd config via abseil |
| `src/format.rs` | CLI pretty-print + display body rules |
| `src/search.rs` | Lazy `memory-indexer` FTS wrapper |
| `src/tui.rs` | ratatui browser |
| `src/error.rs` | Error type |
| `src/logging.rs` | `RUST_LOG` / `LOG` tracing |

No disk note cache. No `.tool/` directory.

## Note grammar (parser)

Comments matched by `(?s)<!--\s*(.*?)\s*-->`.

Body must begin with keyword:

1. **`FIXME`** → `Kind::Fixme`, rest is text.
2. **`NOTE`** then optional define form:
   - `(?i)^(?:def|define|definition)\s+(\S+)\s+(.+)$` → `Kind::Define { term }` (term lowercased), gloss = text.
   - else → `Kind::Note`.
3. Anything else → ignored.

Tags: `#\S+`, normalized lowercase; trailing non-alphanumeric stripped from tag name.

Source path + 1-based line number recorded on each `Note`.

## Display body rules (`format::body_for_display`)

1. Split on whitespace.
2. Peel **trailing** tag tokens (`#…`) from the end — hidden in UI, still in `note.tags` / index.
3. Remaining in-text `#tag` tokens → `BodyWord::Tag` (show without `#`, underline).
4. Other words → `BodyWord::Text`.

Use `plain_body` for list rows and yank. Do not delete in-text tag words.

## Config

- Crate: `abseil`, app name `notes`, config dir, file `config.json`.
- State: `HashMap<canonical_cwd, DirConfig { glob }>`.
- `normalize_glob` strips one layer of matching `'` or `"` quotes.
- First run without config: stdin prompt for glob.
- `notes config [glob]` set/show for current cwd only.

## Store indexes

- `notes: Vec<Note>`
- `by_tag`, `by_term`, `fixmes` → indices into `notes`
- Load: `glob(pattern)` → parse each file → `push`

`glossary()` returns all defines sorted by term.

## FTS

`FtsIndex::build(store)` only from TUI startup or `search -f`.  
Doc id = note index as string. Query via `memory_indexer::InMemoryIndex`.

## TUI behavior

- Focus: Tags | Notes (`tab`); `j`/`k` move focused pane.
- `enter` → detail overlay (scroll, yank).
- `e` / FTS results set `override_ids` on the notes pane.
- Clipboard yank: `arboard`.

## CLI commands

`config`, `define`, `search` (`-f`), `errata`, `glossary`, `all`, `tui`  
Default (no subcommand) → TUI.

`all` pretty-prints every note in scan order via `Formatter::fmt_notes`.

## Conventions

- No explanatory comments in code unless asked.
- Prefer extending existing modules over new crates.
- Run `cargo test` after parser/format/store changes.
- Do not commit secrets; do not add cache layers unless requested.

## Out of scope (unless asked)

- Persistent note DB / darkbird
- `--json` / `--markdown` glossary flags (planned later)
- Editor jump-to-source
- Watching files for live reload
