//! User-authored agent documents stored under `.granit/agent/`.
//!
//! These files are edited as raw text (frontmatter included) and are never
//! round-tripped through the typed frontmatter machinery, so any keys the app
//! does not know about survive untouched.

use super::helpers::{normalize_note_name, validate_name, write_atomic, write_new};
use super::{Cave, CaveError};
use crate::markdown::split_frontmatter;
use chrono::Datelike;
use granit_types::{AgentDocInfo, DocumentMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The frontmatter keys of an agent document (a `SKILL.md` per the Agent
/// Skills specification at <https://agentskills.io/specification>, or a task
/// file). Parsed permissively: unknown keys (`license`, `compatibility`,
/// `metadata`, `allowed-tools`, …) are ignored here and survive on disk
/// because these files are never rewritten by the app.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct AgentDocFrontmatter {
    name: Option<String>,
    description: Option<String>,
}

/// Validate a skill name per the Agent Skills spec: 1-64 characters of
/// lowercase `a-z`, `0-9`, and hyphens; no leading/trailing hyphen and no
/// consecutive hyphens.
fn validate_skill_name(name: &str) -> Result<(), CaveError> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--");
    if valid {
        Ok(())
    } else {
        Err(CaveError::InvalidSkillName(format!(
            "{name:?} (must be 1-64 lowercase letters, digits, or hyphens; \
             no leading/trailing/double hyphens)"
        )))
    }
}

fn skill_meta(name: &str) -> DocumentMeta {
    DocumentMeta {
        slug: name.to_string(),
        relative_path: format!(".granit/agent/skills/{name}/SKILL.md"),
        icon: None,
        favorite: None,
    }
}

fn task_meta(name: &str) -> DocumentMeta {
    DocumentMeta {
        slug: name.to_string(),
        relative_path: format!(".granit/agent/tasks/{name}.md"),
        icon: None,
        favorite: None,
    }
}

/// Parse an agent document's frontmatter permissively; missing or malformed
/// frontmatter degrades to defaults instead of erroring.
fn parse_agent_doc_frontmatter(raw: &str) -> AgentDocFrontmatter {
    let (yaml, _) = split_frontmatter(raw);
    yaml.and_then(|text| serde_yml::from_str(&text).ok())
        .unwrap_or_default()
}

