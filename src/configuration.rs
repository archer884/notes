use std::{collections::HashMap, fs, io, path::Path};

use serde::{Deserialize, Serialize};

pub trait Stash: serde::Serialize + for<'a> serde::Deserialize<'a> {
    fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        Ok(fs::write(path, contents)?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub root: String,
}

impl Stash for Configuration {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cache {
    pub comments: HashMap<String, Vec<String>>,
    pub definitions: HashMap<String, String>,
}

impl Stash for Cache {}
