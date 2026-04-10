use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: u64,
}

pub async fn run_stdio_server() -> anyhow::Result<()> {
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if let Ok(req) = serde_json::from_str::<McpRequest>(&line) {
            println!(r#"{{"jsonrpc": "2.0", "id": {}, "result": "mock_result"}}"#, req.id);
        }
    }
    Ok(())
}