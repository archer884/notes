mod cache;
mod configuration;
mod error;
mod index;
mod note;

use std::{
    env,
    fmt::Debug,
    fs,
    io::{self, Read},
    path::Path,
    process,
};

use cache::{CacheKey, FileCache};
use clap::Parser;
use configuration::Configuration;
use flate2::{bufread::GzEncoder, read::GzDecoder, Compression};
use hashbrown::HashMap;
use index::Index;
use libsw::Sw;
use note::{Comment, Definition, Inline, InlineParser, TagExtractor};
use serde::{Deserialize, Serialize};

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

// This program really needs a better interface, but for right now I'm just going to provide it
// with a root directory to read files from. However, for the future, I should note that the
// following style of command works on Windows even though globbing files does not:
//
// notes (ls ~/foo/bar/baz*.txt)

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    Config(Config),
    Define(Define),
    Search(Search),
}

#[derive(Debug, Parser)]
struct Config {
    root: String,
}

#[derive(Debug, Parser)]
struct Define {
    term: String,
}

#[derive(Debug, Parser)]
struct Search {
    tag: String,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let dir = env::current_dir()?;
    match &args.command {
        Command::Config(command) => config(&args, command, &dir),
        Command::Define(command) => define(&args, command, &dir),
        Command::Search(command) => search(&args, command, &dir),
    }
}

fn config(_args: &Args, command: &Config, directory: &Path) -> Result<()> {
    let root = Path::new(&command.root);
    let configuration = Configuration::from_command(command);

    let tool_path = directory.join(".tool");
    let config_path = tool_path.join("notes.json");
    fs::create_dir_all(&tool_path)?;
    let s = serde_json::to_string_pretty(&configuration)?;
    fs::write(config_path, s)?;

    let cache_path = tool_path.join("notecache.gzip");
    let file_cache = build_file_cache(root, &cache_path)?;
    let d = zip(file_cache)?;
    fs::write(cache_path, d)?;

    Ok(())
}

fn define(_args: &Args, command: &Define, directory: &Path) -> Result<()> {
    // FIXME: UNFUCK!!!
    eprintln!("WARNING: unfuck this code repetition");
    let tool_path = directory.join(".tool");
    let config_path = tool_path.join("notes.json");
    let cache_path = tool_path.join("notecache.gzip");

    let config: Configuration = unzip(&config_path)?;
    let cache = build_file_cache(config.root.as_ref(), &cache_path)?;

    if let Some(definition) = cache.define(&command.term) {
        println!("{}: {definition}", command.term);
    }

    Ok(())
}

fn search(_args: &Args, command: &Search, directory: &Path) -> Result<()> {
    // FIXME: UNFUCK!!!
    eprintln!("WARNING: unfuck this code repetition");
    let tool_path = directory.join(".tool");
    let config_path = tool_path.join("notes.json");
    let cache_path = tool_path.join("notecache.gzip");

    let config: Configuration = unzip(&config_path)?;
    let cache = build_file_cache(config.root.as_ref(), &cache_path)?;

    for comment in cache.search(&command.tag) {
        // shitty proof of concept formatting
        println!("\n{}", comment.comment);
    }

    Ok(())
}

fn build_file_cache(root: &Path, cache: &Path) -> Result<FileCache> {
    let mut sw = Sw::new();
    let cache = {
        let _guard = sw.guard().unwrap();
        let mut current = read_cache(cache)?;
        let files = read_files(root)?;
        let mut cache = HashMap::new();

        for file in files {
            if let Some(cached) = current.map.remove(&file) {
                cache.insert(file, cached);
            } else {
                let index = index_from_path(&file.path)?;
                cache.insert(file, index);
            }
        }
        cache
    };

    println!("cache time: {} ms", sw.elapsed().as_millis());
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
    let mut encoder = GzEncoder::new(&*s, Compression::fast());
    encoder.read_to_end(&mut buf)?;
    Ok(buf)
}

fn unzip<T>(path: &Path) -> io::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let d = fs::read(path)?;
    let mut buf = Vec::new();
    let mut decoder = GzDecoder::new(&*d);
    decoder.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

fn index_from_path(path: &Path) -> Result<Index> {
    let (comments, definitions) = load_cd_from_path(path)?;
    let comments = comments
        .into_iter()
        .flat_map(|c| c.tags.clone().into_iter().map(move |tag| (tag, c.clone())))
        .fold(HashMap::new(), |mut a: HashMap<_, Vec<_>>, (k, v)| {
            a.entry(k).or_default().push(v);
            a
        });
    let definitions = definitions
        .into_iter()
        .map(|Definition { term, definition }| (term, definition))
        .collect();
    Ok(Index {
        comments,
        definitions,
    })
}

fn load_cd_from_path(path: &Path) -> Result<(Vec<Comment>, Vec<Definition>)> {
    // Be good to cache the extractor and parser instead of newing them up on each call--maybe turn
    // this method into an object?
    let extractor = TagExtractor::new();
    let parser = InlineParser::new();

    let mut definitions = Vec::new();
    let mut comments = Vec::new();

    let text = fs::read_to_string(path)?;
    let tags = extractor.tags(&text);
    let inlines: Result<Vec<_>, _> = tags.map(|tag| parser.parse(tag)).collect();

    for inline in inlines? {
        match inline {
            Inline::Comment(comment) => comments.push(*comment),
            Inline::Definition(definition) => definitions.push(*definition),
        }
    }

    Ok((comments, definitions))
}
