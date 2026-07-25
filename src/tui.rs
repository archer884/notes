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

use crate::format::{body_for_display, plain_body, BodyWord};
use crate::note::{Kind, Note};
use crate::search::FtsIndex;
use crate::store::NoteStore;

enum Mode {
    Browse,
    Filter,
    Fts,
    Detail { scroll: u16 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Tags,
    Notes,
}

struct App {
    store: NoteStore,
    fts: FtsIndex,
    tags: Vec<String>,
    tag_state: ListState,
    note_state: ListState,
    filter: String,
    fts_query: String,
    mode: Mode,
    focus: Focus,
    /// When set, right pane shows these note ids (FTS or errata)
    override_ids: Option<Vec<usize>>,
    status: String,
}

impl App {
    fn new(store: NoteStore) -> Self {
        let fts = FtsIndex::build(&store);
        let tags: Vec<String> = store.tags().into_iter().map(str::to_owned).collect();
        let mut tag_state = ListState::default();
        if !tags.is_empty() {
            tag_state.select(Some(0));
        }
        let mut app = Self {
            store,
            fts,
            tags,
            tag_state,
            note_state: ListState::default(),
            filter: String::new(),
            fts_query: String::new(),
            mode: Mode::Browse,
            focus: Focus::Tags,
            override_ids: None,
            status: String::new(),
        };
        app.reset_note_selection();
        app
    }

    fn filtered_tags(&self) -> Vec<&str> {
        let q = self.filter.to_ascii_lowercase();
        self.tags
            .iter()
            .map(|s| s.as_str())
            .filter(|t| q.is_empty() || t.contains(&q))
            .collect()
    }

    fn selected_tag(&self) -> Option<String> {
        let tags = self.filtered_tags();
        self.tag_state
            .selected()
            .and_then(|i| tags.get(i).map(|s| (*s).to_owned()))
    }

    fn current_notes(&self) -> Vec<&Note> {
        if let Some(ids) = &self.override_ids {
            return ids.iter().filter_map(|&id| self.store.get(id)).collect();
        }
        match self.selected_tag() {
            Some(tag) => self.store.search_tag(&tag),
            None => Vec::new(),
        }
    }

    fn reset_note_selection(&mut self) {
        if self.current_notes().is_empty() {
            self.note_state.select(None);
        } else {
            self.note_state.select(Some(0));
        }
    }

    fn select_next_tag(&mut self) {
        let len = self.filtered_tags().len();
        if len == 0 {
            self.tag_state.select(None);
            return;
        }
        let i = self.tag_state.selected().map(|i| (i + 1) % len).unwrap_or(0);
        self.tag_state.select(Some(i));
        self.override_ids = None;
        self.reset_note_selection();
    }

    fn select_prev_tag(&mut self) {
        let len = self.filtered_tags().len();
        if len == 0 {
            self.tag_state.select(None);
            return;
        }
        let i = self
            .tag_state
            .selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.tag_state.select(Some(i));
        self.override_ids = None;
        self.reset_note_selection();
    }

    fn select_next_note(&mut self) {
        let len = self.current_notes().len();
        if len == 0 {
            return;
        }
        let i = self.note_state.selected().map(|i| (i + 1) % len).unwrap_or(0);
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
            Focus::Tags => self.select_next_tag(),
            Focus::Notes => self.select_next_note(),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::Tags => self.select_prev_tag(),
            Focus::Notes => self.select_prev_note(),
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Tags => Focus::Notes,
            Focus::Notes => Focus::Tags,
        };
    }

