use std::collections::BTreeSet;
use std::io::{self, stdout};

use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::format::{plain_body, styled_words, BodyStyle};
use crate::note::{Kind, Note};
use crate::search::FtsIndex;
use crate::store::NoteStore;

enum Mode {
    Browse,
    Filter,
    Fts,
    Detail { scroll: u16 },
    Help,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Left,
    Notes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Catalog {
    Tags,
    Glossary,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverrideKind {
    Fts,
    Errata,
    Pending,
}

struct Override {
    kind: OverrideKind,
    ids: Vec<usize>,
}

struct App {
    store: NoteStore,
    fts: FtsIndex,
    tags: Vec<String>,
    /// Tags toggled on with space; notes pane shows notes carrying all of them.
    selected: BTreeSet<String>,
    /// Built on first `g` from store.by_term (already in memory).
    terms: Option<Vec<String>>,
    left_state: ListState,
    note_state: ListState,
    filter: String,
    fts_query: String,
    mode: Mode,
    focus: Focus,
    catalog: Catalog,
    /// When set, right pane shows these note ids (FTS, errata, or pending)
    override_state: Option<Override>,
    status: String,
}

impl App {
    fn new(store: NoteStore) -> Self {
        let fts = FtsIndex::build(&store);
        let tags: Vec<String> = store.tags().into_iter().map(str::to_owned).collect();
        let mut left_state = ListState::default();
        if !tags.is_empty() {
            left_state.select(Some(0));
        }
        let mut app = Self {
            store,
            fts,
            tags,
            selected: BTreeSet::new(),
            terms: None,
            left_state,
            note_state: ListState::default(),
            filter: String::new(),
            fts_query: String::new(),
            mode: Mode::Browse,
            focus: Focus::Left,
            catalog: Catalog::Tags,
            override_state: None,
            status: String::new(),
        };
        app.reset_note_selection();
        app
    }

    fn left_keys(&self) -> &[String] {
        match self.catalog {
            Catalog::Tags => &self.tags,
            Catalog::Glossary => self.terms.as_deref().unwrap_or(&[]),
        }
    }

    fn ensure_terms(&mut self) {
        if self.terms.is_none() {
            self.terms = Some(self.store.terms().into_iter().map(str::to_owned).collect());
        }
    }

    fn filtered_left(&self) -> Vec<&str> {
        let q = normalize_tag_filter(&self.filter);
        self.left_keys()
            .iter()
            .map(|s| s.as_str())
            .filter(|t| q.is_empty() || normalize_tag_filter(t).contains(&q))
            .collect()
    }

    fn selected_left(&self) -> Option<String> {
        let keys = self.filtered_left();
        self.left_state
            .selected()
            .and_then(|i| keys.get(i).map(|s| (*s).to_owned()))
    }

    fn current_notes(&self) -> Vec<&Note> {
        if let Some(state) = &self.override_state {
            return state.ids.iter().filter_map(|&id| self.store.get(id)).collect();
        }
        match self.catalog {
            Catalog::Tags => self
                .store
                .search_tags(&self.selected.iter().cloned().collect::<Vec<_>>()),
            Catalog::Glossary => match self.selected_left() {
                Some(key) => self.store.define(&key),
                None => Vec::new(),
            },
        }
    }

    fn toggle_tag(&mut self) {
        if self.catalog != Catalog::Tags || self.focus != Focus::Left {
            return;
        }
        let Some(tag) = self.selected_left() else {
            return;
        };
        if !self.selected.remove(&tag) {
            self.selected.insert(tag);
        }
        self.override_state = None;
        self.reset_note_selection();
    }

    fn reset_note_selection(&mut self) {
        if self.current_notes().is_empty() {
            self.note_state.select(None);
        } else {
            self.note_state.select(Some(0));
        }
    }

    fn reset_left_selection(&mut self) {
        if self.filtered_left().is_empty() {
            self.left_state.select(None);
        } else {
            self.left_state.select(Some(0));
        }
        self.override_state = None;
        self.reset_note_selection();
    }

    fn select_next_left(&mut self) {
        let len = self.filtered_left().len();
        if len == 0 {
            self.left_state.select(None);
            return;
        }
        let i = self
            .left_state
            .selected()
            .map(|i| (i + 1) % len)
            .unwrap_or(0);
        self.left_state.select(Some(i));
        self.override_state = None;
        self.reset_note_selection();
    }

    fn select_prev_left(&mut self) {
        let len = self.filtered_left().len();
        if len == 0 {
            self.left_state.select(None);
            return;
        }
        let i = self
            .left_state
            .selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.left_state.select(Some(i));
        self.override_state = None;
        self.reset_note_selection();
    }

    fn select_next_note(&mut self) {
        let len = self.current_notes().len();
        if len == 0 {
            return;
        }
        let i = self
            .note_state
            .selected()
            .map(|i| (i + 1) % len)
            .unwrap_or(0);
        self.note_state.select(Some(i));
    }

    fn select_prev_note(&mut self) {
        let len = self.current_notes().len();
        if len == 0 {
            return;
        }
        let i = self
            .note_state
            .selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.note_state.select(Some(i));
    }

    fn move_down(&mut self) {
        match self.focus {
            Focus::Left => self.select_next_left(),
            Focus::Notes => self.select_next_note(),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::Left => self.select_prev_left(),
            Focus::Notes => self.select_prev_note(),
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Left => Focus::Notes,
            Focus::Notes => Focus::Left,
        };
    }

    fn run_fts(&mut self) {
        let q = self.fts_query.trim();
        if q.is_empty() {
            self.override_state = None;
        } else {
            self.override_state = Some(Override {
                kind: OverrideKind::Fts,
                ids: self.fts.search_ids(q),
            });
        }
        self.focus = Focus::Notes;
        self.reset_note_selection();
    }

    fn show_errata(&mut self) {
        let ids: Vec<usize> = self
            .store
            .notes()
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_fixme())
            .map(|(i, _)| i)
            .collect();
        self.fts_query.clear();
        self.override_state = Some(Override {
            kind: OverrideKind::Errata,
            ids,
        });
        self.focus = Focus::Notes;
        self.reset_note_selection();
    }

    fn show_pending(&mut self) {
        let ids: Vec<usize> = self
            .store
            .notes()
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_todo())
            .map(|(i, _)| i)
            .collect();
        self.fts_query.clear();
        self.override_state = Some(Override {
            kind: OverrideKind::Pending,
            ids,
        });
        self.focus = Focus::Notes;
        self.reset_note_selection();
    }

