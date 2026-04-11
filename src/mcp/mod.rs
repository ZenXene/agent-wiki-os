use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use crate::storage::WikiStorage;
use std::path::PathBuf;
use crate::engine::graph::GraphEngine;

#[derive(Serialize, Deserialize, Debug)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

pub async fn run_stdio_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    // Initialize storage to get wiki root paths
    let storage = WikiStorage::init()?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Determine the active wiki root (prefer local project .wiki, fallback to global)
    let local_wiki = cwd.join(".wiki");
    let wiki_root = if local_wiki.exists() {
        local_wiki
    } else {
        storage.global_path.clone()
    };
    
    let graph = GraphEngine::new(&wiki_root);

    // Log startup to stderr (so it doesn't pollute stdout JSON-RPC)
    eprintln!("[MCP] Agent-Wiki-OS MCP Server started. Wiki root: {}", wiki_root.display());

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        
        match serde_json::from_str::<McpRequest>(&line) {
            Ok(req) => {
                let response = handle_request(req, &graph, &wiki_root).await;
                let response_str = serde_json::to_string(&response)?;
                println!("{}", response_str);
                stdout.flush()?;
            }
            Err(e) => {
                eprintln!("[MCP] Failed to parse request: {}. Line: {}", e, line);
            }
        }
    }
    Ok(())
}

async fn handle_request(req: McpRequest, graph: &GraphEngine, wiki_root: &PathBuf) -> McpResponse {
    match req.method.as_str() {
        "initialize" => {
            // MCP Protocol initialization
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                error: None,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "agent-wiki-os",
                        "version": "0.1.13"
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }))
            }
        },
        "tools/list" => {
            // Register available tools
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                error: None,
                result: Some(json!({
                    "tools": [
                        {
                            "name": "search_wiki",
                            "description": "Search the local Agent-Wiki-OS knowledge base for concepts, skills, personas, or working memory context.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": {
                                        "type": "string",
                                        "description": "The search keyword or concept to look for"
                                    },
                                    "type_filter": {
                                        "type": "string",
                                        "description": "Optional. Filter by type: 'concept', 'entity', 'skill', 'persona', 'postmortem', 'onboard', 'source'",
                                        "enum": ["concept", "entity", "skill", "persona", "postmortem", "onboard", "source"]
                                    }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "read_wiki_page",
                            "description": "Read the full content of a specific markdown page from the Agent-Wiki-OS knowledge base.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": {
                                        "type": "string",
                                        "description": "The relative or absolute path to the wiki markdown file"
                                    }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "save_to_wiki",
                            "description": "Save generated knowledge, postmortems, or skills directly into the Agent-Wiki-OS knowledge base.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "title": {
                                        "type": "string",
                                        "description": "The title of the document (will be used as filename)"
                                    },
                                    "content": {
                                        "type": "string",
                                        "description": "The full markdown content to save"
                                    },
                                    "page_type": {
                                        "type": "string",
                                        "description": "The type of page (determines folder): 'concept', 'entity', 'skill', 'persona', 'postmortem', 'spec', 'onboard', 'source'",
                                        "enum": ["concept", "entity", "skill", "persona", "postmortem", "spec", "onboard", "source"]
                                    }
                                },
                                "required": ["title", "content", "page_type"]
                            }
                        },
                        {
                            "name": "run_ingest",
                            "description": "Trigger the Agent-Wiki-OS ingest process for a file, folder, or URL. Use this to automatically generate concepts, skills, specs, or onboards from external sources.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "target": {
                                        "type": "string",
                                        "description": "The file path, directory path, or URL to ingest"
                                    },
                                    "mode": {
                                        "type": "string",
                                        "description": "The processing mode: 'wiki', 'skill', 'persona', 'postmortem', 'spec', 'onboard'",
                                        "enum": ["wiki", "skill", "persona", "postmortem", "spec", "onboard"]
                                    },
                                    "output": {
                                        "type": "string",
                                        "description": "Optional explicit output file path"
                                    }
                                },
                                "required": ["target", "mode"]
                            }
                        }
                    ]
                }))
            }
        },
        "tools/call" => {
            // Handle tool execution
            let params = req.params.unwrap_or(json!({}));
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").unwrap_or(&json!({}));
            
            let result_content = match name {
                "search_wiki" => {
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                    let type_filter = args.get("type_filter").and_then(|v| v.as_str());
                    
                    // Simple grep-like search implementation for MVP
                    // In a real app, this would use tantivy or another search engine
                    let search_results = perform_simple_search(wiki_root, query, type_filter);
                    json!([{ "type": "text", "text": search_results }])
                },
                "read_wiki_page" => {
                    let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let path = std::path::Path::new(path_str);
                    
                    // Security check to prevent reading outside wiki root
                    let safe_path = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        wiki_root.join(path)
                    };
                    
                    match std::fs::read_to_string(&safe_path) {
                        Ok(content) => json!([{ "type": "text", "text": content }]),
                        Err(e) => json!([{ "type": "text", "text": format!("Error reading file: {}", e), "isError": true }])
                    }
                },
                "save_to_wiki" => {
                    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let page_type = args.get("page_type").and_then(|v| v.as_str()).unwrap_or("concept");
                    
                    match graph.write_page(page_type, title, content) {
                        Ok(path) => json!([{ "type": "text", "text": format!("Successfully saved to: {}", path.display()) }]),
                        Err(e) => json!([{ "type": "text", "text": format!("Failed to save: {}", e), "isError": true }])
                    }
                },
                "run_ingest" => {
                    let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("");
                    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("wiki");
                    let output = args.get("output").and_then(|v| v.as_str()).map(|s| s.to_string());
                    
                    // Simple routing based on target
                    let process_mode = crate::engine::ingest::ProcessMode::from_str(mode);
                    let mut result_text = String::new();
                    
                    if target.starts_with("http://") || target.starts_with("https://") {
                        let web_adapter = crate::adapters::WebAdapter::new(target);
                        match web_adapter.fetch().await {
                            Ok(content) => {
                                match crate::engine::ingest::RefinementEngine::process(&content, wiki_root, "web_clipper", process_mode, output).await {
                                    Ok(path) => result_text = format!("Ingestion complete. Artifact path or task file path: {}", path),
                                    Err(e) => result_text = format!("Error processing URL: {}", e),
                                }
                            }
                            Err(e) => result_text = format!("Failed to fetch URL: {}", e),
                        }
                    } else {
                        let fs_adapter = crate::adapters::FsAdapter::new(target);
                        if let Ok(files_content) = fs_adapter.fetch_all() {
                            let mut paths = Vec::new();
                            for content in files_content {
                                match crate::engine::ingest::RefinementEngine::process(&content, wiki_root, "local_fs", process_mode, output.clone()).await {
                                    Ok(path) => {
                                        if !path.is_empty() {
                                            paths.push(path);
                                        }
                                    },
                                    Err(e) => result_text.push_str(&format!("Error processing file: {}\n", e)),
                                }
                            }
                            if !paths.is_empty() {
                                result_text.push_str(&format!("Ingestion complete. Generated files: \n{}", paths.join("\n")));
                            } else if result_text.is_empty() {
                                result_text = "Ingestion complete. No files generated.".to_string();
                            }
                        } else {
                            result_text = format!("Failed to read target path: {}", target);
                        }
                    }
                    
                    json!([{ "type": "text", "text": result_text }])
                },
                _ => {
                    json!([{ "type": "text", "text": format!("Unknown tool: {}", name), "isError": true }])
                }
            };
            
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                error: None,
                result: Some(json!({
                    "content": result_content
                }))
            }
        },
        _ => {
            // Handle unknown methods
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(json!({
                    "code": -32601,
                    "message": "Method not found"
                }))
            }
        }
    }
}

