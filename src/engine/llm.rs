// removed unused import reqwest::Client
use serde::{Deserialize, Serialize};
use std::env;
use crate::config::AppConfig;
use std::path::PathBuf;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

pub async fn ask_llm(prompt: &str) -> anyhow::Result<String> {
    // 1. Try to load config from global dir
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let global_dir = home_dir.join(".agent-wiki-os");
    let config = AppConfig::load_or_create(&global_dir).unwrap_or_default();

    // 2. Cascade Configuration (Env Var > Config File > Default)
    let api_key = env::var("WIKI_API_KEY")
        .unwrap_or_else(|_| config.llm.api_key.clone());
        
    let base_url = env::var("WIKI_BASE_URL")
        .unwrap_or_else(|_| config.llm.base_url.clone());
        
    let model = env::var("WIKI_MODEL")
        .unwrap_or_else(|_| config.llm.model.clone());

    let is_mock = env::var("WIKI_MOCK")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(config.llm.mock);

    // 3. Check if LLM is enabled
    let is_enabled = env::var("WIKI_LLM_ENABLE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(config.llm.enable);

    if !is_enabled {
        println!("💡 [Agent-Wiki-OS] LLM execution is disabled (llm.enable=false).");
        println!("   Operating in 'Prompt Generation / IDE Proxy' mode.");
        
        // Write the prompt to a file bus so the IDE's LLM can read it
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let tasks_dir = cwd.join(".wiki").join(".awo_tasks");
        std::fs::create_dir_all(&tasks_dir).unwrap_or_default();
        
        use chrono::Local;
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let task_file = tasks_dir.join(format!("task_{}.md", timestamp));
        
        if let Err(e) = std::fs::write(&task_file, prompt) {
            eprintln!("❌ Failed to write task file: {}", e);
            return Ok("".to_string());
        }
        
        println!("✅ Task payload successfully generated at: {}", task_file.display());
        println!("   The IDE's Agent will now read this file to complete the process.");
        
        // Return empty string to prevent GraphEngine from writing anything
        return Ok("".to_string());
    }

    // Mock logic
    if is_mock || (api_key.is_empty() && !base_url.contains("localhost") && !base_url.contains("127.0.0.1")) {
        println!("\u{26a0}\u{fe0f}  [LLM] WIKI_MOCK is true or no API Key found. Returning mock output...");
        return Ok(format!(
            "---\n\
            title: Mock_Context\n\
            type: source\n\
            project: global\n\
            source_tool: mock_tool\n\
            tags: [mock]\n\
            ---\n\n\
            # 1. Current Working Context\n\
            Mocked task execution.\n\n\
            # 2. File & Code Anchors\n\
            Mocked files touched.\n\n\
            # 3. Environment & Commands\n\
            Mocked commands.\n\n\
            # 4. Blockers & Next Steps\n\
            Mocked blockers."
        ));
    }

    let client = reqwest::Client::new();
    let req_body = ChatRequest {
        model,
        messages: vec![
            Message { role: "system".to_string(), content: "You are a Wiki refinement engine. Output valid markdown with YAML frontmatter containing title, type (entity/concept), and tags.".to_string() },
            Message { role: "user".to_string(), content: prompt.to_string() },
        ],
    };

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    
    let mut builder = client.post(&url);
    if !api_key.is_empty() {
        builder = builder.header("Authorization", format!("Bearer {}", api_key));
    }

    let res = builder.json(&req_body).send().await?;

    let chat_res: ChatResponse = res.json().await?;
    Ok(chat_res.choices[0].message.content.clone())
}
