# Agent-Wiki-OS

A unified knowledge refinement and graph management engine designed to solve the memory fragmentation problem across different AI IDEs (Cursor, Trae) and CLIs (Claude Code, Codex, Gemini, Qwen).

## The Core Philosophy

This project embraces the core philosophy of [Karpathy's LLM Wiki pattern](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) and the `llm_wiki` project:
- **Shift from RAG to Incremental Compilation**: LLMs act as background "librarians", digesting new knowledge into a persistent, structured knowledge network instead of retrieving from scratch on every query.
- **Bidirectional Knowledge Evolution**: Active linking between Entities and Concepts.
- **Clear Human-AI Division**: Humans provide high-quality sources and macro guidance; LLMs handle reading, summarizing, cross-referencing, and state maintenance.

## Architecture

Agent-Wiki-OS takes a **Hybrid Toolchain** approach: a Rust-based CLI tool coupled with an MCP Server.

1. **Adapters (Ingestion Pipelines)**: Pulls data from local directories (`FsAdapter`) or external agent histories (`HistoryAdapter` supporting `claude-cli`, `gemini-cli`, `codex-cli`, etc.).
2. **Refinement Engine (2-Step Ingest)**: Uses an LLM (configurable via environment variables) to extract architecture decisions, code history, entities, and concepts from the raw text.
3. **Graph Engine**: Writes the structured YAML/Markdown output into a local (`./.wiki/`) or global (`~/.agent-wiki-os/`) storage directory, categorized by `entities/`, `concepts/`, and `sources/`.
4. **MCP Server**: Allows active agents to read from or write to the Wiki in real-time.

## Installation

### 1. Install via Script (macOS / Linux)
```bash
curl -sSfL https://raw.githubusercontent.com/ZenXene/agent-wiki-os/main/install.sh | sh
```

### 2. Build from Source
You need [Rust](https://rustup.rs/) installed.

```bash
git clone <repository_url>
cd agent-wiki-os
cargo build --release
```

## Usage

### 1. Ingest Local Files
Recursively read a directory (e.g., your project's `_inbox` or `src` folder) and compile the knowledge into the Wiki:
```bash
export WIKI_API_KEY="sk-..."
export WIKI_MOCK=0 # Set to 1 to mock LLM responses
cargo run -- ingest --dir ./src
```

### 2. Pull Agent History
Extract conversation history from a supported Agent CLI and compile it into the Wiki:
```bash
cargo run -- pull trae
```
*Supported Agents*: `cursor`, `trae`, `trae-cn`, `claude-cli`, `gemini-cli`, `codex-cli`, `openclaw`, `opencode`.

### 3. Run the MCP Server
Start the Model Context Protocol server via standard I/O (to be attached to Trae/Cursor):
```bash
cargo run -- mcp --mode stdio
```

### 4. Run the Background Daemon
Agent-Wiki-OS can run in the background to automatically ingest history.
```bash
cargo run -- daemon
```
This is controlled by `~/.agent-wiki-os/config.toml`:
```toml
[daemon]
# mode can be "polling" (time-based) or "watcher" (event-driven file system watcher)
mode = "watcher"
interval_seconds = 3600

[agents]
enabled = ["trae", "cursor", "claude-cli"]
```

## Configuration

The LLM engine is configured via environment variables:
- `WIKI_API_KEY`: Your LLM provider's API key.
- `WIKI_BASE_URL`: (Optional) Custom endpoint, defaults to `https://api.openai.com/v1`. Useful for vLLM, Ollama, or proxy services.
- `WIKI_MODEL`: (Optional) Model name, defaults to `gpt-3.5-turbo`.
- `WIKI_MOCK`: Set to `1` to bypass network calls and generate mock Markdown files. Defaults to `1` if no API key is provided and the URL is not localhost.

## Roadmap
- [x] Basic CLI Router and Storage Resolution
- [x] File System Adapter (`walkdir`)
- [x] Multi-Agent History Adapter (`serde_json`, `dirs`)
- [x] LLM Refinement Engine (`reqwest`)
- [x] Graph Engine Markdown Persistence
- [ ] SQLite Parser for Cursor / Trae local history
- [ ] Complete MCP Tools (`search_wiki`, `read_wiki_page`, `save_to_wiki`)
- [ ] Web Clipper URL Ingestion Adapter