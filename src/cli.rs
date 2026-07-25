use clap::{Parser, Subcommand};

const AFTER_HELP: &str = "\
Note format:
  Notes are HTML comments whose body starts with NOTE or FIXME.

  Plain note (optional #tags):
    <!-- NOTE Spot is a dog. #character #bio -->

  Definition (def | define | definition + term + gloss):
    <!-- NOTE def spearsheaves a tax taken from the crop. -->

  Errata:
    <!-- FIXME timeline broken in ch. 4 #plot -->

  Trailing #tags at the end of a note are hidden in display but still
  searchable. In-text tags (e.g. the #character arc) stay in the text.

  On first run, you will be asked for a file glob to scan (per directory).
  Quote globs in the shell:  notes config \"**/*.md\"
";

#[derive(Debug, Parser)]
#[command(
    name = "notes",
    version,
    about = "Search and browse inline notes in text files",
    long_about = "Search and browse inline notes embedded as HTML comments in text files.\n\n\
Notes use <!-- NOTE ... --> and <!-- FIXME ... --> so editors can highlight them. \
Tag with #tags, define terms with NOTE def|define|definition, and collect errata via FIXME.\n\n\
Configuration is per working directory (a scan glob). There is no on-disk note cache; \
files are rescanned on each run. Full-text search loads only for the TUI or search -f.",
    after_help = AFTER_HELP
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Set or show the scan glob for this directory
    Config(Config),
    /// Look up a definition by term
    Define(Define),
    /// Search notes by tag (default) or full text (-f)
    Search(Search),
    /// List all FIXME (errata) notes
    Errata,
    /// Pretty-print the full glossary of definitions
    Glossary,
    /// Pretty-print every note (notes, definitions, and FIXMEs)
    All,
    /// Open the interactive tag browser (default when no command is given)
    Tui,
}

#[derive(Debug, Parser)]
#[command(
    after_help = "Example:\n  notes config \"**/*.md\"\n  notes config \"src/chapter.*.md\""
)]
pub struct Config {
    /// Glob of files to scan. Omit to show current config.
    /// Quote globs in the shell. Surrounding quotes in the value are stripped.
    pub glob: Option<String>,
}

#[derive(Debug, Parser)]
pub struct Define {
    /// Term to look up (from NOTE def|define|definition …)
    pub term: String,
}

#[derive(Debug, Parser)]
#[command(
    after_help = "Examples:\n  notes search character\n  notes search -f \"tax harvested\""
)]
pub struct Search {
    /// Tag name, or full-text query when -f is set
    pub query: String,

    /// Full-text search over note bodies (builds the in-memory indexer)
    #[arg(short = 'f', long = "full-text")]
    pub full_text: bool,
}