    fn show_glossary(&mut self) {
        self.ensure_terms();
        self.catalog = Catalog::Glossary;
        self.filter.clear();
        self.fts_query.clear();
        self.override_state = None;
        self.focus = Focus::Left;
        self.reset_left_selection();
        self.status.clear();
    }

    fn show_tags(&mut self) {
        self.catalog = Catalog::Tags;
        self.filter.clear();
        self.fts_query.clear();
        self.override_state = None;
        self.focus = Focus::Left;
        self.reset_left_selection();
        self.status.clear();
    }

    fn selected_note(&self) -> Option<&Note> {
        let notes = self.current_notes();
        self.note_state
            .selected()
            .and_then(|i| notes.get(i).copied())
    }

    fn open_detail(&mut self) {
        if self.selected_note().is_some() {
            self.mode = Mode::Detail { scroll: 0 };
            self.status.clear();
        }
    }

    fn yank_selected(&mut self) {
        let Some(note) = self.selected_note() else {
            self.status = "nothing to copy".into();
            return;
        };
        let text = yank_text(note);
        match Clipboard::new().and_then(|mut c| c.set_text(text)) {
            Ok(()) => self.status = "copied".into(),
            Err(e) => self.status = format!("copy failed: {e}"),
        }
    }

    fn on_filter_changed(&mut self) {
        self.reset_left_selection();
    }

    fn left_label(&self) -> &'static str {
        match self.catalog {
            Catalog::Tags => "tags",
            Catalog::Glossary => "terms",
        }
    }
}

