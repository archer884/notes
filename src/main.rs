mod cache;
mod cli;
mod configuration;
mod error;
mod format;
mod index;
mod logging;
mod note;
mod store;

use std::{fs, io, path::Path, process};

use cache::{CacheKey, FileCache};
use cli::{Args, Command, Config, Define, Search};
use configuration::{ApplicationPaths, Configuration};
use format::Formatter;
use hashbrown::HashMap;

use crate::index::Indexer;

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

fn main() {
    logging::initialize();
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    match &args.command {
        Command::Config(command) => config(&args, command),
        Command::Define(command) => define(&args, command),
        Command::Search(command) => search(&args, command),
    }
}

fn config(_args: &Args, command: &Config) -> Result<()> {
    let root = Path::new(&command.root);
    let configuration = Configuration::from_command(command);
    let paths = ApplicationPaths::from_current()?;

    fs::create_dir_all(paths.tools())?;
    let s = serde_json::to_string_pretty(&configuration)?;
    fs::write(paths.config(), s)?;

    let file_cache = build_file_cache(root, paths.cache())?;
    let d = store::zip(file_cache)?;
    fs::write(paths.cache(), d)?;

    Ok(())
}

fn define(_args: &Args, command: &Define) -> Result<()> {
    let paths = ApplicationPaths::from_current()?;
    let config = Configuration::load(paths.config())?;
    let cache = build_file_cache(config.root.as_ref(), paths.cache())?;

    if let Some(definition) = cache.define(&command.term.to_ascii_lowercase()) {
        let formatter = Formatter::new();
        formatter.fmt_definition(io::stdout().lock(), &command.term, definition)?;
    }

    Ok(())
}

fn search(_args: &Args, command: &Search) -> Result<()> {
    let paths = ApplicationPaths::from_current()?;
    let config = Configuration::load(paths.config())?;
    let cache = build_file_cache(config.root.as_ref(), paths.cache())?;
    let formatter = Formatter::new();

    let mut comments = cache.search(&command.tag.to_ascii_lowercase());
    if let Some(comment) = comments.next() {
        formatter.fmt_comment(io::stdout().lock(), comment)?;
    }
    for comment in comments {
        println!();
        formatter.fmt_comment(io::stdout().lock(), comment)?;
    }

    Ok(())
}

fn build_file_cache(root: &Path, cache: &Path) -> Result<FileCache> {
    let indexer = Indexer::new();
    let time = chronograf::start();
    let cache = {
        let files = read_files(root)?;
        let mut current = read_cache(cache)?;
        let mut cache = HashMap::new();

        for file in files {
            if let Some(cached) = current.map.remove(&file) {
                tracing::debug!(path = file.path.display().to_string(), "cache hit");
                cache.insert(file, cached);
            } else {
                tracing::debug!(path = file.path.display().to_string(), "cache miss");
                let index = indexer.index_path(&file.path)?;
                cache.insert(file, index);
            }
        }
        cache
    };

    let elapsed = time.finish();
    tracing::debug!(elapsed = ?elapsed, "cache time");
    Ok(FileCache { map: cache })
}

fn read_cache(path: &Path) -> io::Result<FileCache> {
    if path.exists() {
        store::unzip(path)
    } else {
        Ok(Default::default())
    }
}

fn read_files(path: &Path) -> io::Result<Vec<CacheKey>> {
    Ok(fs::read_dir(path)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            CacheKey::from_path(entry.path()).ok()
        })
        .collect())
}
