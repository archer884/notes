use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::note::Comment;

#[derive(Debug, Deserialize, Serialize)]
pub struct Index {
    pub comments: HashMap<String, Vec<Comment>>,
    pub definitions: HashMap<String, String>,
}
