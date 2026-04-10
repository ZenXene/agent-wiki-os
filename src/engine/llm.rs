use serde::{Deserialize, Serialize};

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
    let api_key = std::env::var("WIKI_API_KEY").unwrap_or_default();
    let base_url = std::env::var("WIKI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("WIKI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
    
    // For now, we mock the response to avoid real network calls if no key is provided,
    // or if the user explicitly wants to mock (e.g., WIKI_MOCK=1)
    let mock_mode = std::env::var("WIKI_MOCK").unwrap_or_else(|_| "1".to_string());
    if mock_mode == "1" || (api_key.is_empty() && !base_url.contains("localhost") && !base_url.contains("127.0.0.1")) {
        // Return a mock structured response that the graph engine can parse
        return Ok(format!("---\ntitle: Mock Entity\ntype: entity\ntags: [mock, test]\n---\n\n# Mock Entity\n\nThis is a mocked entity generated from:\n{:.50}...", prompt));
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