// Helper function for simple file searching
fn perform_simple_search(wiki_root: &PathBuf, query: &str, type_filter: Option<&str>) -> String {
    use walkdir::WalkDir;
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();
    
    // Determine which directory to search based on filter
    let search_dir = match type_filter {
        Some("concept") => wiki_root.join("concepts"),
        Some("entity") => wiki_root.join("entities"),
        Some("skill") => wiki_root.join("skills"),
        Some("persona") => wiki_root.join("personas"),
        Some("postmortem") => wiki_root.join("postmortems"),
        Some("onboard") => wiki_root.join("onboards"),
        Some("spec") => wiki_root.join("specs"),
        Some("source") => wiki_root.join("sources"),
        _ => wiki_root.clone(), // Search all
    };
    
    if !search_dir.exists() {
        return format!("Directory {} does not exist yet.", search_dir.display());
    }
    
    for entry in WalkDir::new(search_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md" || ext == "skill") {
            // Search in filename
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            if file_name.contains(&query_lower) {
                results.push(format!("File Match: {}", path.display()));
                continue;
            }
            
            // Search in content (first few matches only to save space)
            if let Ok(content) = std::fs::read_to_string(path) {
                if content.to_lowercase().contains(&query_lower) {
                    // Extract a snippet
                    if let Some(idx) = content.to_lowercase().find(&query_lower) {
                        let start = idx.saturating_sub(40);
                        let end = (idx + query.len() + 40).min(content.len());
                        let snippet = &content[start..end].replace('\n', " ");
                        results.push(format!("Content Match in [{}]: ...{}...", path.display(), snippet));
                    }
                }
            }
        }
    }
    
    if results.is_empty() {
        format!("No results found for '{}'", query)
    } else {
        // Limit results to avoid massive context windows
        let count = results.len();
        let display_results: Vec<String> = results.into_iter().take(15).collect();
        let mut out = display_results.join("\n\n");
        if count > 15 {
            out.push_str(&format!("\n\n... and {} more results hidden.", count - 15));
        }
        out
    }
}