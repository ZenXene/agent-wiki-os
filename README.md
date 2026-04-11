# Agent-Wiki-OS

A unified knowledge refinement and graph management engine designed to solve the memory fragmentation problem across different AI IDEs (Cursor, Trae) and CLIs (Claude Code, Codex, Gemini, Qwen).

## The Core Philosophy

This project embraces the core philosophy of [Karpathy's LLM Wiki pattern](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) and the `llm_wiki` project:
- **Shift from RAG to Incremental Compilation**: LLMs act as background "librarians", digesting new knowledge into a persistent, structured knowledge network instead of retrieving from scratch on every query.
- **Bidirectional Knowledge Evolution**: Active linking between Entities and Concepts.
- **Clear Human-AI Division**: Humans provide high-quality sources and macro guidance; LLMs handle reading, summarizing, cross-referencing, and state maintenance.

## Key Features

- **"Free" Compute via TaskFile v1**: By default (`llm.enable=false`), AWO delegates the heavy lifting of knowledge refinement to the LLMs already built into your IDE (Trae/Cursor) via a zero-cost `.awo_tasks` file bus.
- **True Cross-Tool Memory**: Runs as a background daemon monitoring the raw SQLite databases of Trae and Cursor, as well as CLI JSONL histories.
- **Human-Readable Whitebox Memory**: Outputs standard Markdown files (`.wiki/concepts`, `.wiki/skills`, `.wiki/onboards`) that can be Git-versioned and reviewed.
- **Context Protection**: Smart chunking, Base64 image stripping, and automatic truncation for massive GitHub repositories to prevent LLM context explosion.
- **Robust MCP Server**: Provides precise `search_wiki` (BM25-style summaries), secure `read_wiki_page` (path traversal protection), and `run_ingest` tools for IDEs.

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
*(This will install `agent-wiki-os` and its shorter alias `awo` to your system)*

### 2. Update
To update to the latest version, simply re-run the installation script:
```bash
curl -sSfL https://raw.githubusercontent.com/ZenXene/agent-wiki-os/main/install.sh | sh
```

### 3. Uninstall
To completely remove Agent-Wiki-OS and its global configuration:
```bash
# Remove the binary and alias
sudo rm /usr/local/bin/agent-wiki-os
sudo rm /usr/local/bin/awo

# Remove the global configuration and global wiki data
rm -rf ~/.agent-wiki-os

# Note: Local project wikis (./.wiki/) will remain in their respective project folders.
```

### 4. Build from Source
You need [Rust](https://rustup.rs/) installed.

```bash
git clone <repository_url>
cd agent-wiki-os
cargo build --release
```

## Usage

### 1. Ingest Local Files or URLs
Recursively read a directory (e.g., your project's `_inbox` or `src` folder) or fetch a web page and compile the knowledge into the Wiki:
```bash
awo ingest ./src --mode spec
awo ingest https://github.com/ZenXene/agent-wiki-os --mode persona
awo ingest ./my_design.pdf --mode skill
```

### 2. Pull Agent History
Extract conversation history from a supported Agent CLI and compile it into the Wiki:
```bash
awo pull trae
```
*Supported Agents*: `cursor`, `trae`, `trae-cn`, `claude-cli`, `gemini-cli`, `codex-cli`, `openclaw`, `opencode`.

### 3. Install IDE Skill
Generate and link the `agent-wiki-os` Master Skill to your favorite IDEs (Trae, Trae-CN, Cursor) so their internal LLMs know how to use the AWO TaskFile protocol.
```bash
awo skills install all
```

### 4. Run the MCP Server
Start the Model Context Protocol server via standard I/O (to be attached to Trae/Cursor as an MCP server):
```bash
awo mcp --mode stdio
```

### 5. Run the Background Daemon
Agent-Wiki-OS can run in the background to automatically ingest history.
```bash
awo daemon
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

The application is configured primarily through `~/.agent-wiki-os/config.toml` (which is created automatically upon first run). You can edit this file directly or use the CLI `config` commands.

```bash
# Set LLM options (if you want AWO to run LLMs directly instead of using IDE)
awo config set llm.enable true
awo config set llm.model "claude-3-7-sonnet-20250219"
awo config set llm.api_key "sk-ant-..."
awo config set llm.base_url "https://api.anthropic.com/v1"
```

The daemon can also be configured in `config.toml` to watch custom directories in addition to IDE histories:
```toml
[daemon]
mode = "watcher"
interval_seconds = 3600
custom_watch_dirs = [
    "/Users/username/Projects/my_project/_inbox"
]
```

Alternatively, environment variables still serve as overrides for the LLM settings:
- `WIKI_API_KEY`
- `WIKI_BASE_URL`
- `WIKI_MODEL`
- `WIKI_MOCK`
- `WIKI_LLM_ENABLE`

## Roadmap
- [x] Basic CLI Router and Storage Resolution
- [x] File System Adapter (`walkdir`)
- [x] Multi-Agent History Adapter (`serde_json`, `dirs`)
- [x] LLM Refinement Engine (`reqwest`)
- [x] Graph Engine Markdown Persistence
- [x] SQLite Parser for Cursor / Trae local history
- [x] Complete MCP Tools (`search_wiki`, `read_wiki_page`, `save_to_wiki`)
- [x] Web Clipper URL Ingestion Adapter