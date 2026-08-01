use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use glob::glob;

use crate::note::{Kind, Note, Parser};

#[derive(Debug, Default)]
pub struct NoteStore {
    notes: Vec<Note>,
    by_tag: HashMap<String, Vec<usize>>,
    by_term: HashMap<String, Vec<usize>>,
    fixmes: Vec<usize>,
}

impl NoteStore {
    pub fn load(pattern: &str) -> crate::Result<Self> {
        let parser = Parser::new();
        let mut store = Self::default();

        for entry in glob(pattern)? {
            let path = match entry {
                Ok(path) => path,
                Err(e) => {
                    tracing::warn!(error = %e, "glob entry error");
                    continue;
                }
            };
            if !path.is_file() {
                continue;
            }
            store.index_file(&parser, &path)?;
        }

        Ok(store)
    }

    fn index_file(&mut self, parser: &Parser, path: &Path) -> crate::Result<()> {
        let source = fs::read_to_string(path)?;
        for note in parser.parse_file(path, &source) {
            self.push(note);
        }
        Ok(())
    }

    fn push(&mut self, note: Note) {
        let id = self.notes.len();

        for tag in &note.tags {
            self.by_tag
                .entry(normalize_tag(tag))
                .or_default()
                .push(id);
        }

        if let Kind::Define { term } = &note.kind {
            self.by_term
                .entry(normalize_term(term))
                .or_default()
                .push(id);
        }

        if note.is_fixme() {
            self.fixmes.push(id);
        }

        self.notes.push(note);
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn tags(&self) -> Vec<&str> {
        let mut seen = HashSet::new();
        let mut tags = Vec::new();
        for note in &self.notes {
            for tag in &note.tags {
                if seen.insert(normalize_tag(tag)) {
                    tags.push(tag.as_str());
                }
            }
        }
        tags.sort_by_cached_key(|t| normalize_tag(t));
        tags
    }

    pub fn terms(&self) -> Vec<&str> {
        let mut terms: Vec<_> = self
            .notes
            .iter()
            .filter_map(|n| match &n.kind {
                Kind::Define { term } => Some(term.as_str()),
                _ => None,
            })
            .collect();
        terms.sort_by_cached_key(|a| a.to_ascii_lowercase());
        terms.dedup();
        terms
    }

    /// Notes carrying ALL the given tags. Empty slice → every note.
    pub fn search_tags(&self, tags: &[String]) -> Vec<&Note> {
        let mut iter = tags.iter();
        let Some(first) = iter.next() else {
            return self.notes.iter().collect();
        };
        let mut ids = self
            .by_tag
            .get(&normalize_tag(first))
            .cloned()
            .unwrap_or_default();
        ids.dedup();
        for tag in iter {
            let Some(set) = self.by_tag.get(&normalize_tag(tag)) else {
                return Vec::new();
            };
            ids.retain(|id| set.binary_search(id).is_ok());
        }
        ids.into_iter().map(|id| &self.notes[id]).collect()
    }

    pub fn search_tag(&self, tag: &str) -> Vec<&Note> {
        let key = normalize_tag(tag);
        self.by_tag
            .get(&key)
            .into_iter()
            .flat_map(|ids| ids.iter().copied())
            .map(|id| &self.notes[id])
            .collect()
    }

    pub fn define(&self, term: &str) -> Vec<&Note> {
        let key = normalize_term(term);
        self.by_term
            .get(&key)
            .into_iter()
            .flat_map(|ids| ids.iter().copied())
            .map(|id| &self.notes[id])
            .collect()
    }

    pub fn errata(&self) -> Vec<&Note> {
        self.fixmes.iter().map(|&id| &self.notes[id]).collect()
    }

    /// All definitions, sorted by term.
    pub fn glossary(&self) -> Vec<&Note> {
        let mut notes: Vec<_> = self
            .by_term
            .values()
            .flat_map(|ids| ids.iter().copied())
            .map(|id| &self.notes[id])
            .collect();
        notes.sort_by(|a, b| {
            let ta = match &a.kind {
                Kind::Define { term } => term.as_str(),
                _ => "",
            };
            let tb = match &b.kind {
                Kind::Define { term } => term.as_str(),
                _ => "",
            };
            ta.to_ascii_lowercase()
                .cmp(&tb.to_ascii_lowercase())
                .then_with(|| a.path.cmp(&b.path).then(a.line.cmp(&b.line)))
        });
        notes
    }

    pub fn get(&self, id: usize) -> Option<&Note> {
        self.notes.get(id)
    }
}

fn normalize_tag(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('#')
        .replace(' ', "_")
        .to_ascii_lowercase()
}

fn normalize_term(term: &str) -> String {
    term.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{Kind, Note};
    use std::path::PathBuf;

    fn define_note(term: &str, gloss: &str) -> Note {
        Note {
            path: PathBuf::from("t.md"),
            line: 1,
            kind: Kind::Define { term: term.into() },
            tags: vec![],
            text: gloss.into(),
        }
    }

    fn tagged_note(text: &str, tags: &[&str]) -> Note {
        Note {
            path: PathBuf::from("t.md"),
            line: 1,
            kind: Kind::Note,
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
            text: text.into(),
        }
    }

    fn store_with(notes: Vec<Note>) -> NoteStore {
        let mut store = NoteStore::default();
        for n in notes {
            store.push(n);
        }
        store
    }

    #[test]
    fn define_lookup_single_word() {
        let store = store_with(vec![define_note("spearsheaves", "a tax.")]);
        let found = store.define("spearsheaves");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn define_lookup_multi_word() {
        let store = store_with(vec![define_note("blue bear", "a large mammal.")]);
        let found = store.define("blue bear");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "a large mammal.");
    }

    #[test]
    fn define_lookup_multi_word_case_insensitive() {
        let store = store_with(vec![define_note("blue bear", "a large mammal.")]);
        let found = store.define("Blue Bear");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn define_preserves_original_casing() {
        let store = store_with(vec![define_note("Blue Bear", "a large mammal.")]);
        let found = store.define("blue bear");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].kind,
            Kind::Define {
                term: "Blue Bear".into()
            }
        );
    }

