use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_shared_cave, SharedCave};
use crate::cave::CaveError;

// ── read_note ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ReadNoteArgs {
    /// The slug (filename without .md extension) of the note to read.
    /// If omitted, reads the note currently open in the editor.
    slug: Option<String>,
}

#[derive(Serialize)]
pub struct ReadNoteOutput {
    slug: String,
    relative_path: String,
    content: String,
    backlinks: Vec<String>,
}

pub struct ReadNoteTool {
    pub cave: SharedCave,
}

impl Tool for ReadNoteTool {
    const NAME: &'static str = "read_note";
    type Error = CaveError;
    type Args = ReadNoteArgs;
    type Output = ReadNoteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "read_note".to_string(),
            description: "Read the body of a note (markdown without frontmatter) and the slugs of notes linking to it. Pass a slug (filename without .md) to read a specific note, or omit it to read the note currently open in the editor."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "slug": {
                        "type": "string",
                        "description": "The slug (filename without .md) of the note to read. Omit to read the active note."
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let slug = match &args.slug {
                Some(s) => cave.resolve_slug(s)?.to_string(),
                None => cave
                    .active_slug()
                    .ok_or_else(|| {
                        crate::cave::CaveError::NotFound(
                            "no note is currently open in the editor".to_string(),
                        )
                    })?
                    .to_string(),
            };
            let note = cave.read_note(&slug)?;
            let backlinks = cave.backlink_slugs(&slug)?;
            Ok(ReadNoteOutput {
                slug: note.meta.slug,
                relative_path: note.meta.relative_path,
                content: note.content,
                backlinks,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn shared_cave(cave: crate::cave::Cave) -> SharedCave {
        Arc::new(Mutex::new(Some(cave)))
    }

    #[tokio::test]
    async fn read_note_tool_returns_backlinks_for_explicit_slug() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "[[target]]\n").unwrap();

        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let tool = ReadNoteTool {
            cave: shared_cave(cave),
        };

        let output = tool
            .call(ReadNoteArgs {
                slug: Some("target".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(output.slug, "target");
        assert_eq!(output.backlinks, vec!["source".to_string()]);
    }

    #[tokio::test]
    async fn read_note_tool_uses_active_note_when_slug_omitted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        std::fs::write(dir.path().join("source.md"), "[[target]]\n").unwrap();

        let mut cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        cave.set_active_slug(Some("target".to_string()));
        let tool = ReadNoteTool {
            cave: shared_cave(cave),
        };

        let output = tool.call(ReadNoteArgs { slug: None }).await.unwrap();

        assert_eq!(output.slug, "target");
        assert_eq!(output.backlinks, vec!["source".to_string()]);
    }
}
