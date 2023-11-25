use std::{fs, path::Path};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::note::{Comment, Definition, Inline, InlineParser, TagExtractor};

#[derive(Debug, Deserialize, Serialize)]
pub struct Index {
    pub comments: HashMap<String, Vec<Comment>>,
    pub definitions: HashMap<String, String>,
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
        let (comments, definitions) = self.read_inlines(path.as_ref())?;
        let comments = comments
            .into_iter()
            .flat_map(|c| c.tags.clone().into_iter().map(move |tag| (tag, c.clone())))
            .fold(HashMap::new(), |mut a: HashMap<_, Vec<_>>, (k, v)| {
                a.entry(k).or_default().push(v);
                a
            });

        let definitions = definitions
            .into_iter()
            .map(|Definition { term, definition }| (term, definition))
            .collect();

        Ok(Index {
            comments,
            definitions,
        })
    }

    fn read_inlines(&self, path: &Path) -> crate::Result<(Vec<Comment>, Vec<Definition>)> {
        let mut definitions = Vec::new();
        let mut comments = Vec::new();

        let text = fs::read_to_string(path)?;
        let tags = self.extract.tags(&text);
        let inlines: Result<Vec<_>, _> = tags.map(|tag| self.parse.parse(tag)).collect();

        for inline in inlines? {
            match inline {
                Inline::Comment(comment) => comments.push(*comment),
                Inline::Definition(definition) => definitions.push(*definition),
            }
        }

        Ok((comments, definitions))
    }
}