    #[test]
    fn terms_preserve_casing_and_sort_case_insensitive() {
        let store = store_with(vec![
            define_note("zebra", "x"),
            define_note("Apple", "y"),
            define_note("blue bear", "z"),
        ]);
        assert_eq!(store.terms(), vec!["Apple", "blue bear", "zebra"]);
    }

    #[test]
    fn define_lookup_miss() {
        let store = store_with(vec![define_note("blue bear", "a large mammal.")]);
        assert!(store.define("red fox").is_empty());
    }

    #[test]
    fn search_tags_empty_returns_all() {
        let store = store_with(vec![tagged_note("one", &["a"]), tagged_note("two", &["b"])]);
        let found = store.search_tags(&[]);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn search_tags_intersects() {
        let store = store_with(vec![
            tagged_note("one", &["a", "b"]),
            tagged_note("two", &["a"]),
            tagged_note("three", &["b"]),
            tagged_note("four", &["a", "b", "c"]),
        ]);
        let tags = vec!["a".to_string(), "b".to_string()];
        let found = store.search_tags(&tags);
        let texts: Vec<&str> = found.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, vec!["one", "four"]);
    }

    #[test]
    fn search_tags_missing_tag_returns_none() {
        let store = store_with(vec![tagged_note("one", &["a"])]);
        let tags = vec!["a".to_string(), "zzz".to_string()];
        assert!(store.search_tags(&tags).is_empty());
    }

    #[test]
    fn search_tags_normalizes_and_dedups() {
        let store = store_with(vec![tagged_note("one", &["a", "a"])]);
        let tags = vec!["#A".to_string()];
        let found = store.search_tags(&tags);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn search_tag_spaces_and_underscores_interchangeable() {
        let store = store_with(vec![
            tagged_note("under", &["foo_bar"]),
            tagged_note("space", &["foo bar"]),
            tagged_note("other", &["baz"]),
        ]);
        for query in ["foo_bar", "foo bar", "Foo Bar", "#foo_bar"] {
            let found = store.search_tag(query);
            let texts: Vec<&str> = found.iter().map(|n| n.text.as_str()).collect();
            assert_eq!(texts, vec!["under", "space"], "query {query:?}");
        }
        let tags = vec!["foo bar".to_string()];
        let found = store.search_tags(&tags);
        let texts: Vec<&str> = found.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, vec!["under", "space"]);
    }

    #[test]
    fn tags_list_preserves_casing_dedupes_normalized() {
        let store = store_with(vec![
            tagged_note("a", &["Foo Bar"]),
            tagged_note("b", &["foo_bar"]),
            tagged_note("c", &["Baz"]),
            tagged_note("d", &["#baz"]),
        ]);
        assert_eq!(store.tags(), vec!["Baz", "Foo Bar"]);
        assert_eq!(store.search_tag("foo_bar").len(), 2);
        assert_eq!(store.search_tag("BAZ").len(), 2);
    }
}
