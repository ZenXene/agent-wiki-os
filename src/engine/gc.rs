use std::path::{Path, PathBuf};
use crate::engine::vector::VectorStore;
use crate::engine::ingest::{RefinementEngine, ProcessMode};
use crate::config::AppConfig;
use std::collections::HashMap;
use std::str::FromStr;

pub struct GCEngine {
    wiki_root: PathBuf,
    vector_store: VectorStore,
}

impl GCEngine {
    pub async fn new(wiki_root: &Path) -> anyhow::Result<Self> {
        let vector_store = VectorStore::new(wiki_root).await?;
        Ok(Self {
            wiki_root: wiki_root.to_path_buf(),
            vector_store,
        })
    }

    pub async fn run_gc_sweep(&self, config: &AppConfig) -> anyhow::Result<()> {
        println!("🧹 [GC] Starting memory lifecycle sweep...");
        
        let types_to_check = vec!["concept", "postmortem"];
        for doc_type in types_to_check {
            let docs = self.vector_store.get_all_documents(Some(doc_type)).await?;
            if docs.len() < 2 {
                continue;
            }
            
            // Find highly similar pairs
            let mut merged_paths = std::collections::HashSet::new();
            
            for i in 0..docs.len() {
                if merged_paths.contains(&docs[i].0) { continue; }
                
                for j in (i + 1)..docs.len() {
                    if merged_paths.contains(&docs[j].0) { continue; }
                    
                    let sim = cosine_similarity(&docs[i].2, &docs[j].2);
                    if sim > 0.88 {
                        println!("🔍 [GC] Found highly similar documents (sim: {:.2}):\n  - {}\n  - {}", sim, docs[i].0, docs[j].0);
                        
                        // Resolve conflict
                        let combined_content = format!(
                            "--- DOCUMENT 1 ---\n{}\n\n--- DOCUMENT 2 ---\n{}",
                            docs[i].1, docs[j].1
                        );
                        
                        let mode = ProcessMode::from_str(doc_type);
                        
                        // We use the RefinementEngine to process the merged content
                        // If llm.enable is true, this will do an API call and write the new file
                        // If llm.enable is false, this will generate a task_gc.md file for the IDE
                        let agent_name = "gc_daemon";
                        match RefinementEngine::process(&combined_content, &self.wiki_root, agent_name, mode, None).await {
                            Ok(new_path) => {
                                println!("✅ [GC] Successfully merged into: {}", new_path);
                                
                                // In a fully autonomous mode, we would delete the old files here.
                                // However, if it's TaskFile mode, the IDE LLM should delete them.
                                // For safety, we only delete if llm.enable is true.
                                if config.llm.enable {
                                    let _ = std::fs::remove_file(&docs[i].0);
                                    let _ = std::fs::remove_file(&docs[j].0);
                                    println!("🗑️  [GC] Cleaned up old files.");
                                } else {
                                    println!("ℹ️  [GC] TaskFile generated. IDE will perform cleanup.");
                                }
                                
                                merged_paths.insert(docs[i].0.clone());
                                merged_paths.insert(docs[j].0.clone());
                            },
                            Err(e) => eprintln!("❌ [GC] Failed to merge documents: {}", e),
                        }
                    }
                }
            }
        }
        
        println!("✨ [GC] Sweep complete.");
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}