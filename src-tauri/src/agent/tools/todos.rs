use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_shared_cave, SharedCave};
use crate::cave::CaveError;

// ── list_todos ─────────────────────────────────────────────────────

/// Optional slug filter for the list_todos tool.
#[derive(Deserialize)]
pub struct ListTodosArgs {
    /// If provided, only return todos from this note slug.
    slug: Option<String>,
}

#[derive(Serialize)]
pub struct ListTodosOutput {
    incomplete: Vec<TodoEntry>,
    completed: Vec<TodoEntry>,
}

#[derive(Serialize)]
struct TodoEntry {
    slug: String,
    relative_path: String,
    line: usize,
    text: String,
}

pub struct ListTodosTool {
    pub cave: SharedCave,
}

impl Tool for ListTodosTool {
    const NAME: &'static str = "list_todos";
    type Error = CaveError;
    type Args = ListTodosArgs;
    type Output = ListTodosOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_todos".to_string(),
            description: "List todo items (checkboxes) from notes in the cave, pre-split into incomplete and completed. Optionally filter by note slug.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "Only return todos from this note slug. Omit to list todos from all notes."
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let list = cave.list_todos()?;
            let to_entry = |t: granit_types::TodoItem| TodoEntry {
                slug: t.slug,
                relative_path: t.relative_path,
                line: t.line,
                text: t.text,
            };
            let filter =
                |t: &granit_types::TodoItem| args.slug.as_ref().is_none_or(|s| &t.slug == s);
            Ok(ListTodosOutput {
                incomplete: list
                    .incomplete
                    .into_iter()
                    .filter(filter)
                    .map(to_entry)
                    .collect(),
                completed: list
                    .completed
                    .into_iter()
                    .filter(filter)
                    .map(to_entry)
                    .collect(),
            })
        })
    }
}

// ── toggle_todo ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ToggleTodoArgs {
    /// The note slug containing the todo.
    slug: String,
    /// The 1-based line number of the todo checkbox in the raw file.
    line: usize,
}

#[derive(Serialize)]
pub struct ToggleTodoOutput {
    message: String,
}

pub struct ToggleTodoTool {
    pub cave: SharedCave,
}

impl Tool for ToggleTodoTool {
    const NAME: &'static str = "toggle_todo";
    type Error = CaveError;
    type Args = ToggleTodoArgs;
    type Output = ToggleTodoOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "toggle_todo".to_string(),
            description: "Toggle the completion status of a todo checkbox in a note. Use list_todos to find the slug and line number.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug of the note containing the todo"
                    },
                    "line": {
                        "type": "integer",
                        "description": "The 1-based line number of the todo checkbox in the note file"
                    }
                },
                "required": ["slug", "line"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            cave.toggle_todo(&args.slug, args.line)?;
            Ok(ToggleTodoOutput {
                message: format!("Toggled todo on line {} in '{}'", args.line, args.slug),
            })
        })
    }
}
