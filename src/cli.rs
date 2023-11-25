use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

impl Args {
    pub fn parse() -> Self {
        Parser::parse()
    }
}

#[derive(Debug, Parser)]
pub enum Command {
    Config(Config),
    Define(Define),
    Search(Search),
}

#[derive(Debug, Parser)]
pub struct Config {
    pub root: String,
}

#[derive(Debug, Parser)]
pub struct Define {
    pub term: String,
}

#[derive(Debug, Parser)]
pub struct Search {
    pub tag: String,
}
