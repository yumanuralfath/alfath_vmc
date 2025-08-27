use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TitleEntry {
    pub id: String,
    pub title: String,
    pub developer: String,
    pub genre: String,
    pub language: String,
    pub publisher: String,
    pub region: String,
    pub release_date: String,
}
