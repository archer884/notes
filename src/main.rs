mod cache;
mod cli;
mod configuration;
mod error;
mod format;
mod index;
mod logging;
mod note;

use std::{
    fs,
    io::{self, Read},
    path::Path,
    process,
};

use cache::{CacheKey, FileCache};
use cli::{Args, Command, Config, Define, Search};
use configuration::{ApplicationPaths, Configuration};
use flate2::{read::GzDecoder as Decoder, read::GzEncoder as Encoder, Compression};
use format::Formatter;
use hashbrown::HashMap;
use libsw::Sw;
use serde::{Deserialize, Serialize};

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
    let d = zip(file_cache)?;
    fs::write(paths.cache(), d)?;

    Ok(())
}

fn define(_args: &Args, command: &Define) -> Result<()> {
    let paths = ApplicationPaths::from_current()?;
    let config = Configuration::load(paths.config())?;
    let cache = build_file_cache(config.root.as_ref(), paths.cache())?;

    if let Some(definition) = cache.define(&command.term) {
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

    let mut comments = cache.search(&command.tag);

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
    let mut sw = Sw::new();
    let cache = {
        let _guard = sw.guard().unwrap();
        let files = read_files(root)?;
        let mut current = read_cache(cache)?;
        let mut cache = HashMap::new();

        for file in files {
            let path = file.path.display().to_string();
            if let Some(cached) = current.map.remove(&file) {
                tracing::debug!(path, "cache hit");
                cache.insert(file, cached);
            } else {
                tracing::debug!(path, "cache miss");
                let index = indexer.index_path(&file.path)?;
                cache.insert(file, index);
            }
        }
        cache
    };

    let elapsed = sw.elapsed().as_millis();
    tracing::debug!(elapsed, "cache time {elapsed} ms");
    Ok(FileCache { map: cache })
}

fn read_cache(path: &Path) -> io::Result<FileCache> {
    if path.exists() {
        unzip(path)
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

fn zip<T: Serialize>(s: T) -> io::Result<Vec<u8>> {
    let s = serde_json::to_vec(&s)?;
    let mut buf = Vec::with_capacity(s.len());
    let mut encoder = Encoder::new(&*s, Compression::fast());
    encoder.read_to_end(&mut buf)?;
    Ok(buf)
}

fn unzip<T>(path: &Path) -> io::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let d = fs::read(path)?;
    let mut buf = Vec::new();
    let mut decoder = Decoder::new(&*d);
    decoder.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}
