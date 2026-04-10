use super::llm;
use super::graph::GraphEngine;
use std::path::Path;

pub struct RefinementEngine;

impl RefinementEngine {
    pub async fn process(raw_data: &str, wiki_root: &Path, source_agent: &str) -> anyhow::Result<()> {
        println!("Processing 2-Step Ingest for data length: {}", raw_data.len());
        
        let prompt = format!(
            "You are a 'Working Memory Graph Librarian' operating under the 'llm_wiki' philosophy. \n\
            Your mission is to read raw chat history from an AI coding assistant (Source: {}) and distill it into a highly actionable, \n\
            deeply technical 'Session Working Memory Document'. \n\n\
            The goal of this document is NOT to write a high-level philosophy whitepaper. \n\
            The goal is context restoration: if an AI reads this tomorrow in a DIFFERENT tool, it should instantly know exactly what the user is building, \n\
            where they left off, what exact files were touched, and what the current bugs/blockers are.\n\n\
            You MUST output a single Markdown file starting with this YAML frontmatter:\n\
            ---\n\
            title: [A highly specific context title, e.g., 'Context_ProjectName_TaskName']\n\
            type: source\n\
            project: [Extract the project name if possible, else 'Unknown']\n\
            source_tool: {}\n\
            tags: [working_memory, context, languages]\n\
            ---\n\n\
            Followed by these precise sections:\n\
            # 1. Current Working Context\n\
            Exactly what task was being executed? What is the current state of the project?\n\n\
            # 2. File & Code Anchors\n\
            List the exact file paths modified and the specific structs/functions changed. Provide critical code snippets if they define the architecture.\n\n\
            # 3. Environment & Commands\n\
            What terminal commands were executed? What environment variables or dependencies were used?\n\n\
            # 4. Blockers & Next Steps\n\
            Where did the session stop? What errors are currently unresolved? What is the immediate next step for the AI to take when it resumes?\n\n\
            Raw Chat History:\n\
            {}", source_agent, source_agent, raw_data
        );
        
        let result = llm::ask_llm(&prompt).await?;
        println!("LLM Output received. Writing to graph...");
        
        let graph = GraphEngine::new(wiki_root);
        
        let mut page_type = "source";
        let mut title = "extracted_data";
        let mut project = "global";
        
        for line in result.lines() {
            if line.starts_with("type:") {
                page_type = line.split(':').nth(1).unwrap_or("source").trim();
            }
            if line.starts_with("title:") {
                title = line.split(':').nth(1).unwrap_or("extracted_data").trim();
            }
            if line.starts_with("project:") {
                project = line.split(':').nth(1).unwrap_or("global").trim();
            }
        }
        
        // Include project name in the title for better cross-tool isolation if it's not already there
        let final_title = if title.to_lowercase().contains(&project.to_lowercase()) || project == "global" || project == "Unknown" {
            title.to_string()
        } else {
            format!("{}_{}", project, title)
        };
        
        let saved_path = graph.write_page(page_type, &final_title, &result)?;
        println!("Saved to wiki: {}", saved_path.display());
        
        Ok(())
    }
}
