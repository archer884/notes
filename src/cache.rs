use std::{
    fs, io,
    ops::Add,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::{error::Error, index::Index, note::Comment};

// FIXME: CacheKey requires a custom ser/de impl because the key of a json map must be a string.

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub path: PathBuf,
    pub modified: SystemTime,
}

impl CacheKey {
    pub fn from_path(path: impl AsRef<Path> + Into<PathBuf>) -> crate::Result<Self> {
        let meta = fs::metadata(&path)?;
        if !meta.is_file() {
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "expected file",
            )));
        }

        Ok(Self {
            path: path.into(),
            modified: meta.modified()?,
        })
    }
}

impl Serialize for CacheKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let t = self
            .modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let s = t.as_secs();
        let n = t.subsec_nanos();
        let s = format!("{}::{s}::{n}", self.path.display());
        s.serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for CacheKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        let segments: Vec<_> = s.split("::").collect();
        if let &[path, s, n] = &*segments {
            let s: u64 = s.parse().map_err(serde::de::Error::custom)?;
            let n: u64 = n.parse().map_err(serde::de::Error::custom)?;
            let d = Duration::from_secs(s) + Duration::from_nanos(n);
            Ok(Self {
                path: path.into(),
                modified: SystemTime::UNIX_EPOCH.add(d),
            })
        } else {
            Err(serde::de::Error::custom("bad format"))
        }
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
