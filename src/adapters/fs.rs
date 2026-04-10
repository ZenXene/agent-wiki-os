use std::path::Path;
use walkdir::WalkDir;

pub struct FsAdapter {
    pub target_path: String,
}

impl FsAdapter {
    pub fn new(path: &str) -> Self {
        Self {
            target_path: path.to_string(),
        }
    }

    pub fn fetch_all(&self) -> anyhow::Result<Vec<String>> {
        let mut results = Vec::new();
        let path = Path::new(&self.target_path);

        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", self.target_path);
        }

        if path.is_file() {
            // Process single file
            if let Ok(content) = self.read_file(path) {
                results.push(content);
            }
        } else {
            // Process directory
            for entry in WalkDir::new(&self.target_path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    if let Ok(content) = self.read_file(p) {
                        results.push(content);
                    }
                }
            }
        }
        
        Ok(results)
    }

    fn read_file(&self, path: &Path) -> anyhow::Result<String> {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        
        match extension.as_str() {
            "md" | "txt" | "rs" | "py" | "js" | "ts" | "json" | "toml" | "yaml" | "yml" | "html" | "css" | "csv" => {
                // Read as plain text
                let content = std::fs::read_to_string(path)?;
                Ok(format!("File: {}\n\n{}", path.display(), content))
            },
            "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" => {
                // TODO: Implement binary file parsing using crates like `pdf-extract` or `dotx`
                // For now, we return a placeholder to let the LLM know the file exists but isn't parsed
                Ok(format!("File: {}\n\n[Content extraction for binary format '{}' is not yet supported in this version]", path.display(), extension))
            },
            _ => {
                // Try reading as text anyway, if it fails, it's probably binary
                match std::fs::read_to_string(path) {
                    Ok(content) => Ok(format!("File: {}\n\n{}", path.display(), content)),
                    Err(_) => anyhow::bail!("Unsupported or binary file format"),
                }
            }
        }
    }
}
