pub mod fs;
pub mod history;
pub mod web;

pub use fs::FsAdapter;
pub use history::HistoryAdapter;
pub use web::WebAdapter;
use std::collections::HashMap;

pub trait Adapter {
    // Fetches all history as a single string (fallback)
    fn fetch(&self) -> anyhow::Result<String>;
    // Fetches history grouped by the project path they belong to
    fn fetch_grouped_by_project(&self) -> anyhow::Result<HashMap<String, String>> {
        let all = self.fetch()?;
        let mut map = HashMap::new();
        map.insert("global".to_string(), all);
        Ok(map)
    }
}
