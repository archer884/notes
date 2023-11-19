mod error;
mod note;

use std::{fs, process};

use clap::Parser;
use note::{InlineParser, TagExtractor};

pub type Result<T, E = error::Error> = std::result::Result<T, E>;

#[derive(Debug, Parser)]
struct Args {
    paths: Vec<String>,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let extractor = TagExtractor::new();
    let parser = InlineParser::new();
    for path in &args.paths {
        list_notes(&extractor, &parser, path)?;
    }
    Ok(())
}

fn list_notes(extractor: &TagExtractor, parser: &InlineParser, path: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let tags = extractor.tags(&text);
    let inlines = tags.map(|tag| parser.parse(tag));

    for inline in inlines {
        println!("{:?}", inline?);
    }

    Ok(())
}
