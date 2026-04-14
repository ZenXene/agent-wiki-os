use assert_cmd::Command;
use serde_json::json;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command as StdCommand, Stdio};

#[test]
fn test_e2e_ingest_wiki_mock_writes_source() {
    let cwd = std::env::current_dir().unwrap();
    let temp_home_dir = tempfile::Builder::new()
        .prefix("awo-home-")
        .tempdir_in(&cwd)
        .unwrap();
    let temp_work_dir = tempfile::Builder::new()
        .prefix("awo-work-")
        .tempdir_in(&cwd)
        .unwrap();

    let src_dir = temp_work_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() { println!(\"Hello\"); }\n").unwrap();

    Command::cargo_bin("agent-wiki-os")
        .unwrap()
        .env("HOME", temp_home_dir.path())
        .env("WIKI_LLM_ENABLE", "1")
        .env("WIKI_MOCK", "1")
        .env("WIKI_DISABLE_VECTOR_DB", "1")
        .current_dir(temp_work_dir.path())
        .arg("ingest")
        .arg(&src_dir)
        .arg("--mode")
        .arg("wiki")
        .assert()
        .success();

    let expected = temp_work_dir
        .path()
        .join(".wiki")
        .join("sources")
        .join("Mock_Context.md");
    assert!(expected.exists());
}

#[test]
fn test_e2e_mcp_stdio_tools_list_save_and_read() {
    let cwd = std::env::current_dir().unwrap();
    let temp_home_dir = tempfile::Builder::new()
        .prefix("awo-home-")
        .tempdir_in(&cwd)
        .unwrap();
    let temp_work_dir = tempfile::Builder::new()
        .prefix("awo-work-")
        .tempdir_in(&cwd)
        .unwrap();

    fs::create_dir_all(temp_work_dir.path().join(".wiki")).unwrap();

    let bin = assert_cmd::cargo::cargo_bin("agent-wiki-os");
    let mut child = StdCommand::new(bin)
        .env("HOME", temp_home_dir.path())
        .env("WIKI_DISABLE_VECTOR_DB", "1")
        .current_dir(temp_work_dir.path())
        .arg("mcp")
        .arg("--mode")
        .arg("stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let req1 = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    writeln!(stdin, "{}", req1.to_string()).unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let resp1: serde_json::Value = serde_json::from_str(&line).unwrap();
    let tools = resp1["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "save_to_wiki"));
    assert!(tools.iter().any(|t| t["name"] == "read_wiki_page"));

    let req2 = json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"tools/call",
        "params":{
            "name":"save_to_wiki",
            "arguments":{
                "title":"TestDoc",
                "content":"hello from mcp",
                "page_type":"concept"
            }
        }
    });
    writeln!(stdin, "{}", req2.to_string()).unwrap();
    stdin.flush().unwrap();

    line.clear();
    reader.read_line(&mut line).unwrap();
    let resp2: serde_json::Value = serde_json::from_str(&line).unwrap();
    let saved_text = resp2["result"]["content"][0]["text"].as_str().unwrap();
    assert!(saved_text.contains("Successfully saved to:"));

    let req3 = json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"tools/call",
        "params":{
            "name":"read_wiki_page",
            "arguments":{
                "path":"concepts/TestDoc.md"
            }
        }
    });
    writeln!(stdin, "{}", req3.to_string()).unwrap();
    stdin.flush().unwrap();

    line.clear();
    reader.read_line(&mut line).unwrap();
    let resp3: serde_json::Value = serde_json::from_str(&line).unwrap();
    let read_text = resp3["result"]["content"][0]["text"].as_str().unwrap();
    assert!(read_text.contains("hello from mcp"));

    drop(stdin);
    let _ = child.wait();
}
