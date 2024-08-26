use regex::Regex;

use crate::note::Inline;

#[derive(Clone, Debug)]
pub struct InlineParser {
    tags: Regex,
}

impl InlineParser {
    pub fn new() -> Self {
        Self {
            // #foo #bar_baz #define:saltare_temporum
            tags: Regex::new(r"#\S+").unwrap(),
        }
    }

    pub fn parse(&self, s: &str) -> Inline {
        let s = s.trim();
        let tags: Vec<_> = self
            .tags
            .captures_iter(s)
            .filter_map(|cx| Some(cx.get(0)?.as_str().trim_start_matches('#').to_string()))
            .collect();

        Inline {
            tags,
            text: s.into(),
        }
    }
}
