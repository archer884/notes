use memory_indexer::InMemoryIndex;

use crate::note::Note;
use crate::store::NoteStore;

const INDEX: &str = "notes";

/// Lazy full-text index over note bodies (and define terms).
pub struct FtsIndex {
    index: InMemoryIndex,
}

impl FtsIndex {
    pub fn build(store: &NoteStore) -> Self {
        let mut index = InMemoryIndex::default();
        for (id, note) in store.notes().iter().enumerate() {
            let doc_id = id.to_string();
            let text = note.search_text();
            index.add_doc(INDEX, &doc_id, &text, true);
        }
        Self { index }
    }

    pub fn search_ids(&self, query: &str) -> Vec<usize> {
        self.index
            .search(INDEX, query)
            .into_iter()
            .filter_map(|(doc_id, _score)| doc_id.parse().ok())
            .collect()
    }

    pub fn search<'a>(&self, store: &'a NoteStore, query: &str) -> Vec<&'a Note> {
        self.search_ids(query)
            .into_iter()
            .filter_map(|id| store.get(id))
            .collect()
    }
}
