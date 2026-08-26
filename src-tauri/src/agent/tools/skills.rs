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
