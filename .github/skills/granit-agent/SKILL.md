---
name: granit-agent
description: "Build and extend the rig-core AI agent. Use when: adding agent tools, configuring LLM providers, setting up vector DB for RAG, implementing chat streaming, or modifying agent behavior."
---

# rig-core Agent

Build the AI agent for Granit using `rig-core`. The agent lives entirely in the backend (`src-tauri/src/agent/`).

## Architecture

```
src-tauri/src/agent/
  mod.rs        — Agent builder, provider config
  tools/        — CRUD tools for cave operations
  vectordb.rs   — In-memory vector DB for RAG
```

## Provider Configuration

`rig-core` supports multiple providers. Make the provider configurable (e.g., via a config file or environment variable):

```rust
use rig::providers::{openai, anthropic};

// Example: OpenAI
let client = openai::Client::new(&api_key);
let model = client.agent("gpt-4o").build();

// Example: Anthropic
let client = anthropic::Client::new(&api_key);
let model = client.agent("claude-sonnet-4-20250514").build();
```

## Tool Pattern

Each agent tool implements the `rig::tool::Tool` trait. Keep tools focused — one operation per tool:

```rust
use rig::tool::Tool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ReadNoteArgs {
    filename: String,
}

#[derive(Serialize)]
struct ReadNoteOutput {
    content: String,
    frontmatter: Option<Frontmatter>,
}

#[derive(thiserror::Error, Debug)]
#[error("Tool error: {0}")]
struct ToolError(String);

struct ReadNoteTool {
    cave_path: PathBuf,
}

impl Tool for ReadNoteTool {
    const NAME: &'static str = "read_note";
    type Args = ReadNoteArgs;
    type Output = ReadNoteOutput;
    type Error = ToolError;

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Read from cave, return content + metadata
    }
}
```

### Standard Cave Tools

Build these tools for the agent:
- **`read_note`** — Read a note's content and frontmatter by filename
- **`create_note`** — Create a new `.md` file in the cave
- **`update_note`** — Overwrite a note's content
- **`delete_note`** — Remove a note from the cave
- **`list_notes`** — List all notes (optionally in a subfolder)
- **`search_notes`** — Search notes by filename/title

## RAG with Vector DB

Use `rig-core`'s in-memory vector store for semantic search over cave contents:

1. On cave open: embed all note contents and store in the vector DB
2. On agent query: retrieve top-k relevant notes as context
3. On note save: re-embed the updated note

```rust
use rig::vector_store::in_memory_store::InMemoryVectorStore;

let mut store = InMemoryVectorStore::default();
// Add documents with embeddings
// Query with similarity search
```

## Frontend Integration

The frontend renders a side panel chat UI. Communication is via Tauri IPC:

- **`send_message`** command — Takes user message, returns agent response
- For streaming: use Tauri events (`app.emit(...)`) to push partial responses to the frontend

Agent state (conversation history, vector DB) lives in `tauri::State<AgentState>`.
