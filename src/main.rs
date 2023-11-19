mod error;
mod note;

use std::{collections::HashMap, fmt::Debug, fs, path::PathBuf, process};

use clap::Parser;
use note::{Comment, Definition, Inline, InlineParser, TagExtractor};

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

// This program really needs a better interface, but for right now I'm just going to provide it
// with a root directory to read files from. However, for the future, I should note that the
// following style of command works on Windows even though globbing files does not:
//
// notes (ls ~/foo/bar/baz*.txt)

#[derive(Debug, Parser)]
struct Args {
    root: String,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Parser)]
enum Command {
    Define(Define),
    Search(Search),
}

#[derive(Debug, Parser)]
struct Define {
    term: String,
}

#[derive(Debug, Parser)]
struct Search {
    tag: String,
}

struct Index<'a> {
    definitions: HashMap<&'a str, &'a str>,
    comments: HashMap<&'a str, Vec<&'a str>>,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let mut time = stopwatch::Stopwatch::start_new();

    // Rust has strict rules on ownership and borrowing of data. Ordinarily, adding a key/value
    // pair to a dictionary implies that both key and value are "owned" by the dictionary. We're
    // not interested in that behavior at the moment, mostly because a given "comment" may be
    // associated with an arbitrary number of "keys" (which in this case are actually "tags" from
    // the document), and we have no interest in copying the same comment to each such entry in
    // the dictionary. Rather, it seems a much better idea to associate pointers with dictionary
    // keys in at least some cases.

    // Note: when I write "dictionary," I mean "hashmap." I've been writing a lot of C# lately.

    let (definitions, comments) = load_from_path(&args)?;

    // Creating the definitions index is actually easy, since terms and definitions have a
    // one-to-one relationship.

    let definitions_index: HashMap<&str, &str> = definitions
        .iter()
        .map(|x| (x.term.as_ref(), x.definition.as_ref()))
        .collect();

    // Creating the tagged comments index is a little more convoluted, because tags and comments do
    // NOT have a one-to-one relationship.

    let comments_index = build_comments_index(&comments);

    // We are now ready to serve up data gathered directly from the "source code" of my novel!
    // ...so I can stop the timer and we can report how long that took.

    time.stop();
    let elapsed = time.elapsed();
    println!("data collected in {:.02} seconds", elapsed.as_secs_f64());

    // For those of you who are not in on the joke, "seconds" is not the appropriate descriptor for
    // how long this process is going to take. On modern hardware, even *hundredths* of seconds may
    // not be sufficient.

    let index = Index {
        comments: comments_index,
        definitions: definitions_index,
    };

    if let Some(command) = &args.command {
        dispatch(command, &index);
    } else {
        println!(
            "{} definitions\n{} comments",
            definitions.len(),
            comments.len()
        );
    }

    Ok(())
}

fn dispatch(command: &Command, index: &Index) {
    match command {
        Command::Define(Define { term }) => {
            if let Some(&entry) = index.definitions.get(&**term) {
                println!("{term}: {entry}");
            } else {
                println!("{term} not found");
                for &key in index.definitions.keys() {
                    println!("{key}");
                }
            }
        }
        Command::Search(Search { tag }) => {
            // This bare bones implementation fails to do anything with regard to formatting and
            // headings and so forth.
            if let Some(entries) = index.comments.get(&**tag) {
                for &entry in entries {
                    println!("{entry}\n");
                }
            } else {
                println!("no entries found for {tag}");
            }
        }
    }
}

fn build_comments_index(comments: &[Comment]) -> HashMap<&str, Vec<&str>> {
    let mut map = HashMap::new();
    let tagged_comments = comments
        .iter()
        .flat_map(|x| x.tags.iter().map(|tag| (tag, &x.comment)));

    for (key, value) in tagged_comments {
        map.entry(key.as_ref())
            .or_insert_with(Vec::new)
            .push(value.as_ref())
    }
    map
}

fn load_from_path(args: &Args) -> Result<(Vec<Definition>, Vec<Comment>)> {
    let extractor = TagExtractor::new();
    let parser = InlineParser::new();
    let mut definitions = Vec::new();
    let mut comments = Vec::new();

    for path in get_paths(&args.root)? {
        let text = fs::read_to_string(path)?;
        let tags = extractor.tags(&text);
        let inlines: Result<Vec<_>, _> = tags.map(|tag| parser.parse(tag)).collect();

        for inline in inlines? {
            match inline {
                Inline::Comment(comment) => comments.push(*comment),
                Inline::Definition(definition) => definitions.push(*definition),
            }
        }
    }
    Ok((definitions, comments))
}

fn get_paths(path: &str) -> Result<Vec<PathBuf>> {
    let dir = fs::read_dir(path)?;
    println!("ok...");
    Ok(dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let entry_type = entry.file_type().ok()?;
            Some(entry)
                .filter(|_| entry_type.is_file())
                .map(|entry| entry.path())
        })
        .collect())
}
