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
    pattern: Regex,
}

impl TagExtractor {
    pub fn new() -> TagExtractor {
        Self {
            pattern: Regex::new("<!-- (.+?) -->").unwrap(),
        }
    }

    pub fn tags<'a>(&'a self, text: &'a str) -> impl Iterator<Item = &str> {
        self.pattern
            .captures_iter(text)
            .flat_map(move |cx| cx.get(1))
            .map(|cx| cx.as_str())
    }
}
