mod parser;

pub use parser::InlineParser;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inline {
    pub tags: Vec<String>,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct TagExtractor {
    tag_rx: Regex,
}

impl TagExtractor {
    pub fn new() -> TagExtractor {
        Self {
            tag_rx: Regex::new("<!-- (.+?) -->").unwrap(),
        }
    }

    pub fn tags<'a>(&'a self, text: &'a str) -> impl Iterator<Item = &str> {
        self.tag_rx.find_iter(text).map(move |cx| cx.as_str())
    }
}
