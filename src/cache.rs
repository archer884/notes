use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::{error::Error, index::Index, note::Comment};

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CacheKey {
    pub path: PathBuf,
    pub modified: SystemTime,
}

impl CacheKey {
    pub fn from_path(path: impl AsRef<Path> + Into<PathBuf>) -> crate::Result<Self> {
        let meta = fs::metadata(&path)?;
        if !meta.is_file() {
            tracing::debug!(
                path = path.as_ref().display().to_string(),
                "path must reference file"
            );
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "expected file, found directory or link",
            )));
        }

        Ok(Self {
            path: path.into(),
            modified: meta.modified()?,
        })
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FileCache {
    pub map: HashMap<CacheKey, Index>,
}

impl FileCache {
    pub fn define(&self, term: &str) -> Option<&str> {
        self.map
            .values()
            .find_map(|x| x.definitions.get(term))
            .map(|x| x.as_ref())
    }

    pub fn search(&self, tag: &str) -> impl Iterator<Item = &Comment> {
        let mut comments_by_file: Vec<_> = self
            .map
            .iter()
            .filter_map(|(f, i)| i.comments.get(tag).map(|comments| (f, comments)))
            .collect();
        comments_by_file.sort_unstable_by(|a, b| a.0.modified.cmp(&b.0.modified));
        comments_by_file.into_iter().flat_map(|(_, c)| c)
    }
}
