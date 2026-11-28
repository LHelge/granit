use rig_core::tool::PortableTool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{with_shared_cave, SharedCave};
use crate::cave::CaveError;

// ── use_skill ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UseSkillArgs {
    /// The name of the skill to load, as listed in the system prompt.
    name: String,
}

#[derive(Debug, Serialize)]
pub struct UseSkillOutput {
    name: String,
    instructions: String,
}

pub struct UseSkillTool {
    pub cave: SharedCave,
}

impl PortableTool for UseSkillTool {
    const NAME: &'static str = "use_skill";
    type Error = CaveError;
    type Args = UseSkillArgs;
    type Output = UseSkillOutput;

    fn description(&self) -> String {
        "Load the full instructions of a skill by name. The available skills and their descriptions are listed in the system prompt; call this before performing a task a skill covers, then follow the returned instructions."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name of the skill to load."
                }
            },
            "required": ["name"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let instructions = cave.skill_body(&args.name)?;
            Ok(UseSkillOutput {
                name: args.name,
                instructions,
            })
        })
    }
}

// ── read_agent_doc / write_agent_doc ───────────────────────────────
//
// Let the agent inspect and improve its own skills and tasks. Reading is
// available in both modes; writing is a mutating tool (Agent mode only).
// A skill edit cannot reset the running agent (that would wipe the
// conversation), so the system prompt's skill listing refreshes on the
// next rebuild — the file content itself is always read fresh.

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentDocKind {
    Skill,
    Task,
}

#[derive(Deserialize)]
pub struct ReadAgentDocArgs {
    /// Whether to read a skill or a task.
    kind: AgentDocKind,
    /// The name of the skill or task.
    name: String,
}

#[derive(Debug, Serialize)]
pub struct AgentDocOutput {
    name: String,
    description: String,
    body: String,
}

pub struct ReadAgentDocTool {
    pub cave: SharedCave,
}

impl PortableTool for ReadAgentDocTool {
    const NAME: &'static str = "read_agent_doc";
    type Error = CaveError;
    type Args = ReadAgentDocArgs;
    type Output = AgentDocOutput;

    fn description(&self) -> String {
        "Read one of your own skills or slash-command tasks for review or improvement: returns its description and body. Skills are reusable instructions loaded via use_skill; tasks are Tera prompt templates the user invokes with /name in the chat."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["skill", "task"],
                    "description": "Whether to read a skill or a task."
                },
                "name": {
                    "type": "string",
                    "description": "The name of the skill or task."
                }
            },
            "required": ["kind", "name"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            let doc = match args.kind {
                AgentDocKind::Skill => cave.read_skill(&args.name)?,
                AgentDocKind::Task => cave.read_task(&args.name)?,
            };
            Ok(AgentDocOutput {
                name: doc.meta.slug,
                description: doc.meta.description.unwrap_or_default(),
                body: doc.content,
            })
        })
    }
}

#[derive(Deserialize)]
pub struct WriteAgentDocArgs {
    /// Whether to write a skill or a task.
    kind: AgentDocKind,
    /// The name of the skill or task; created if it does not exist yet.
    name: String,
    /// The new body. Omit to keep the current body.
    body: Option<String>,
    /// The new description. Omit to keep the current description.
    description: Option<String>,
}

pub struct WriteAgentDocTool {
    pub cave: SharedCave,
}

impl PortableTool for WriteAgentDocTool {
    const NAME: &'static str = "write_agent_doc";
    type Error = CaveError;
    type Args = WriteAgentDocArgs;
    type Output = AgentDocOutput;

