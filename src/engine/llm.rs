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
    // Read API key from env for mock purpose, fallback to empty
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    
    // If no key is provided, just return a mock response for now to avoid failing
    if api_key.is_empty() {
        return Ok(format!("[MOCK LLM RESPONSE] Processed: {:.30}...", prompt));
    }

    let client = reqwest::Client::new();
    let req_body = ChatRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            Message { role: "system".to_string(), content: "You are a Wiki refinement engine.".to_string() },
            Message { role: "user".to_string(), content: prompt.to_string() },
        ],
    };

    let res = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&req_body)
        .send()
        .await?;

    let chat_res: ChatResponse = res.json().await?;
    Ok(chat_res.choices[0].message.content.clone())
}
