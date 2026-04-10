use super::llm;
use super::graph::GraphEngine;
use std::path::Path;

use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    WorkingMemory,
    KnowledgeWiki,
    Skill,
}

impl ProcessMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "skill" => Self::Skill,
            "wiki" => Self::KnowledgeWiki,
            _ => Self::WorkingMemory,
        }
    }
}

pub struct RefinementEngine;

impl RefinementEngine {
    pub async fn process(raw_data: &str, wiki_root: &Path, source_agent: &str, mode: ProcessMode) -> anyhow::Result<()> {
        println!("Processing 2-Step Ingest for data length: {}", raw_data.len());
        
        let prompt = match mode {
            ProcessMode::WorkingMemory => {
                format!(
                    "You are a 'Working Memory Graph Librarian' operating under the 'llm_wiki' philosophy. \n\
                    Your mission is to read raw chat history from an AI coding assistant (Source: {}) and distill it into a highly actionable, \n\
                    deeply technical 'Session Working Memory Document'. \n\n\
                    The goal of this document is NOT to write a high-level philosophy whitepaper. \n\
                    The goal is context restoration: if an AI reads this tomorrow in a DIFFERENT tool, it should instantly know exactly what the user is building, \n\
                    where they left off, what exact files were touched, and what the current bugs/blockers are.\n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [A highly specific context title, e.g., 'Context_ProjectName_TaskName']\n\
                    type: [Decide if this is a 'source' (working memory logs), 'entity' (concrete module explanation), or 'concept' (abstract architecture design)]\n\
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
                )
            },
            ProcessMode::KnowledgeWiki => {
                format!(
                    "You are a 'Knowledge Base Architect' operating under the 'llm_wiki' philosophy. \n\
                    Your mission is to read the provided documents, files, or webpages (Source: {}) and distill them into a highly structured 'Knowledge Wiki Document'. \n\n\
                    The goal is to extract core concepts, architecture, usage instructions, or domain knowledge so that an AI assistant can instantly understand this material later. \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [A highly specific title for this knowledge, e.g., 'Protocol_Architecture_Explanation']\n\
                    type: [Decide if this is an 'entity' (concrete module) or 'concept' (abstract theory/design)]\n\
                    project: [Extract the project name if possible, else 'global']\n\
                    source_tool: {}\n\
                    tags: [knowledge_base, documentation]\n\
                    ---\n\n\
                    Followed by sections appropriate for a Wiki, such as:\n\
                    # 1. Overview & Core Concept\n\
                    What is this about? Summarize the main idea.\n\n\
                    # 2. Key Architecture / Mechanisms\n\
                    How does it work? Extract the important technical details, workflows, or rules.\n\n\
                    # 3. Usage & Implementation Details\n\
                    Provide critical code snippets, API endpoints, or configurations found in the text.\n\n\
                    # 4. Important References\n\
                    Any important links or related entities mentioned.\n\n\
                    Raw Document Content:\n\
                    {}", source_agent, source_agent, raw_data
                )
            },
            ProcessMode::Skill => {
                format!(
                    "You are an 'AI Skill Creator'. Your mission is to read the provided documents, code, or webpages (Source: {}) and generate a complete, highly effective 'AI Skill Prompt' (.skill / SKILL.md format) based on the content. \n\n\
                    An AI Skill is a specialized system prompt that instructs an AI agent on how to perform a specific workflow, use specific tools, or adopt a specific persona. \n\
                    Instead of summarizing the text, you must write INSTRUCTIONS for an AI to follow to become an expert at what the text describes. \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [Name of the skill, e.g., 'rust-expert', 'pdf-analyzer', 'api-router']\n\
                    type: skill\n\
                    project: [Extract the project name if possible, else 'global']\n\
                    source_tool: {}\n\
                    tags: [skill, prompt, agent_instruction]\n\
                    ---\n\n\
                    Followed by the actual Skill Prompt Content:\n\
                    # [Skill Name]\n\
                    [A concise description of what the skill does and when to use it]\n\n\
                    ## System Instructions\n\
                    [Detailed instructions, step-by-step workflows, constraints, and tool usage rules for the AI to follow when this skill is activated. Write this AS IF speaking directly to the AI agent. Use imperatives like 'You must...', 'Always check...', 'Step 1: ...']\n\n\
                    ## Examples / Edge Cases\n\
                    [Provide examples of how the AI should respond or behave based on the provided material]\n\n\
                    Raw Source Material for Skill Creation:\n\
                    {}", source_agent, source_agent, raw_data
                )
            }
        };
        
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
        let project_title = if title.to_lowercase().contains(&project.to_lowercase()) || project == "global" || project == "Unknown" {
            title.to_string()
        } else {
            format!("{}_{}", project, title)
        };

        let final_title = match mode {
            ProcessMode::WorkingMemory => {
                // Prepend date and tool name for timeline ordering of chat history
                let current_date = Local::now().format("%Y%m%d").to_string();
                format!("{}_{}_{}", current_date, source_agent, project_title)
            },
            ProcessMode::Skill => {
                // Name it clearly as a skill
                format!("{}_Skill", project_title)
            },
            ProcessMode::KnowledgeWiki => {
                // Just use the project and title
                project_title
            }
        };
        
        let saved_path = graph.write_page(page_type, &final_title, &result)?;
        println!("Saved to wiki: {}", saved_path.display());
        
        Ok(())
    }
}
