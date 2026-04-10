use super::llm;
use super::graph::GraphEngine;
use std::path::Path;

pub struct RefinementEngine;

impl RefinementEngine {
    pub async fn process(raw_data: &str, wiki_root: &Path) -> anyhow::Result<()> {
        println!("Processing 2-Step Ingest for data length: {}", raw_data.len());
        
        let prompt = format!("Extract key concepts and architecture decisions from this raw data:\n\n{}", raw_data);
        
        let result = llm::ask_llm(&prompt).await?;
        println!("LLM Output received. Writing to graph...");
        
        let graph = GraphEngine::new(wiki_root);
        
        // Very basic mock parser: look for type and title in the output
        // In a real scenario, this would use a robust YAML frontmatter parser.
        let mut page_type = "source";
        let mut title = "extracted_data";
        
        for line in result.lines() {
            if line.starts_with("type:") {
                page_type = line.split(':').nth(1).unwrap_or("source").trim();
            }
            if line.starts_with("title:") {
                title = line.split(':').nth(1).unwrap_or("extracted_data").trim();
            }
        }
        
        let saved_path = graph.write_page(page_type, title, &result)?;
        println!("Saved to wiki: {}", saved_path.display());
        
        Ok(())
    }
}
