mod cli;
mod configuration;
mod error;
mod format;
mod logging;
mod note;
mod search;
mod store;
mod tui;

use std::{io, process};

use cli::{Args, Command, Config, Define, Search};
use format::Formatter;
use search::FtsIndex;
use store::NoteStore;

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

fn main() {
    logging::initialize();
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    match args.command {
        None | Some(Command::Tui) => cmd_tui(),
        Some(Command::Config(cmd)) => cmd_config(cmd),
        Some(Command::Define(cmd)) => cmd_define(cmd),
        Some(Command::Search(cmd)) => cmd_search(cmd),
        Some(Command::Errata) => cmd_errata(),
        Some(Command::Pending) => cmd_pending(),
        Some(Command::Glossary) => cmd_glossary(),
        Some(Command::All) => cmd_all(),
    }
}

fn load_store() -> Result<NoteStore> {
    let config = configuration::load_or_prompt()?;
    NoteStore::load(&config.glob)
}

fn cmd_config(cmd: Config) -> Result<()> {
    match cmd.glob {
        Some(glob) => {
            let config = configuration::set_glob(glob)?;
            println!("scan glob set to {:?}", config.glob);
        }
        None => match configuration::show_config()? {
            Some((dir, config)) => {
                println!("{}: {}", dir.display(), config.glob);
            }
            None => {
                println!("no config for this directory; run `notes config <glob>`");
            }
        },
    }
    Ok(())
}

fn cmd_define(cmd: Define) -> Result<()> {
    let store = load_store()?;
    let notes = store.define(&cmd.term);
    if notes.is_empty() {
        eprintln!("no definition for {:?}", cmd.term);
        return Ok(());
    }
    Formatter::new().fmt_notes(io::stdout().lock(), &notes)?;
    Ok(())
}

fn cmd_search(cmd: Search) -> Result<()> {
    let store = load_store()?;
    let notes = if cmd.full_text {
        let fts = FtsIndex::build(&store);
        fts.search(&store, &cmd.query)
    } else {
        store.search_tag(&cmd.query)
    };

    if notes.is_empty() {
        eprintln!("no notes matched {:?}", cmd.query);
        return Ok(());
    }
    Formatter::new().fmt_notes(io::stdout().lock(), &notes)?;
    Ok(())
}

fn cmd_errata() -> Result<()> {
    let store = load_store()?;
    let notes = store.errata();
    if notes.is_empty() {
        eprintln!("no FIXME notes");
        return Ok(());
    }
    Formatter::new().fmt_notes(io::stdout().lock(), &notes)?;
    Ok(())
}

fn cmd_pending() -> Result<()> {
    let store = load_store()?;
    let notes = store.todos();
    if notes.is_empty() {
        eprintln!("no TODO notes");
        return Ok(());
    }
    Formatter::new().fmt_notes(io::stdout().lock(), &notes)?;
    Ok(())
}

fn cmd_glossary() -> Result<()> {
    let store = load_store()?;
    let notes = store.glossary();
    if notes.is_empty() {
        eprintln!("no definitions");
        return Ok(());
    }
    Formatter::new().fmt_glossary(io::stdout().lock(), &notes)?;
    Ok(())
}

fn cmd_all() -> Result<()> {
    let store = load_store()?;
    let notes: Vec<_> = store.notes().iter().collect();
    if notes.is_empty() {
        eprintln!("no notes");
        return Ok(());
    }
    Formatter::new().fmt_notes(io::stdout().lock(), &notes)?;
    Ok(())
}

fn cmd_tui() -> Result<()> {
    let store = load_store()?;
    tui::run(store)
}