    fn run_fts(&mut self) {
        let q = self.fts_query.trim();
        if q.is_empty() {
            self.override_ids = None;
        } else {
            self.override_ids = Some(self.fts.search_ids(q));
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
        self.override_ids = Some(ids);
        self.focus = Focus::Notes;
        self.reset_note_selection();
    }

    fn selected_note(&self) -> Option<&Note> {
        let notes = self.current_notes();
        self.note_state.selected().and_then(|i| notes.get(i).copied())
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
        if self.filtered_tags().is_empty() {
            self.tag_state.select(None);
        } else {
            self.tag_state.select(Some(0));
        }
        self.override_ids = None;
        self.reset_note_selection();
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
                KeyCode::Esc => {
                    if app.override_ids.is_some() {
                        app.override_ids = None;
                        app.fts_query.clear();
                        app.reset_note_selection();
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
                KeyCode::Tab => app.toggle_focus(),
                KeyCode::BackTab => app.toggle_focus(),
                KeyCode::Enter => app.open_detail(),
                KeyCode::Char('y') => app.yank_selected(),
                KeyCode::Char('e') => app.show_errata(),
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
                    app.override_ids = None;
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
                format!(
                    " j/k move  tab focus ({})  enter expand  y yank  / filter  f fts  e errata  q quit ",
                    match app.focus {
                        Focus::Tags => "tags",
                        Focus::Notes => "notes",
                    }
                )
            } else {
                format!(" {} ", app.status)
            }
        }
        Mode::Filter => format!(" filter tags: {}_  (Enter/Esc) ", app.filter),
        Mode::Fts => format!(" full-text: {}_  (Enter search, Esc cancel) ", app.fts_query),
        Mode::Detail { .. } => {
            if app.status.is_empty() {
                " j/k scroll  y yank  enter/esc close ".to_string()
            } else {
                format!(" {}  |  j/k scroll  y yank  enter/esc close ", app.status)
            }
        }
    };
    f.render_widget(Paragraph::new(status), chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(chunks[1]);

    render_tags(f, app, body[0]);
    render_notes(f, app, body[1]);

    let preview = app
        .selected_note()
        .map(preview_lines)
        .unwrap_or_default();
    f.render_widget(
        Paragraph::new(preview)
            .block(Block::default().borders(Borders::ALL).title(" preview ")),
        chunks[2],
    );

    if let Mode::Detail { scroll } = app.mode {
        if let Some(note) = app.selected_note().cloned() {
            render_detail(f, &note, scroll);
        }
    }
}

fn render_tags(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_tags()
        .into_iter()
        .map(|t| ListItem::new(Line::from(t.to_string())))
        .collect();

    let focused = app.focus == Focus::Tags && matches!(app.mode, Mode::Browse);
    let title = if app.override_ids.is_some() {
        " tags (override) "
    } else if focused {
        " tags * "
    } else {
        " tags "
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

    f.render_stateful_widget(list, area, &mut app.tag_state);
}

fn render_notes(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .current_notes()
        .into_iter()
        .map(|n| {
            let kind = match &n.kind {
                Kind::Fixme => "FIXME ".to_string(),
                Kind::Define { term } => format!("def:{term} "),
                Kind::Note => String::new(),
            };
            let summary = truncate(&plain_body(&n.text), 72);
            ListItem::new(Line::from(vec![Span::raw(kind), Span::raw(summary)]))
        })
        .collect();

    let focused = app.focus == Focus::Notes && matches!(app.mode, Mode::Browse);
    let title = match &app.override_ids {
        Some(_) if app.fts_query.is_empty() => {
            if focused {
                " notes (errata) * ".to_string()
            } else {
                " notes (errata) ".to_string()
            }
        }
        Some(_) => {
            if focused {
                format!(" notes (fts: {}) * ", app.fts_query)
            } else {
                format!(" notes (fts: {}) ", app.fts_query)
            }
        }
        None => match app.selected_tag() {
            Some(tag) if focused => format!(" notes ({tag}) * "),
            Some(tag) => format!(" notes ({tag}) "),
            None if focused => " notes * ".to_string(),
            None => " notes ".to_string(),
        },
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
        Kind::Note => " note ".to_string(),
    };

    let paragraph = Paragraph::new(Text::from(detail_lines(note)))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(Block::default().borders(Borders::ALL).title(title));

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
    for (i, word) in body_for_display(text).into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        match word {
            BodyWord::Text(s) => spans.push(Span::raw(s)),
            BodyWord::Tag(s) => spans.push(Span::styled(
                s,
                Style::default().add_modifier(Modifier::UNDERLINED),
            )),
        }
    }
    Line::from(spans)
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
