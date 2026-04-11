use super::llm;
use super::graph::GraphEngine;
use std::path::Path;

use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    WorkingMemory,
    KnowledgeWiki,
    Skill,
    Persona,
    Postmortem,
    Spec,
    Onboard,
}

impl ProcessMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "skill" => Self::Skill,
            "wiki" => Self::KnowledgeWiki,
            "persona" => Self::Persona,
            "postmortem" => Self::Postmortem,
            "spec" => Self::Spec,
            "onboard" => Self::Onboard,
            _ => Self::WorkingMemory,
        }
    }
}

pub struct RefinementEngine;

impl RefinementEngine {
    pub async fn process(raw_data: &str, wiki_root: &Path, source_agent: &str, mode: ProcessMode, custom_output: Option<String>) -> anyhow::Result<String> {
        println!("Processing 2-Step Ingest for data length: {}", raw_data.len());
        
        let output_instruction = if let Some(ref out) = custom_output {
            format!("CRITICAL: You MUST save the final document exactly to this path: {}. Do not infer the subfolder.", out)
        } else {
            "Crucially, you must infer the correct subfolder based on the requested mode:
- `--mode wiki` -> `.wiki/concepts/` or `.wiki/entities/`
- `--mode skill` -> `.wiki/skills/`
- `--mode spec` -> `.wiki/specs/`
- `--mode onboard` -> `.wiki/onboards/`
- `--mode persona` -> `.wiki/personas/`
- `--mode postmortem` -> `.wiki/postmortems/`".to_string()
        };

        let prompt = match mode {
            ProcessMode::WorkingMemory => {
                format!(
                    "You are a 'Working Memory Graph Librarian' operating under the 'llm_wiki' philosophy. \n\
                    Your mission is to read raw chat history from an AI coding assistant (Source: {}) and distill it into a highly actionable, \n\
                    deeply technical 'Session Working Memory Document'. \n\n\
                    {}\n\n\
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
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            },
            ProcessMode::KnowledgeWiki => {
                format!(
                    "You are a 'Knowledge Base Architect' operating under the 'llm_wiki' philosophy. \n\
                    Your mission is to read the provided documents, files, or webpages (Source: {}) and distill them into a highly structured 'Knowledge Wiki Document'. \n\n\
                    {}\n\n\
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
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            },
            ProcessMode::Skill => {
                format!(
                    "You are an 'AI Skill Creator'. Your mission is to read the provided documents, code, or webpages (Source: {}) and generate a complete, highly effective 'AI Skill Prompt' (.skill / SKILL.md format) based on the content. \n\n\
                    {}\n\n\
                    An AI Skill is a specialized system prompt that instructs an AI agent on how to perform a specific workflow, use specific tools, or adopt a specific persona. \n\
                    Instead of summarizing the text, you must write INSTRUCTIONS for an AI to follow to become an expert at what the text describes. \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    name: [Name of the skill, e.g., 'rust-expert', 'pdf-analyzer', 'api-router']\n\
                    description: [A concise description of what the skill does and when to use it]\n\
                    tags: [skill, prompt, agent_instruction]\n\
                    version: 1.0.0\n\
                    ---\n\n\
                    Followed by the actual Skill Prompt Content:\n\
                    # [Skill Name]\n\
                    [A concise description of what the skill does and when to use it]\n\n\
                    ## System Instructions\n\
                    [Detailed instructions, step-by-step workflows, constraints, and tool usage rules for the AI to follow when this skill is activated. Write this AS IF speaking directly to the AI agent. Use imperatives like 'You must...', 'Always check...', 'Step 1: ...']\n\n\
                    ## Examples / Edge Cases\n\
                    [Provide examples of how the AI should respond or behave based on the provided material]\n\n\
                    Raw Source Material for Skill Creation:\n\
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            },
            ProcessMode::Persona => {
                format!(
                    "You are an 'AI Psychologist & Developer Profiler'. Your mission is to read the provided chat history or documents (Source: {}) and extract the user's technical persona, coding preferences, architecture biases, and communication style. \n\n\
                    {}\n\n\
                    The goal is to create a 'Persona Profile' that future AI agents will read to instantly understand how this user likes their code written and how they prefer to interact (e.g., 'Hates Electron', 'Loves Rust', 'Prefers solo execution without asking'). \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [A highly specific title, e.g., 'User_Tech_Persona_Preferences']\n\
                    type: persona\n\
                    project: [Extract the project name if possible, else 'global']\n\
                    source_tool: {}\n\
                    tags: [persona, preferences, style]\n\
                    ---\n\n\
                    Followed by:\n\
                    # 1. Tech Stack & Architecture Biases\n\
                    What technologies does the user love or hate? What architectural patterns do they enforce?\n\n\
                    # 2. Coding Style & Conventions\n\
                    Do they prefer DRY or WET? Strong typing? Specific naming conventions? \n\n\
                    # 3. AI Interaction Preferences\n\
                    Do they want the AI to write the code directly, or discuss it first? Do they use specific terminology?\n\n\
                    Raw Source Material:\n\
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            },
            ProcessMode::Postmortem => {
                format!(
                    "You are a 'Principal SRE & Debugging Analyst'. Your mission is to read the provided chat history or logs (Source: {}) representing a resolved (or partially resolved) bug/incident, and generate a structured 'Postmortem Report'. \n\n\
                    {}\n\n\
                    The goal is to permanently record the symptom, the root cause, and the exact code fix so that future AIs can instantly recall this solution if the same error occurs. \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [A specific bug title, e.g., 'BugFix_Rust_Lifetime_Mutex_Panic']\n\
                    type: postmortem\n\
                    project: [Extract the project name if possible, else 'Unknown']\n\
                    source_tool: {}\n\
                    tags: [incident, bugfix, postmortem]\n\
                    ---\n\n\
                    Followed by:\n\
                    # 1. Symptom & Error Logs\n\
                    What exactly broke? Provide the exact error message or stack trace.\n\n\
                    # 2. Root Cause Analysis\n\
                    Why did it break? Explain the underlying technical reason.\n\n\
                    # 3. Resolution & Code Fix\n\
                    How was it fixed? Provide the exact code snippets showing the 'Before' and 'After' states, or the terminal commands used to fix it.\n\n\
                    Raw Incident Material:\n\
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            },
            ProcessMode::Spec => {
                format!(
                    "You are a 'Staff Product Engineer & Architect'. Your mission is to read the provided raw ideas, meeting notes, or chat history (Source: {}) and convert them into a structured 'Product Requirement & Architecture Spec (PRD/Spec)'. \n\n\
                    {}\n\n\
                    The goal is to bridge the gap between raw human thoughts and a formal specification that an AI coder can immediately start implementing. \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [A specific feature or project title, e.g., 'Spec_OAuth2_Integration']\n\
                    type: spec\n\
                    project: [Extract the project name if possible, else 'Unknown']\n\
                    source_tool: {}\n\
                    tags: [prd, architecture, specification]\n\
                    ---\n\n\
                    Followed by:\n\
                    # 1. Product Requirements\n\
                    What are we building? List the core user stories and features.\n\n\
                    # 2. Architecture & Tech Design\n\
                    What is the technical approach? Define the data models, API endpoints, or component structures needed.\n\n\
                    # 3. Implementation Steps\n\
                    Break down the development into actionable, sequential tasks for an AI to execute.\n\n\
                    Raw Ideas Material:\n\
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            },
            ProcessMode::Onboard => {
                format!(
                    "You are a 'Senior Tech Lead'. Your mission is to read the provided codebase files, READMEs, or directory structures (Source: {}) and generate a comprehensive 'Project Onboarding Guide'. \n\n\
                    {}\n\n\
                    The goal is to create a document that a brand new AI assistant can read to instantly understand the entire project structure, entry points, and how to start contributing without needing to scan every file. \n\n\
                    You MUST output a single Markdown file starting with this YAML frontmatter:\n\
                    ---\n\
                    title: [Project Name, e.g., 'Project_AgentWikiOS_Onboarding']\n\
                    type: onboard\n\
                    project: [Extract the project name if possible, else 'Unknown']\n\
                    source_tool: {}\n\
                    tags: [onboarding, overview, architecture]\n\
                    ---\n\n\
                    Followed by:\n\
                    # 1. Project Overview\n\
                    What does this project do? What is its main purpose?\n\n\
                    # 2. Directory Structure & Key Files\n\
                    Explain the folder structure. Where is the main entry point? Where are the database models? Where are the routes/handlers?\n\n\
                    # 3. Tech Stack & Dependencies\n\
                    What languages, frameworks, and core libraries are used?\n\n\
                    # 4. How to Run / Test\n\
                    What are the commands to start the dev server, build, or test the project?\n\n\
                    Raw Codebase Material:\n\
                    {}", source_agent, output_instruction, source_agent, raw_data
                )
            }
        };
        
        let result = llm::ask_llm(&prompt).await?;
        
        // If result is empty, it means LLM is disabled and task file was written instead. Stop here.
        if result.is_empty() {
            return Ok("".to_string());
        }

        println!("LLM Output received. Writing to graph...");
        
        let graph = GraphEngine::new(wiki_root);
        
        if let Some(out_path) = custom_output {
            let path = std::path::PathBuf::from(&out_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap_or_default();
            }
            std::fs::write(&path, &result)?;
            println!("✅ Saved to custom path: {}", path.display());
            return Ok(path.to_string_lossy().to_string());
        }

        // Try to extract type and title from frontmatter
        let mut page_type = "source";
        let mut title = "Untitled";
        let mut project_title = "global";

        for line in result.lines() {
            if line.starts_with("type:") {
                page_type = line.replace("type:", "").trim().trim_matches(|c| c == '\'' || c == '"').to_lowercase().leak();
            } else if line.starts_with("title:") {
                title = line.replace("title:", "").trim().trim_matches(|c| c == '\'' || c == '"').to_string().leak();
            } else if line.starts_with("name:") && page_type == "skill" {
                title = line.replace("name:", "").trim().trim_matches(|c| c == '\'' || c == '"').to_string().leak();
            } else if line.starts_with("project:") {
                project_title = line.replace("project:", "").trim().trim_matches(|c| c == '\'' || c == '"').to_string().leak();
            }
        }

        let final_title = match mode {
            ProcessMode::WorkingMemory => {
                // Prepend date and tool name for timeline ordering of chat history
                let current_date = Local::now().format("%Y%m%d").to_string();
                format!("{}_{}_{}", current_date, source_agent, project_title)
            },
            ProcessMode::Skill => {
                // Name it clearly as a skill
                format!("{}_Skill", title)
            },
            ProcessMode::Persona => {
                format!("{}_Persona", project_title)
            },
            ProcessMode::Postmortem => {
                let current_date = Local::now().format("%Y%m%d").to_string();
                format!("{}_{}_Postmortem", current_date, project_title)
            },
            ProcessMode::Spec => {
                format!("{}_Spec", title)
            },
            ProcessMode::Onboard => {
                format!("{}_Onboarding", project_title)
            },
            ProcessMode::KnowledgeWiki => {
                // Just use the project and title
                title.to_string()
            }
        };
        
        let saved_path = graph.write_page(page_type, &final_title, &result)?;
        println!("✅ Saved to wiki: {}", saved_path.display());
        
        Ok(saved_path.to_string_lossy().to_string())
    }
}
