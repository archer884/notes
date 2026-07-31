use std::{collections::HashMap, fs, path::Path};

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
            self.by_tag.entry(tag.clone()).or_default().push(id);
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
        let mut tags: Vec<_> = self.by_tag.keys().map(|s| s.as_str()).collect();
        tags.sort_unstable();
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
        terms.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
        terms.dedup();
        terms
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
}
