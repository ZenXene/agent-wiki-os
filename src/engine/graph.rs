use std::path::{Path, PathBuf};
use std::fs;
use crate::engine::vector::VectorStore;
use std::env;

pub struct GraphEngine {
    wiki_root: PathBuf,
    vector_store: Option<VectorStore>,
}

impl GraphEngine {
    pub async fn new(wiki_root: &Path) -> Self {
        // Ensure standard directories exist
        let entities_dir = wiki_root.join("entities");
        let concepts_dir = wiki_root.join("concepts");
        let sources_dir = wiki_root.join("sources");
        let skills_dir = wiki_root.join("skills");
        let personas_dir = wiki_root.join("personas");
        let postmortems_dir = wiki_root.join("postmortems");
        let specs_dir = wiki_root.join("specs");
        let onboards_dir = wiki_root.join("onboards");
        
        fs::create_dir_all(&entities_dir).unwrap_or_default();
        fs::create_dir_all(&concepts_dir).unwrap_or_default();
        fs::create_dir_all(&sources_dir).unwrap_or_default();
        fs::create_dir_all(&skills_dir).unwrap_or_default();
        fs::create_dir_all(&personas_dir).unwrap_or_default();
        fs::create_dir_all(&postmortems_dir).unwrap_or_default();
        fs::create_dir_all(&specs_dir).unwrap_or_default();
        fs::create_dir_all(&onboards_dir).unwrap_or_default();
        
        let disable_vector_store = env::var("WIKI_DISABLE_VECTOR_DB")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let vector_store = if disable_vector_store {
            None
        } else {
            match VectorStore::new(wiki_root).await {
                Ok(store) => Some(store),
                Err(e) => {
                    eprintln!("⚠️ [Graph] Failed to initialize Vector DB (LanceDB): {}. Falling back to standard mode.", e);
                    None
                }
            }
        };

        Self {
            wiki_root: wiki_root.to_path_buf(),
            vector_store,
        }
    }

    pub async fn write_page(&self, page_type: &str, title: &str, content: &str) -> anyhow::Result<PathBuf> {
        let dir = match page_type {
            "entity" => self.wiki_root.join("entities"),
            "concept" => self.wiki_root.join("concepts"),
            "skill" => self.wiki_root.join("skills"),
            "persona" => self.wiki_root.join("personas"),
            "postmortem" => self.wiki_root.join("postmortems"),
            "spec" => self.wiki_root.join("specs"),
            "onboard" => self.wiki_root.join("onboards"),
            _ => self.wiki_root.join("sources"),
        };
        
        fs::create_dir_all(&dir).unwrap_or_default();
        
        let safe_title = title.replace(|c: char| !c.is_alphanumeric() && c != '-', "_");
        let file_path = if page_type == "skill" {
            dir.join(format!("{}.skill", safe_title))
        } else {
            dir.join(format!("{}.md", safe_title))
        };
        
        fs::write(&file_path, content)?;
        
        // Sync with Vector DB for Hybrid RAG
        if let Some(store) = &self.vector_store {
            if let Err(e) = store.upsert_document(&file_path, content, page_type, title).await {
                eprintln!("⚠️ [Graph] Failed to index document in Vector DB: {}", e);
            }
        }
        
        Ok(file_path)
    }

    pub fn get_vector_store(&self) -> Option<&VectorStore> {
        self.vector_store.as_ref()
    }
}
