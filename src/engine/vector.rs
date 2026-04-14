use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;

// A simple in-memory vector store that persists to JSON
pub struct VectorStore {
    db_path: PathBuf,
    model: Option<TextEmbedding>,
    // Path -> (Type, Title, Content, Vector)
    index: std::sync::RwLock<HashMap<String, (String, String, String, Vec<f32>)>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IndexData {
    records: HashMap<String, (String, String, String, Vec<f32>)>,
}

impl VectorStore {
    pub async fn new(wiki_root: &Path) -> anyhow::Result<Self> {
        let db_path = wiki_root.join(".simple_vector_db.json");
        
        let model = if std::env::var("WIKI_DISABLE_VECTOR_DB").is_ok() {
            println!("[Vector DB] Disabled via environment variable");
            None
        } else {
            let mut options = InitOptions::new(EmbeddingModel::AllMiniLML6V2);
            options.show_download_progress = true;
            Some(TextEmbedding::try_new(options)?)
        };

        let index = if db_path.exists() {
            let data = fs::read_to_string(&db_path)?;
            let index_data: IndexData = serde_json::from_str(&data).unwrap_or(IndexData { records: HashMap::new() });
            std::sync::RwLock::new(index_data.records)
        } else {
            std::sync::RwLock::new(HashMap::new())
        };

        Ok(Self {
            db_path,
            model,
            index,
        })
    }

    fn save(&self) -> anyhow::Result<()> {
        let records = self.index.read().unwrap().clone();
        let data = IndexData { records };
        let json = serde_json::to_string(&data)?;
        fs::write(&self.db_path, json)?;
        Ok(())
    }

    pub async fn upsert_document(&self, path: &Path, content: &str, doc_type: &str, title: &str) -> anyhow::Result<()> {
        let snippet = content.chars().take(1500).collect::<String>();
        let vector = if let Some(model) = &self.model {
            let embeddings = model.embed(vec![snippet], None)?;
            embeddings[0].clone()
        } else {
            vec![0.0; 384] // Mock vector if disabled
        };
        
        let path_str = path.to_string_lossy().to_string();
        
        {
            let mut idx = self.index.write().unwrap();
            idx.insert(path_str, (doc_type.to_string(), title.to_string(), content.to_string(), vector));
        }
        
        self.save()?;
        Ok(())
    }

    pub async fn search(&self, query: &str, type_filter: Option<&str>, top_k: usize) -> anyhow::Result<Vec<String>> {
        let query_vec = if let Some(model) = &self.model {
            let embeddings = model.embed(vec![query], None)?;
            embeddings[0].clone()
        } else {
            vec![0.0; 384] // Mock vector if disabled
        };
        
        let idx = self.index.read().unwrap();
        if idx.is_empty() {
            return Ok(vec!["Index is empty.".to_string()]);
        }
        
        let mut scored_results: Vec<(f32, String, String, String, String)> = idx.iter()
            .filter(|(_, (doc_type, _, _, _))| {
                if let Some(t) = type_filter {
                    doc_type == t
                } else {
                    true
                }
            })
            .map(|(path, (doc_type, title, content, vector))| {
                let score = cosine_similarity(&query_vec, vector);
                (score, path.clone(), title.clone(), doc_type.clone(), content.clone())
            })
            .collect();
            
        // Sort descending by score
        scored_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut results = Vec::new();
        for (_, path, title, doc_type, content) in scored_results.into_iter().take(top_k) {
            let summary = content.chars().take(150).collect::<String>();
            results.push(format!("File: {}\nTitle: [{}] (Type: {})\nSummary: {}...", path, title, doc_type, summary));
        }
        
        Ok(results)
    }

    pub async fn get_all_documents(&self, type_filter: Option<&str>) -> anyhow::Result<Vec<(String, String, Vec<f32>)>> {
        let idx = self.index.read().unwrap();
        let mut docs = Vec::new();
        
        for (path, (doc_type, _, content, vector)) in idx.iter() {
            if let Some(t) = type_filter {
                if doc_type != t {
                    continue;
                }
            }
            docs.push((path.clone(), content.clone(), vector.clone()));
        }
        
        Ok(docs)
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