    fn description(&self) -> String {
        "Create or update one of your own skills or slash-command tasks. The body replaces the whole body, so call read_agent_doc first when editing. Skill names must be kebab-case (lowercase letters, digits, hyphens). A skill's description should say what it does and when to use it; a task body is a Tera template with access to {{ input }}, {{ active_note }}, and date variables."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["skill", "task"],
                    "description": "Whether to write a skill or a task."
                },
                "name": {
                    "type": "string",
                    "description": "The name of the skill or task; created if it does not exist yet."
                },
                "body": {
                    "type": "string",
                    "description": "The new body (replaces the whole body). Omit to keep the current one."
                },
                "description": {
                    "type": "string",
                    "description": "The new description. Omit to keep the current one."
                }
            },
            "required": ["kind", "name"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        with_shared_cave(&self.cave, |cave| {
            // Upsert: read the existing document, creating it when absent.
            let existing = match args.kind {
                AgentDocKind::Skill => cave.read_skill(&args.name),
                AgentDocKind::Task => cave.read_task(&args.name),
            };
            let existing = match existing {
                Ok(doc) => doc,
                Err(CaveError::SkillNotFound(_)) => {
                    cave.create_skill(&args.name)?;
                    cave.read_skill(&args.name)?
                }
                Err(CaveError::TaskNotFound(_)) => {
                    cave.create_task(&args.name)?;
                    cave.read_task(&args.name)?
                }
                Err(e) => return Err(e),
            };

            let body = args.body.unwrap_or(existing.content);
            let description = args
                .description
                .or(existing.meta.description)
                .unwrap_or_default();
            let meta = match args.kind {
                AgentDocKind::Skill => {
                    cave.update_skill(&args.name, &args.name, &body, &description)?
                }
                AgentDocKind::Task => {
                    cave.update_task(&args.name, &args.name, &body, &description)?
                }
            };
            Ok(AgentDocOutput {
                name: meta.slug,
                description,
                body,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    fn shared_cave(cave: crate::cave::Cave) -> SharedCave {
        Arc::new(Mutex::new(Some(cave)))
    }

    #[tokio::test]
    async fn use_skill_tool_returns_instructions_without_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join(".granit/agent/skills/my-skill");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\n---\nDo the thing carefully.\n",
        )
        .unwrap();

        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let tool = UseSkillTool {
            cave: shared_cave(cave),
        };

        let output = tool
            .call(UseSkillArgs {
                name: "my-skill".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(output.name, "my-skill");
        assert_eq!(output.instructions, "Do the thing carefully.\n");
    }

    #[tokio::test]
    async fn write_agent_doc_creates_and_read_agent_doc_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let cave = shared_cave(crate::cave::Cave::open(dir.path().to_path_buf()).unwrap());
        let write = WriteAgentDocTool { cave: cave.clone() };
        let read = ReadAgentDocTool { cave: cave.clone() };

        // Create a new task with body + description.
        write
            .call(WriteAgentDocArgs {
                kind: AgentDocKind::Task,
                name: "summarize".to_string(),
                body: Some("Summarize: {{ input }}\n".to_string()),
                description: Some("Summarize something.".to_string()),
            })
            .await
            .unwrap();

        let doc = read
            .call(ReadAgentDocArgs {
                kind: AgentDocKind::Task,
                name: "summarize".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(doc.description, "Summarize something.");
        assert_eq!(doc.body, "Summarize: {{ input }}\n");

        // Partial update: a new description keeps the existing body.
        write
            .call(WriteAgentDocArgs {
                kind: AgentDocKind::Task,
                name: "summarize".to_string(),
                body: None,
                description: Some("Better description.".to_string()),
            })
            .await
            .unwrap();
        let doc = read
            .call(ReadAgentDocArgs {
                kind: AgentDocKind::Task,
                name: "summarize".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(doc.description, "Better description.");
        assert_eq!(doc.body, "Summarize: {{ input }}\n");
    }

    #[tokio::test]
    async fn write_agent_doc_creates_skills_with_valid_names_only() {
        let dir = tempfile::tempdir().unwrap();
        let cave = shared_cave(crate::cave::Cave::open(dir.path().to_path_buf()).unwrap());
        let write = WriteAgentDocTool { cave: cave.clone() };

        write
            .call(WriteAgentDocArgs {
                kind: AgentDocKind::Skill,
                name: "code-review".to_string(),
                body: Some("Review carefully.\n".to_string()),
                description: Some("How to review code.".to_string()),
            })
            .await
            .unwrap();
        assert!(dir
            .path()
            .join(".granit/agent/skills/code-review/SKILL.md")
            .exists());

        let err = write
            .call(WriteAgentDocArgs {
                kind: AgentDocKind::Skill,
                name: "Bad Name".to_string(),
                body: None,
                description: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CaveError::InvalidSkillName(_)), "{err:?}");
    }

    #[tokio::test]
    async fn use_skill_tool_errors_for_unknown_skill() {
        let dir = tempfile::tempdir().unwrap();
        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let tool = UseSkillTool {
            cave: shared_cave(cave),
        };

        let err = tool
            .call(UseSkillArgs {
                name: "missing".to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CaveError::SkillNotFound(_)));
    }
}
