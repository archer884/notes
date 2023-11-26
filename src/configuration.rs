use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::cli::Config;

#[derive(Debug)]
pub struct ApplicationPaths {
    tools: PathBuf,
    config: PathBuf,
    cache: PathBuf,
}

impl ApplicationPaths {
    pub fn from_current() -> io::Result<Self> {
        let dir = env::current_dir()?;
        let tools = dir.join(".tool");
        Ok(Self {
            config: tools.join("notes.json"),
            cache: tools.join("notecache.bin.gz"),
            tools,
        })
    }

    pub fn tools(&self) -> &Path {
        &self.tools
    }

    pub fn config(&self) -> &Path {
        &self.config
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub root: String,
}

impl Configuration {
    pub fn load(path: &Path) -> io::Result<Self> {
        let s = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&s)?)
    }

    pub fn from_command(command: &Config) -> Self {
        Self {
            root: command.root.clone(),
        }
    }
}
