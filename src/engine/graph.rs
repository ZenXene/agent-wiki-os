use std::path::{Path, PathBuf};
use std::fs;

pub struct GraphEngine {
    wiki_root: PathBuf,
}

impl GraphEngine {
    pub fn new(wiki_root: &Path) -> Self {
        // Ensure standard directories exist
        let entities_dir = wiki_root.join("entities");
        let concepts_dir = wiki_root.join("concepts");
        let sources_dir = wiki_root.join("sources");
        let skills_dir = wiki_root.join("skills");
        
        fs::create_dir_all(&entities_dir).unwrap_or_default();
        fs::create_dir_all(&concepts_dir).unwrap_or_default();
        fs::create_dir_all(&sources_dir).unwrap_or_default();
        fs::create_dir_all(&skills_dir).unwrap_or_default();
        
        Self {
            wiki_root: wiki_root.to_path_buf(),
        }
    }

    pub fn write_page(&self, page_type: &str, title: &str, content: &str) -> anyhow::Result<PathBuf> {
        let dir = match page_type {
            "entity" => self.wiki_root.join("entities"),
            "concept" => self.wiki_root.join("concepts"),
            "skill" => self.wiki_root.join("skills"),
            _ => self.wiki_root.join("sources"),
        };
        
        // Ensure the target directory exists just in case it was deleted
        fs::create_dir_all(&dir).unwrap_or_default();
        
        // Sanitize filename
        let safe_title = title.replace(|c: char| !c.is_alphanumeric() && c != '-', "_");
        let file_path = if page_type == "skill" {
            dir.join(format!("{}.skill", safe_title))
        } else {
            dir.join(format!("{}.md", safe_title))
        };
        
        fs::write(&file_path, content)?;
        Ok(file_path)
    }
}
