use std::{fs, path::Path};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::note::{Inline, InlineParser, TagExtractor};

#[derive(Debug, Deserialize, Serialize)]
pub struct Index {
    pub comments: HashMap<String, Vec<Inline>>,
}

#[derive(Clone, Debug)]
pub struct Indexer {
    extract: TagExtractor,
    parse: InlineParser,
}

impl Indexer {
    pub fn new() -> Self {
        Self {
            extract: TagExtractor::new(),
            parse: InlineParser::new(),
        }
    }

    pub fn index_path(&self, path: impl AsRef<Path>) -> crate::Result<Index> {
        let comments = self.read_inlines(path.as_ref())?;
        let comments = comments
            .into_iter()
            .flat_map(|inline| {
                let normalized_tags: Vec<_> = inline
                    .tags
                    .iter()
                    .map(|t| {
                        t.replace(|u| u == '-' || u == '_', " ")
                            .to_ascii_lowercase()
                    })
                    .collect();
                normalized_tags
                    .into_iter()
                    .map(move |tag| (tag, inline.clone()))
            })
            .fold(HashMap::new(), |mut a: HashMap<_, Vec<_>>, (k, v)| {
                a.entry(k).or_default().push(v);
                a
            });

        Ok(Index { comments })
    }

    fn read_inlines(&self, path: &Path) -> crate::Result<Vec<Inline>> {
        let text = fs::read_to_string(path)?;
        let tags = self.extract.tags(&text);
        Ok(tags.map(|tag| self.parse.parse(tag)).collect())
    }
}
