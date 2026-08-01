mod parser;

pub use parser::Parser;

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Note,
    Fixme,
    Todo,
    Define { term: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: PathBuf,
    pub line: usize,
    pub kind: Kind,
    pub tags: Vec<String>,
    pub text: String,
}

impl Note {
    pub fn is_fixme(&self) -> bool {
        matches!(self.kind, Kind::Fixme)
    }

    pub fn is_todo(&self) -> bool {
        matches!(self.kind, Kind::Todo)
    }

    pub fn search_text(&self) -> String {
        match &self.kind {
            Kind::Define { term } => format!("{term} {}", self.text),
            _ => self.text.clone(),
        }
    }
}
