use std::{
    collections::HashMap,
    env, io,
    path::{Path, PathBuf},
};

use abseil::Provider;
use serde::{Deserialize, Serialize};

use crate::error::Error;

const APP_NAME: &str = "notes";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AppState {
    /// Canonical directory path → scan config
    #[serde(default)]
    pub directories: HashMap<String, DirConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DirConfig {
    pub glob: String,
}

fn provider() -> crate::Result<Provider> {
    Ok(Provider::builder(APP_NAME)
        .pretty()
        .use_config_dir()
        .with_filename("config.json")
        .build()?)
}

impl AppState {
    pub fn load() -> crate::Result<Self> {
        let provider = provider()?;
        match provider.load::<AppState>() {
            Ok(state) => Ok(state),
            Err(abseil::Error::NotFound) => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self) -> crate::Result<()> {
        provider()?.store(self)?;
        Ok(())
    }

    pub fn get(&self, dir: &Path) -> Option<&DirConfig> {
        self.directories.get(&key(dir))
    }

    pub fn set(&mut self, dir: &Path, config: DirConfig) {
        self.directories.insert(key(dir), config);
    }
}

fn key(dir: &Path) -> String {
    dir.to_string_lossy().into_owned()
}

pub fn current_dir() -> crate::Result<PathBuf> {
    let dir = env::current_dir()?;
    Ok(dir.canonicalize()?)
}

/// Load config for the current directory, prompting on first run if missing.
pub fn load_or_prompt() -> crate::Result<DirConfig> {
    let dir = current_dir()?;
    let mut state = AppState::load()?;

    if let Some(config) = state.get(&dir).cloned() {
        return Ok(config);
    }

    let glob = prompt_glob(&dir)?;
    let config = DirConfig { glob };
    state.set(&dir, config.clone());
    state.save()?;
    Ok(config)
}

pub fn set_glob(glob: impl AsRef<str>) -> crate::Result<DirConfig> {
    let dir = current_dir()?;
    let mut state = AppState::load()?;
    let config = DirConfig {
        glob: normalize_glob(glob.as_ref()),
    };
    if config.glob.is_empty() {
        return Err(Error::Config("glob must not be empty".into()));
    }
    state.set(&dir, config.clone());
    state.save()?;
    Ok(config)
}

pub fn show_config() -> crate::Result<Option<(PathBuf, DirConfig)>> {
    let dir = current_dir()?;
    let state = AppState::load()?;
    Ok(state.get(&dir).cloned().map(|c| (dir, c)))
}

fn prompt_glob(dir: &Path) -> crate::Result<String> {
    eprintln!("No notes config for {}.", dir.display());
    eprintln!("Enter a glob of files to scan (e.g. **/*.md — no quotes needed here):");
    eprint!("> ");

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let glob = normalize_glob(line.trim());

    if glob.is_empty() {
        return Err(Error::Config(
            "glob must not be empty; re-run and provide a pattern, or use `notes config <glob>`"
                .into(),
        ));
    }

    Ok(glob)
}

/// Strip surrounding quotes users may paste from shell examples.
fn normalize_glob(raw: &str) -> String {
    let s = raw.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let (first, last) = (bytes[0], bytes[bytes.len() - 1]);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].trim().to_string();
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_glob;

    #[test]
    fn strips_double_quotes() {
        assert_eq!(normalize_glob(r#""**/*.md""#), "**/*.md");
    }

    #[test]
    fn strips_single_quotes() {
        assert_eq!(normalize_glob("'src/*.md'"), "src/*.md");
    }

    #[test]
    fn leaves_bare_glob() {
        assert_eq!(normalize_glob("**/*.md"), "**/*.md");
    }
}
