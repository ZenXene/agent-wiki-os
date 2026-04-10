use walkdir::WalkDir;
use std::path::Path;

pub struct FsAdapter {
    pub target_dir: String,
}

impl FsAdapter {
    pub fn new(dir: &str) -> Self {
        Self {
            target_dir: dir.to_string(),
        }
    }

    pub fn fetch_all(&self) -> anyhow::Result<Vec<String>> {
        let mut contents = Vec::new();
        
        for entry in WalkDir::new(&self.target_dir)
            .into_iter()
            .filter_map(|e| e.ok()) 
        {
            let path = entry.path();
            if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    contents.push(format!("File: {}\n\n{}", path.display(), content));
                }
            }
        }
        
        Ok(contents)
    }
}