impl Cave {
    /// Raw contents of `.granit/agent/system.md`; `Ok(None)` when the file
    /// does not exist.
    pub fn read_system_prompt_raw(&self) -> Result<Option<String>, CaveError> {
        match std::fs::read_to_string(self.system_prompt_path()) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Atomically write `.granit/agent/system.md`, creating `.granit/agent/`
    /// as needed.
    pub fn write_system_prompt(&self, content: &str) -> Result<(), CaveError> {
        std::fs::create_dir_all(self.agent_dir())?;
        write_atomic(&self.system_prompt_path(), content)?;
        Ok(())
    }

    // ── Skills ─────────────────────────────────────────────────────

    /// Scan `.granit/agent/skills/` for skill directories containing a
    /// `SKILL.md`, returning name → SKILL.md path. Directories with invalid
    /// names or without a `SKILL.md` are logged and skipped rather than
    /// refusing to open the cave.
    pub(crate) fn scan_skills(dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
        if !dir.is_dir() {
            return Ok(HashMap::new());
        }

        let mut skills = HashMap::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Err(e) = validate_skill_name(&name) {
                log::warn!("skipping skill directory: {e}");
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.is_file() {
                log::warn!("skipping skill directory {name:?}: no SKILL.md");
                continue;
            }
            skills.insert(name, skill_md);
        }
        Ok(skills)
    }

    /// List all skills sorted by name, with their frontmatter descriptions.
    /// A frontmatter `name` that mismatches the directory name is logged;
    /// the directory name is canonical.
    pub fn list_skills(&self) -> Result<Vec<AgentDocInfo>, CaveError> {
        let mut skills: Vec<AgentDocInfo> = self
            .skills
            .iter()
            .map(|(name, path)| {
                let fm = std::fs::read_to_string(path)
                    .map(|raw| parse_agent_doc_frontmatter(&raw))
                    .unwrap_or_default();
                if let Some(fm_name) = &fm.name {
                    if fm_name != name {
                        log::warn!(
                            "skill {name:?}: frontmatter name {fm_name:?} does not match \
                             the directory name (the directory name is used)"
                        );
                    }
                }
                AgentDocInfo {
                    name: name.clone(),
                    description: fm.description.unwrap_or_default(),
                }
            })
            .collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    fn skill_md_path(&self, name: &str) -> Result<&PathBuf, CaveError> {
        self.skills
            .get(name)
            .ok_or_else(|| CaveError::SkillNotFound(name.to_string()))
    }

    /// Raw contents of a skill's `SKILL.md`, frontmatter included.
    pub fn read_skill_raw(&self, name: &str) -> Result<String, CaveError> {
        Ok(std::fs::read_to_string(self.skill_md_path(name)?)?)
    }

    /// A skill's instructions: the `SKILL.md` body with frontmatter stripped.
    pub fn skill_body(&self, name: &str) -> Result<String, CaveError> {
        let raw = self.read_skill_raw(name)?;
        let (_, body) = split_frontmatter(&raw);
        Ok(body.trim_start_matches(['\n', '\r']).to_string())
    }

    /// Create a new skill directory with a seeded `SKILL.md`.
    ///
    /// The default name `"untitled-skill"` auto-numbers on collision, like
    /// untitled notes and templates; any other existing name is an error.
    pub fn create_skill(&mut self, name: &str) -> Result<DocumentMeta, CaveError> {
        validate_skill_name(name)?;
        let name = &if name == "untitled-skill" && self.skills.contains_key(name) {
            let mut n = 2u32;
            loop {
                let candidate = format!("untitled-skill-{n}");
                if !self.skills.contains_key(&candidate) {
                    break candidate;
                }
                n = n
                    .checked_add(1)
                    .ok_or_else(|| CaveError::SlugExhausted("untitled-skill".into()))?;
            }
        } else if self.skills.contains_key(name) {
            return Err(CaveError::SkillAlreadyExists(name.to_string()));
        } else {
            name.to_string()
        };

        let skill_dir = self.agent_skills_dir().join(name);
        std::fs::create_dir_all(&skill_dir)?;
        let skill_md = skill_dir.join("SKILL.md");
        let seed = format!(
            "---\nname: {name}\ndescription: Describe what this skill does and when to use it.\n---\n\nStep-by-step instructions for the agent.\n"
        );
        write_new(&skill_md, seed)?;
        self.skills.insert(name.to_string(), skill_md);
        Ok(skill_meta(name))
    }

    /// Update a skill: optionally rename its directory, then write `content`
    /// to its `SKILL.md` (rolling the rename back if the write fails).
    ///
    /// A rename does not touch the frontmatter `name:` field — files are
    /// never rewritten by the app; a mismatch is tolerated (and logged when
    /// listing).
    pub fn update_skill(
        &mut self,
        old_name: &str,
        new_name: &str,
        content: &str,
    ) -> Result<DocumentMeta, CaveError> {
        validate_skill_name(new_name)?;
        let old_md = self.skill_md_path(old_name)?.clone();

        let renamed = old_name != new_name;
        let final_md = if renamed {
            if self.skills.contains_key(new_name) {
                return Err(CaveError::SkillAlreadyExists(new_name.to_string()));
            }
            let old_dir = self.agent_skills_dir().join(old_name);
            let new_dir = self.agent_skills_dir().join(new_name);
            std::fs::rename(&old_dir, &new_dir)?;
            new_dir.join("SKILL.md")
        } else {
            old_md
        };

        if let Err(e) = write_atomic(&final_md, content) {
            if renamed {
                let new_dir = self.agent_skills_dir().join(new_name);
                let old_dir = self.agent_skills_dir().join(old_name);
                if let Err(rollback_err) = std::fs::rename(&new_dir, &old_dir) {
                    return Err(CaveError::Io(format!(
                        "failed to write skill after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e.into());
        }

        if renamed {
            self.skills.remove(old_name);
            self.skills.insert(new_name.to_string(), final_md);
        }
        Ok(skill_meta(new_name))
    }

    /// Delete a skill directory and everything in it.
    pub fn delete_skill(&mut self, name: &str) -> Result<(), CaveError> {
        self.skill_md_path(name)?;
        std::fs::remove_dir_all(self.agent_skills_dir().join(name))?;
        self.skills.remove(name);
        Ok(())
    }

    // ── Tasks ──────────────────────────────────────────────────────

    /// List all tasks sorted by name, with their frontmatter descriptions.
    pub fn list_tasks(&self) -> Result<Vec<AgentDocInfo>, CaveError> {
        let mut tasks: Vec<AgentDocInfo> = self
            .tasks
            .iter()
            .map(|(slug, path)| {
                let fm = std::fs::read_to_string(path)
                    .map(|raw| parse_agent_doc_frontmatter(&raw))
                    .unwrap_or_default();
                AgentDocInfo {
                    name: slug.clone(),
                    description: fm.description.unwrap_or_default(),
                }
            })
            .collect();
        tasks.sort_by_key(|t| t.name.to_lowercase());
        Ok(tasks)
    }

    fn task_path(&self, slug: &str) -> Result<&PathBuf, CaveError> {
        self.tasks
            .get(slug)
            .ok_or_else(|| CaveError::TaskNotFound(slug.to_string()))
    }

    /// Raw contents of a task file, frontmatter included.
    pub fn read_task_raw(&self, slug: &str) -> Result<String, CaveError> {
        Ok(std::fs::read_to_string(self.task_path(slug)?)?)
    }

    /// Create a new task file in `.granit/agent/tasks`.
    ///
    /// The default name `"untitled-task"` auto-numbers on collision, like
    /// untitled notes and templates; any other existing name is an error.
    pub fn create_task(&mut self, name: &str) -> Result<DocumentMeta, CaveError> {
        let name = normalize_note_name(name);
        validate_name(name)?;

        let name = &if name == "untitled-task" && self.tasks.contains_key(name) {
            let mut n = 2u32;
            loop {
                let candidate = format!("untitled-task-{n}");
                if !self.tasks.contains_key(&candidate) {
                    break candidate;
                }
                n = n
                    .checked_add(1)
                    .ok_or_else(|| CaveError::SlugExhausted("untitled-task".into()))?;
            }
        } else if self.tasks.contains_key(name) {
            return Err(CaveError::TaskAlreadyExists(name.to_string()));
        } else {
            name.to_string()
        };

        let tasks_dir = self.agent_tasks_dir();
        std::fs::create_dir_all(&tasks_dir)?;
        let path = tasks_dir.join(format!("{name}.md"));
        let seed = "---\ndescription: Describe what this task does.\n---\n\nWrite the prompt to send here. The text typed after the slash command is available as {{ input }}.\n";
        write_new(&path, seed)?;
        self.tasks.insert(name.to_string(), path);
        Ok(task_meta(name))
    }

    /// Update a task: optionally rename its file, then write `content`
    /// (rolling the rename back if the write fails).
    pub fn update_task(
        &mut self,
        old_name: &str,
        new_name: &str,
        content: &str,
    ) -> Result<DocumentMeta, CaveError> {
        let new_name = normalize_note_name(new_name);
        validate_name(new_name)?;
        let old_path = self.task_path(old_name)?.clone();

        let renamed = old_name != new_name;
        let final_path = if renamed {
            if self.tasks.contains_key(new_name) {
                return Err(CaveError::TaskAlreadyExists(new_name.to_string()));
            }
            let new_path = self.agent_tasks_dir().join(format!("{new_name}.md"));
            std::fs::rename(&old_path, &new_path)?;
            new_path
        } else {
            old_path.clone()
        };

        if let Err(e) = write_atomic(&final_path, content) {
            if renamed {
                if let Err(rollback_err) = std::fs::rename(&final_path, &old_path) {
                    return Err(CaveError::Io(format!(
                        "failed to write task after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e.into());
        }

        if renamed {
            self.tasks.remove(old_name);
            self.tasks.insert(new_name.to_string(), final_path);
        }
        Ok(task_meta(new_name))
    }

    /// Delete a task file.
    pub fn delete_task(&mut self, slug: &str) -> Result<(), CaveError> {
        let path = self.task_path(slug)?.clone();
        std::fs::remove_file(path)?;
        self.tasks.remove(slug);
        Ok(())
    }

    /// Render a task's body as the prompt to send to the agent.
    ///
    /// The body (frontmatter stripped) is a Tera template with access to
    /// `input` (the text typed after the slash command), `active_note` (only
    /// when a note is open in the editor), and today's date variables.
    ///
    /// Unlike the system prompt, a template error propagates: a broken task
    /// should fail visibly in the chat instead of silently sending mangled
    /// text.
    pub fn render_task(&self, slug: &str, input: &str) -> Result<String, CaveError> {
        let raw = self.read_task_raw(slug)?;
        let (_, body) = split_frontmatter(&raw);
        let body = body.trim_start_matches(['\n', '\r']);

        let mut context = tera::Context::new();
        context.insert("input", input);
        if let Some(active) = self.active_slug() {
            context.insert("active_note", active);
        }
        let now = chrono::Local::now();
        context.insert("today", &now.format("%Y-%m-%d").to_string());
        context.insert(
            "tomorrow",
            &(now.date_naive() + chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string(),
        );
        context.insert(
            "yesterday",
            &(now.date_naive() - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string(),
        );
        context.insert("year", &now.year());
        context.insert("month", &now.month());
        context.insert("day", &now.day());
        context.insert("weekday", &now.format("%A").to_string());
        context.insert("weekday_short", &now.format("%a").to_string());

        Ok(tera::Tera::one_off(body, &context, false)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::cave::{Cave, CaveError};

    #[test]
    fn test_system_prompt_roundtrip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(cave.read_system_prompt_raw().unwrap(), None);

        cave.write_system_prompt("custom prompt {{ today }}")
            .unwrap();
        assert_eq!(
            cave.read_system_prompt_raw().unwrap().as_deref(),
            Some("custom prompt {{ today }}")
        );
        assert!(dir.path().join(".granit/agent/system.md").exists());
    }

    // ── Skills ─────────────────────────────────────────────────────

    #[test]
    fn test_create_and_list_skills_with_description() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_skill("pdf-processing").unwrap();
        cave.update_skill(
            "pdf-processing",
            "pdf-processing",
            "---\nname: pdf-processing\ndescription: Handle PDFs.\nlicense: MIT\n---\n\nDo PDF things.\n",
        )
        .unwrap();

        let skills = cave.list_skills().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "pdf-processing");
        assert_eq!(skills[0].description, "Handle PDFs.");

        // Unknown spec keys survive on disk: the app never rewrites the file.
        let raw = cave.read_skill_raw("pdf-processing").unwrap();
        assert!(raw.contains("license: MIT"));

        // The body strips frontmatter.
        assert_eq!(
            cave.skill_body("pdf-processing").unwrap(),
            "Do PDF things.\n"
        );
    }

    #[test]
    fn test_create_skill_rejects_invalid_names_and_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        for bad in [
            "",
            "PDF-Processing",
            "-pdf",
            "pdf-",
            "pdf--processing",
            "a b",
        ] {
            let err = cave.create_skill(bad).unwrap_err();
            assert!(
                matches!(err, CaveError::InvalidSkillName(_)),
                "{bad}: {err:?}"
            );
        }

        cave.create_skill("dup").unwrap();
        let err = cave.create_skill("dup").unwrap_err();
        assert!(matches!(err, CaveError::SkillAlreadyExists(_)));
    }

    #[test]
    fn test_scan_skills_finds_valid_dirs_and_skips_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join(".granit/agent/skills");
        std::fs::create_dir_all(skills_dir.join("good-skill")).unwrap();
        std::fs::write(
            skills_dir.join("good-skill/SKILL.md"),
            "---\nname: good-skill\ndescription: ok\n---\nbody",
        )
        .unwrap();
        // Invalid name: skipped, not fatal.
        std::fs::create_dir_all(skills_dir.join("Bad Name")).unwrap();
        std::fs::write(skills_dir.join("Bad Name/SKILL.md"), "x").unwrap();
        // Missing SKILL.md: skipped.
        std::fs::create_dir_all(skills_dir.join("empty-skill")).unwrap();

        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        let skills = cave.list_skills().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "good-skill");

        // Skills never leak into the note index.
        assert!(cave.list_notes().unwrap().is_empty());
    }

    #[test]
    fn test_update_skill_renames_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_skill("old-name").unwrap();
        let meta = cave
            .update_skill("old-name", "new-name", "new content")
            .unwrap();

        assert_eq!(meta.slug, "new-name");
        assert!(dir
            .path()
            .join(".granit/agent/skills/new-name/SKILL.md")
            .exists());
        assert!(!dir.path().join(".granit/agent/skills/old-name").exists());
        assert_eq!(cave.read_skill_raw("new-name").unwrap(), "new content");
        assert!(matches!(
            cave.read_skill_raw("old-name").unwrap_err(),
            CaveError::SkillNotFound(_)
        ));
    }

    // ── Tasks ──────────────────────────────────────────────────────

    #[test]
    fn test_create_list_and_delete_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_task("summarize").unwrap();
        cave.update_task(
            "summarize",
            "summarize",
            "---\ndescription: Summarize something.\n---\n\nSummarize: {{ input }}\n",
        )
        .unwrap();

        let tasks = cave.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "summarize");
        assert_eq!(tasks[0].description, "Summarize something.");

        // Tasks never leak into the note index.
        assert!(cave.list_notes().unwrap().is_empty());

        cave.delete_task("summarize").unwrap();
        assert!(cave.list_tasks().unwrap().is_empty());
        assert!(!dir.path().join(".granit/agent/tasks/summarize.md").exists());
    }

    #[test]
    fn test_render_task_substitutes_input_and_context() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_task("review").unwrap();
        cave.update_task(
            "review",
            "review",
            "---\ndescription: d\n---\nReview {{ input }} on {{ today }} for [[{{ active_note }}]]",
        )
        .unwrap();
        cave.set_active_slug(Some("my-note".to_string()));

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let rendered = cave.render_task("review", "chapter two").unwrap();
        assert_eq!(
            rendered,
            format!("Review chapter two on {today} for [[my-note]]")
        );
    }

    #[test]
    fn test_render_task_errors_propagate() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        // Missing task errors.
        assert!(matches!(
            cave.render_task("missing", "x").unwrap_err(),
            CaveError::TaskNotFound(_)
        ));

        // A task referencing active_note with no note open is a render error,
        // not a silent fallback.
        cave.create_task("broken").unwrap();
        cave.update_task("broken", "broken", "Note: {{ active_note }}")
            .unwrap();
        assert!(matches!(
            cave.render_task("broken", "x").unwrap_err(),
            CaveError::TemplateRender(_)
        ));
    }

    #[test]
    fn test_delete_skill_removes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.create_skill("doomed").unwrap();
        cave.delete_skill("doomed").unwrap();

        assert!(!dir.path().join(".granit/agent/skills/doomed").exists());
        assert!(cave.list_skills().unwrap().is_empty());
    }
}