pub fn run(store: NoteStore) -> crate::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(store);
    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> crate::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match &app.mode {
            Mode::Browse => match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('h') | KeyCode::Char('?') => app.mode = Mode::Help,
                KeyCode::Esc => {
                    if app.override_state.is_some() {
                        app.override_state = None;
                        app.fts_query.clear();
                        app.reset_note_selection();
                    } else if app.catalog == Catalog::Glossary {
                        app.show_tags();
                    } else {
                        return Ok(());
                    }
                }
                KeyCode::Char('/') => {
                    app.mode = Mode::Filter;
                    app.filter.clear();
                    app.status.clear();
                }
                KeyCode::Char('f') => {
                    app.mode = Mode::Fts;
                    app.fts_query.clear();
                    app.status.clear();
                }
                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                KeyCode::Char(' ') => app.toggle_tag(),
                KeyCode::Tab => app.toggle_focus(),
                KeyCode::BackTab => app.toggle_focus(),
                KeyCode::Enter => app.open_detail(),
                KeyCode::Char('y') => app.yank_selected(),
                KeyCode::Char('e') => app.show_errata(),
                KeyCode::Char('p') => app.show_pending(),
                KeyCode::Char('g') => match app.catalog {
                    Catalog::Glossary => app.show_tags(),
                    Catalog::Tags => app.show_glossary(),
                },
                _ => {}
            },
            Mode::Filter => match key.code {
                KeyCode::Esc => {
                    app.mode = Mode::Browse;
                    app.filter.clear();
                    app.on_filter_changed();
                }
                KeyCode::Enter => {
                    app.mode = Mode::Browse;
                    app.on_filter_changed();
                }
                KeyCode::Backspace => {
                    app.filter.pop();
                    app.on_filter_changed();
                }
                KeyCode::Char(c) => {
                    app.filter.push(c);
                    app.on_filter_changed();
                }
                _ => {}
            },
            Mode::Fts => match key.code {
                KeyCode::Esc => {
                    app.mode = Mode::Browse;
                    app.fts_query.clear();
                    app.override_state = None;
                    app.reset_note_selection();
                }
                KeyCode::Enter => {
                    app.run_fts();
                    app.mode = Mode::Browse;
                }
                KeyCode::Backspace => {
                    app.fts_query.pop();
                }
                KeyCode::Char(c) => {
                    app.fts_query.push(c);
                }
                _ => {}
            },
            Mode::Detail { scroll } => {
                let scroll = *scroll;
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        app.mode = Mode::Browse;
                        app.status.clear();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.mode = Mode::Detail {
                            scroll: scroll.saturating_add(1),
                        };
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.mode = Mode::Detail {
                            scroll: scroll.saturating_sub(1),
                        };
                    }
                    KeyCode::Char('y') => app.yank_selected(),
                    _ => {}
                }
            }
            Mode::Help => app.mode = Mode::Browse,
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(4),
        ])
        .split(f.area());

    let status = match &app.mode {
        Mode::Browse => {
            if app.status.is_empty() {
                let focus = match app.focus {
                    Focus::Left => app.left_label(),
                    Focus::Notes => "notes",
                };
                format!(" {focus}   h help   q quit ")
            } else {
                format!(" {} ", app.status)
            }
        }
        Mode::Filter => format!(
            " filter {}: {}_  (Enter/Esc) ",
            app.left_label(),
            app.filter
        ),
        Mode::Fts => format!(
            " full-text: {}_  (Enter search, Esc cancel) ",
            app.fts_query
        ),
        Mode::Detail { .. } => {
            if app.status.is_empty() {
                " j/k scroll  y yank  enter/esc close ".to_string()
            } else {
                format!(" {}  |  j/k scroll  y yank  enter/esc close ", app.status)
            }
        }
        Mode::Help => " help — any key to close ".to_string(),
    };
    f.render_widget(Paragraph::new(status), chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(chunks[1]);

    render_left(f, app, body[0]);
    render_notes(f, app, body[1]);

    let preview = app.selected_note().map(preview_lines).unwrap_or_default();
    f.render_widget(
        Paragraph::new(preview).block(Block::default().borders(Borders::ALL).title(" preview ")),
        chunks[2],
    );

    if let Mode::Detail { scroll } = app.mode {
        if let Some(note) = app.selected_note().cloned() {
            render_detail(f, &note, scroll);
        }
    }

    if matches!(app.mode, Mode::Help) {
        render_help(f);
    }
}

