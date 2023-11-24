use serde::{Deserialize, Serialize};

use crate::Config;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub root: String,
}

impl Configuration {
    pub fn from_command(command: &Config) -> Self {
        Self {
            root: command.root.clone(),
        }
    }
}
