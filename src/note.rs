mod parser;

use std::fmt;

pub use parser::{InlineParser, ParseInlineError};
use regex::Regex;

#[derive(Clone)]
pub enum Inline {
    Comment(Box<Comment>),
    Definition(Box<Definition>),
}

impl fmt::Debug for Inline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comment(comment) => comment.fmt(f),
            Self::Definition(definition) => definition.fmt(f),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Comment {
    tags: Vec<String>,
    heading: Option<String>,
    comment: String,
}

#[derive(Clone, Debug)]
pub struct Definition {
    term: String,
    definition: String,
}

pub struct TagExtractor {
    tag_rx: Regex,
}

impl TagExtractor {
    pub fn new() -> TagExtractor {
        Self {
            tag_rx: Regex::new("<note[^>]+?>").unwrap(),
        }
    }

    pub fn tags<'a>(&'a self, text: &'a str) -> impl Iterator<Item = &str> {
        self.tag_rx.find_iter(text).map(move |cx| cx.as_str())
    }
}