fn render_left(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_left()
        .into_iter()
        .map(|t| match app.catalog {
            Catalog::Tags => {
                let mark = if app.selected.contains(t) {
                    "[x] "
                } else {
                    "[ ] "
                };
                ListItem::new(Line::from(format!("{mark}{t}")))
            }
            Catalog::Glossary => ListItem::new(Line::from(t.to_string())),
        })
        .collect();

    let focused = app.focus == Focus::Left && matches!(app.mode, Mode::Browse);
    let label = app.left_label();
    let title = if app.override_state.is_some() {
        format!(" {label} (override) ")
    } else if focused {
        format!(" {label} * ")
    } else {
        format!(" {label} ")
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let block = if focused {
        block.border_style(Style::default().add_modifier(Modifier::BOLD))
    } else {
        block
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, area, &mut app.left_state);
}

fn render_notes(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .current_notes()
        .into_iter()
        .map(|n| {
            let kind = match &n.kind {
                Kind::Fixme => "FIXME ".to_string(),
                Kind::Todo => "TODO ".to_string(),
                Kind::Define { term } => {
                    if app.catalog == Catalog::Glossary && app.override_state.is_none() {
                        String::new()
                    } else {
                        format!("def:{term} ")
                    }
                }
                Kind::Note => String::new(),
            };
            let summary = truncate(&plain_body(&n.text), 72);
            ListItem::new(Line::from(vec![Span::raw(kind), Span::raw(summary)]))
        })
        .collect();

    let focused = app.focus == Focus::Notes && matches!(app.mode, Mode::Browse);
    let title = match &app.override_state {
        Some(state) => {
            let label = match state.kind {
                OverrideKind::Errata => "errata".to_string(),
                OverrideKind::Pending => "pending".to_string(),
                OverrideKind::Fts => format!("fts: {}", app.fts_query),
            };
            if focused {
                format!(" notes ({label}) * ")
            } else {
                format!(" notes ({label}) ")
            }
        }
        None => {
            let label = match app.catalog {
                Catalog::Tags => {
                    if app.selected.is_empty() {
                        "all".to_string()
                    } else {
                        app.selected
                            .iter()
                            .map(|t| format!("#{t}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                }
                Catalog::Glossary => app.selected_left().unwrap_or_default(),
            };
            match (label.is_empty(), focused) {
                (false, true) => format!(" notes ({label}) * "),
                (false, false) => format!(" notes ({label}) "),
                (true, true) => " notes * ".to_string(),
                (true, false) => " notes ".to_string(),
            }
        }
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let block = if focused {
        block.border_style(Style::default().add_modifier(Modifier::BOLD))
    } else {
        block
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(list, area, &mut app.note_state);
}

fn render_detail(f: &mut Frame, note: &Note, scroll: u16) {
    let area = centered_rect(72, 70, f.area());
    f.render_widget(Clear, area);

    let title = match &note.kind {
        Kind::Define { term } => format!(" {term} "),
        Kind::Fixme => " FIXME ".to_string(),
        Kind::Todo => " TODO ".to_string(),
        Kind::Note => " note ".to_string(),
    };

    let paragraph = Paragraph::new(Text::from(detail_lines(note)))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(74, 80, f.area());
    f.render_widget(Clear, area);

    let heading_style = Style::new().add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for (heading, rows) in [
        (
            "Navigation",
            &[
                ("j / k  ↓ / ↑", "move the focused pane"),
                ("tab", "switch focus: left ↔ notes"),
                ("enter", "open detail  /  confirm"),
                ("esc", "close overlay  /  cancel  /  back"),
            ][..],
        ),
        (
            "Catalog",
            &[
                ("g", "toggle tags ↔ glossary"),
                ("/", "filter the left list"),
                ("space", "toggle a tag pick (tags)"),
            ][..],
        ),
        (
            "Lists",
            &[
                ("e", "errata — show FIXMEs"),
                ("p", "pending — show TODOs"),
                ("f", "full-text search"),
            ][..],
        ),
        (
            "Other",
            &[
                ("y", "yank selected note to clipboard"),
                ("h / ?", "this help"),
                ("q", "quit"),
            ][..],
        ),
    ] {
        lines.push(Line::from(format!(" {heading}")).style(heading_style));
        for (key, desc) in rows {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::raw(format!("{key:<14}")),
                Span::raw(*desc),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" help (any key closes) "),
    );
    f.render_widget(paragraph, area);
}

fn detail_lines(note: &Note) -> Vec<Line<'static>> {
    let loc = format!("{}:{}", note.path.display(), note.line);
    vec![
        Line::from(loc),
        Line::from(""),
        styled_body_line(&note.text),
    ]
}

fn preview_lines(note: &Note) -> Vec<Line<'static>> {
    let loc = format!("{}:{}", note.path.display(), note.line);
    vec![Line::from(loc), styled_body_line(&note.text)]
}

fn styled_body_line(text: &str) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, word) in styled_words(text).into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        for (seg_text, seg_style) in word.segments {
            spans.push(Span::styled(seg_text, tui_style(seg_style)));
        }
    }
    Line::from(spans)
}

fn tui_style(style: BodyStyle) -> Style {
    let mut s = Style::default();
    if style.bold {
        s = s.add_modifier(Modifier::BOLD);
    }
    if style.italic {
        s = s.add_modifier(Modifier::ITALIC);
    }
    if style.tag {
        s = s.add_modifier(Modifier::UNDERLINED);
    }
    s
}

fn normalize_tag_filter(s: &str) -> String {
    s.trim()
        .trim_start_matches('#')
        .replace(' ', "_")
        .to_ascii_lowercase()
}

fn yank_text(note: &Note) -> String {
    match &note.kind {
        Kind::Define { term } => format!("{term}\n{}", plain_body(&note.text)),
        _ => plain_body(&note.text),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn help_renders_all_sections() {
        let backend = TestBackend::new(82, 26);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(render_help).unwrap();

        let buf = terminal.backend().buffer();
        let area = buf.area();
        let (w, h) = (area.width as usize, area.height as usize);
        let content = buf.content();
        let rendered: String = (0..h)
            .flat_map(|y| content[y * w..(y + 1) * w].iter())
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();

        for needle in [
            "Navigation", "Catalog", "Lists", "Other", "quit", "h / ?", "pending",
        ] {
            assert!(rendered.contains(needle), "help dialog missing {needle:?}");
        }
    }
}
