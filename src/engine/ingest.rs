use super::llm;

pub struct RefinementEngine;

impl RefinementEngine {
    pub async fn process(raw_data: &str) -> anyhow::Result<()> {
        println!("Processing 2-Step Ingest for data length: {}", raw_data.len());
        
        let prompt = format!("Extract key concepts and architecture decisions from this raw data:\n\n{}", raw_data);
        
        let result = llm::ask_llm(&prompt).await?;
        println!("LLM Output:\n{}\n", result);
        
        Ok(())
    }
}
