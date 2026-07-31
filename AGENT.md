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
   - `(?i)^(?:def|define|definition)\s+(?:"([^"]+)"|'([^']+)'|(\S+))\s+(.+)$` → `Kind::Define { term }` (term lowercased), gloss = text. Term may be a bare word or a `"..."` / `'...'`-quoted phrase; quotes are stripped and not stored.
   - else → `Kind::Note`.
3. Anything else → ignored.

Tags: `#\S+`, normalized lowercase; trailing non-alphanumeric stripped from tag name.

Source path + 1-based line number recorded on each `Note`.

## Display body rules (`format::body_for_display` / `styled_words`)

1. Parse inline markdown emphasis: `**`/`__` bold, `*`/`_` italic (nestable); unclosed markers stay literal.
2. Every `#tag` token → tag style (no `#`, underline). Trailing tags are **not** stripped — a sentence-final tag like `…his mother, #Athrune.` is kept as prose. Only the tag name is underlined; trailing punctuation (e.g. the period) stays plain and attached with no gap. A `BodyWord` carries styled `segments` so a word can mix a tag name with plain punctuation.
3. CLI and TUI apply bold / italic / underline from `BodyStyle`.

Use `plain_body` for list rows and yank (markers and `#` removed).

## Body wrapping (`format::wrap_words`)

Greedy word wrap with end-of-line hyphenation via the `hyphenation` crate
(`embed_en-us` feature, `Standard` dictionary built once in `Formatter::new`).
A word that overflows its line is split at the largest dictionary-permitted
opportunity, emitting a trailing `-`; uncappable words overflow whole. TUI
preview/detail do not wrap (they scroll).

## TUI glossary terms

`by_term` is filled at store load. The sorted terms list for the left pane is built lazily on first `g`.

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

- Catalog: Tags (default) or Glossary (`g`); Esc from glossary returns to tags.
- Left pane: tags or terms; `/` filters the active left list.
- Focus: Left | Notes (`tab`); `j`/`k` move focused pane.
- `enter` → detail overlay (scroll, yank). Preview pane always shows selection.
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
