---
applyTo: "src-tauri/**"
---

# Tauri Backend

Instructions for working in the Tauri 2 backend (`src-tauri/`).

## Command Pattern

Every Tauri command follows this structure:

```rust
#[tauri::command]
fn command_name(arg: ArgType) -> Result<ReturnType, MyError> {
    // Delegate to a module function — keep the handler thin
    module::do_work(arg)
}
```

Register in the builder:
```rust
.invoke_handler(tauri::generate_handler![command_name])
```

## Error Handling

Define typed errors with `thiserror` per module. Implement `serde::Serialize` so Tauri can return them to the frontend:

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum CaveError {
    #[error("Cave not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(String),
}

// Convert std::io::Error → CaveError
impl From<std::io::Error> for CaveError {
    fn from(e: std::io::Error) -> Self {
        CaveError::Io(e.to_string())
    }
}
```

## State Management

Use `tauri::State<T>` for shared backend state (e.g., current cave, agent config). Wrap mutable state in `Mutex` or `RwLock`:

```rust
struct AppState {
    cave: RwLock<Option<Cave>>,
}
```

## Testing

Every new module should include a `#[cfg(test)]` block. Test the module functions directly — not through the Tauri command layer:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Use tempdir for file system tests
    }
}
```

## Module Organization

Keep `lib.rs` thin — it only wires up commands and plugins. Domain logic goes in dedicated modules:

```
src-tauri/src/
  lib.rs        — Builder, command registration
  main.rs       — Entry point
  cave/         — Cave operations (open, list, read, write)
  markdown/     — pulldown-cmark parsing, frontmatter, wiki-links
  agent/        — rig-core agent, tools, vector DB
```
