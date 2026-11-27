//! User-authored agent documents stored under `.granit/agent/`.
//!
//! The editor works on the body; the frontmatter is managed by the app,
//! which owns only the `name` (skills) and `description` keys. The Tauri
//! app's `update_*` writes go through [`rebuild_agent_doc`], a rewrite of a
//! generic YAML mapping, so keys the app does not know about (`license`,
//! `metadata`, `allowed-tools`, …) survive — frontmatter comments do not.
//! The GPUI app's `save_*`, `set_*_description` and `rename_*` splice only
//! the body or the one key they change and keep everything else byte for
//! byte, comments included.

use super::helpers::{normalize_note_name, validate_name, write_atomic, write_new};
use super::{Cave, CaveError};
use crate::markdown::{locate_frontmatter, splice_frontmatter_scalar, split_frontmatter};
use chrono::Datelike;
use granit_types::{AgentDocInfo, Document, DocumentMeta};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The frontmatter keys of an agent document (a `SKILL.md` per the Agent
/// Skills specification at <https://agentskills.io/specification>, or a task
/// file). Parsed permissively: unknown keys (`license`, `compatibility`,
/// `metadata`, `allowed-tools`, …) are ignored here; they survive on disk
/// because the app's writes only touch the keys it owns (see the module
/// docs).
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

fn skill_meta(name: &str, description: Option<String>) -> DocumentMeta {
    DocumentMeta {
        slug: name.to_string(),
        relative_path: format!(".granit/agent/skills/{name}/SKILL.md"),
        icon: None,
        favorite: None,
        description,
    }
}

fn task_meta(name: &str, description: Option<String>) -> DocumentMeta {
    DocumentMeta {
        slug: name.to_string(),
        relative_path: format!(".granit/agent/tasks/{name}.md"),
        icon: None,
        favorite: None,
        description,
    }
}

/// Parse an agent document's frontmatter permissively; missing or malformed
/// frontmatter degrades to defaults instead of erroring.
fn parse_agent_doc_frontmatter(raw: &str) -> AgentDocFrontmatter {
    let (yaml, _) = split_frontmatter(raw);
    yaml.and_then(|text| serde_yml::from_str(&text).ok())
        .unwrap_or_default()
}

/// Strip the frontmatter block and any blank lines that follow it.
fn agent_doc_body(raw: &str) -> String {
    raw[agent_doc_body_start(raw)..].to_string()
}

/// Where the body of an agent document starts in `raw`: after the
/// frontmatter block and the blank lines that follow it.
fn agent_doc_body_start(raw: &str) -> usize {
    let start = locate_frontmatter(raw).map_or(0, |span| span.body_start);
    let rest = &raw[start..];
    start + rest.len() - rest.trim_start_matches(['\n', '\r']).len()
}

/// `raw` with its body (as [`agent_doc_body`] reads it) replaced by `body`;
/// the frontmatter and the blank lines after it are kept byte for byte.
fn splice_agent_doc_body(raw: &str, body: &str) -> String {
    format!("{}{body}", &raw[..agent_doc_body_start(raw)])
}

/// `raw` with the frontmatter key `key` set to `value`, splicing only that
/// key; unchanged when it already holds `value`.
fn splice_agent_doc_key(raw: &str, key: &str, value: &str) -> String {
    let (yaml, _) = split_frontmatter(raw);
    let current = yaml
        .and_then(|text| serde_yml::from_str::<serde_yml::Mapping>(&text).ok())
        .and_then(|map| map.get(key).and_then(|v| v.as_str()).map(str::to_string));
    if current.as_deref() == Some(value) {
        return raw.to_string();
    }
    splice_frontmatter_scalar(raw, key, value)
}

/// Apply `update` to the raw content of the agent document at `path` and
/// write the result; writes nothing (and returns `false`) when `update`
/// changes nothing.
fn rewrite_agent_doc(path: &Path, update: impl FnOnce(&str) -> String) -> Result<bool, CaveError> {
    let raw = std::fs::read_to_string(path)?;
    let updated = update(&raw);
    if updated == raw {
        return Ok(false);
    }
    write_atomic(path, updated)?;
    Ok(true)
}

/// Rebuild an agent document from its existing raw content: the frontmatter
/// is parsed into a generic YAML mapping, `set` updates only the keys the
/// app owns, and everything else — including spec keys the app doesn't know
/// (`license`, `compatibility`, `metadata`, `allowed-tools`, …) — is carried
/// over unchanged. Comments in the frontmatter do not survive the rewrite.
fn rebuild_agent_doc(raw: &str, body: &str, set: impl FnOnce(&mut serde_yml::Mapping)) -> String {
    let (yaml, _) = split_frontmatter(raw);
    let mut map: serde_yml::Mapping = yaml
        .filter(|text| !text.trim().is_empty())
        .and_then(|text| match serde_yml::from_str(&text) {
            Ok(map) => Some(map),
            Err(e) => {
                log::warn!("agent document frontmatter is not valid YAML, rebuilding it: {e}");
                None
            }
        })
        .unwrap_or_default();
    set(&mut map);
    // The serializer emits no trailing newline; the closing fence needs one.
    let mut yaml_out = serde_yml::to_string(&map).unwrap_or_default();
    if !yaml_out.ends_with('\n') {
        yaml_out.push('\n');
    }
    let body = body.trim_start_matches(['\n', '\r']);
    let newline = if body.is_empty() || body.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("---\n{yaml_out}---\n\n{body}{newline}")
}

/// Set a string key on a frontmatter mapping, preserving its position when
/// the key already exists.
fn set_frontmatter_key(map: &mut serde_yml::Mapping, key: &str, value: &str) {
    map.insert(key, serde_yml::Value::from(value));
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
    pub fn scan_skills(dir: &Path) -> Result<HashMap<String, PathBuf>, CaveError> {
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
        Ok(agent_doc_body(&self.read_skill_raw(name)?))
    }

    /// Read a skill for editing: the `SKILL.md` body as content, with the
    /// frontmatter description on the metadata. The frontmatter itself is
    /// managed by the app, like note frontmatter.
    pub fn read_skill(&self, name: &str) -> Result<Document, CaveError> {
        let raw = self.read_skill_raw(name)?;
        let fm = parse_agent_doc_frontmatter(&raw);
        Ok(Document {
            meta: skill_meta(name, Some(fm.description.unwrap_or_default())),
            content: agent_doc_body(&raw),
        })
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
        Ok(skill_meta(
            name,
            Some("Describe what this skill does and when to use it.".to_string()),
        ))
    }

    /// Update a skill: optionally rename its directory, then rebuild its
    /// `SKILL.md` from `body` and `description` (rolling the rename back if
    /// the write fails).
    ///
    /// The app owns the `name` (kept in sync with the directory) and
    /// `description` frontmatter keys; all other keys are preserved.
    pub fn update_skill(
        &mut self,
        old_name: &str,
        new_name: &str,
        body: &str,
        description: &str,
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

        let write_result = std::fs::read_to_string(&final_md)
            .map_err(CaveError::from)
            .and_then(|existing_raw| {
                let updated = rebuild_agent_doc(&existing_raw, body, |map| {
                    set_frontmatter_key(map, "name", new_name);
                    set_frontmatter_key(map, "description", description);
                });
                write_atomic(&final_md, updated).map_err(CaveError::from)
            });
        if let Err(e) = write_result {
            if renamed {
                let new_dir = self.agent_skills_dir().join(new_name);
                let old_dir = self.agent_skills_dir().join(old_name);
                if let Err(rollback_err) = std::fs::rename(&new_dir, &old_dir) {
                    return Err(CaveError::Io(format!(
                        "failed to write skill after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e);
        }

        if renamed {
            self.skills.remove(old_name);
            self.skills.insert(new_name.to_string(), final_md);
        }
        Ok(skill_meta(new_name, Some(description.to_string())))
    }

    /// Delete a skill directory and everything in it.
    pub fn delete_skill(&mut self, name: &str) -> Result<(), CaveError> {
        self.skill_md_path(name)?;
        std::fs::remove_dir_all(self.agent_skills_dir().join(name))?;
        self.skills.remove(name);
        Ok(())
    }

    /// Write `body` as a skill's instructions, under its frontmatter kept
    /// byte for byte (with the blank lines after it). Returns whether the
    /// file changed: an unchanged body writes nothing.
    pub fn save_skill(&self, name: &str, body: &str) -> Result<bool, CaveError> {
        rewrite_agent_doc(self.skill_md_path(name)?, |raw| {
            splice_agent_doc_body(raw, body)
        })
    }

    /// Set a skill's frontmatter `description`, splicing only that key (its
    /// quoting kept); the body and every other key, comment and line stay
    /// as they are. Returns whether the file changed.
    pub fn set_skill_description(&self, name: &str, description: &str) -> Result<bool, CaveError> {
        rewrite_agent_doc(self.skill_md_path(name)?, |raw| {
            splice_agent_doc_key(raw, "description", description)
        })
    }

    /// Rename a skill: move its directory (with everything in it) and set
    /// the frontmatter `name` to match, splicing only that key. A failed
    /// write moves the directory back. Renaming to the same name changes
    /// nothing.
    pub fn rename_skill(
        &mut self,
        old_name: &str,
        new_name: &str,
    ) -> Result<DocumentMeta, CaveError> {
        validate_skill_name(new_name)?;
        self.skill_md_path(old_name)?;
        if old_name == new_name {
            return Ok(self.read_skill(old_name)?.meta);
        }
        let old_dir = self.agent_skills_dir().join(old_name);
        let new_dir = self.agent_skills_dir().join(new_name);
        if self.skills.contains_key(new_name) || new_dir.exists() {
            return Err(CaveError::SkillAlreadyExists(new_name.to_string()));
        }
        std::fs::rename(&old_dir, &new_dir)?;
        let new_md = new_dir.join("SKILL.md");
        if let Err(e) =
            rewrite_agent_doc(&new_md, |raw| splice_agent_doc_key(raw, "name", new_name))
        {
            if let Err(rollback_err) = std::fs::rename(&new_dir, &old_dir) {
                return Err(CaveError::Io(format!(
                    "failed to write skill after rename: {e}; rollback also failed: {rollback_err}"
                )));
            }
            return Err(e);
        }
        self.skills.remove(old_name);
        self.skills.insert(new_name.to_string(), new_md);
        Ok(self.read_skill(new_name)?.meta)
    }

    /// The cave-relative directory of a skill, which its relative images
    /// resolve from.
    pub fn skill_dir(&self, name: &str) -> Result<String, CaveError> {
        self.skill_md_path(name)?;
        Ok(self.relative_dir(&self.agent_skills_dir().join(name)))
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
        Ok(task_meta(
            name,
            Some("Describe what this task does.".to_string()),
        ))
    }

    /// Read a task for editing: the body as content, with the frontmatter
    /// description on the metadata. The frontmatter itself is managed by
    /// the app, like note frontmatter.
    pub fn read_task(&self, slug: &str) -> Result<Document, CaveError> {
        let raw = self.read_task_raw(slug)?;
        let fm = parse_agent_doc_frontmatter(&raw);
        Ok(Document {
            meta: task_meta(slug, Some(fm.description.unwrap_or_default())),
            content: agent_doc_body(&raw),
        })
    }

    /// Update a task: optionally rename its file, then rebuild it from
    /// `body` and `description` (rolling the rename back if the write
    /// fails). The app owns the `description` frontmatter key; all other
    /// keys are preserved.
    pub fn update_task(
        &mut self,
        old_name: &str,
        new_name: &str,
        body: &str,
        description: &str,
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

        let write_result = std::fs::read_to_string(&final_path)
            .map_err(CaveError::from)
            .and_then(|existing_raw| {
                let updated = rebuild_agent_doc(&existing_raw, body, |map| {
                    set_frontmatter_key(map, "description", description);
                });
                write_atomic(&final_path, updated).map_err(CaveError::from)
            });
        if let Err(e) = write_result {
            if renamed {
                if let Err(rollback_err) = std::fs::rename(&final_path, &old_path) {
                    return Err(CaveError::Io(format!(
                        "failed to write task after rename: {e}; rollback also failed: {rollback_err}"
                    )));
                }
            }
            return Err(e);
        }

        if renamed {
            self.tasks.remove(old_name);
            self.tasks.insert(new_name.to_string(), final_path);
        }
        Ok(task_meta(new_name, Some(description.to_string())))
    }

    /// Delete a task file.
    pub fn delete_task(&mut self, slug: &str) -> Result<(), CaveError> {
        let path = self.task_path(slug)?.clone();
        std::fs::remove_file(path)?;
        self.tasks.remove(slug);
        Ok(())
    }

    /// Write `body` as a task's prompt, under its frontmatter kept byte for
    /// byte (with the blank lines after it). Returns whether the file
    /// changed: an unchanged body writes nothing.
    pub fn save_task(&self, slug: &str, body: &str) -> Result<bool, CaveError> {
        rewrite_agent_doc(self.task_path(slug)?, |raw| {
            splice_agent_doc_body(raw, body)
        })
    }

    /// Set a task's frontmatter `description`, splicing only that key; the
    /// body and every other key, comment and line stay as they are. Returns
    /// whether the file changed.
    pub fn set_task_description(&self, slug: &str, description: &str) -> Result<bool, CaveError> {
        rewrite_agent_doc(self.task_path(slug)?, |raw| {
            splice_agent_doc_key(raw, "description", description)
        })
    }

    /// Rename a task file; its content is not touched. Renaming to the same
    /// name changes nothing.
    pub fn rename_task(
        &mut self,
        old_name: &str,
        new_name: &str,
    ) -> Result<DocumentMeta, CaveError> {
        let new_name = normalize_note_name(new_name);
        validate_name(new_name)?;
        let old_path = self.task_path(old_name)?.clone();
        if old_name == new_name {
            return Ok(self.read_task(old_name)?.meta);
        }
        let new_path = self.agent_tasks_dir().join(format!("{new_name}.md"));
        // A case-only rename of the same file is allowed (on a
        // case-insensitive file system the new path "exists" already).
        let case_only = old_name.eq_ignore_ascii_case(new_name);
        if self.tasks.contains_key(new_name) || (new_path.exists() && !case_only) {
            return Err(CaveError::TaskAlreadyExists(new_name.to_string()));
        }
        std::fs::rename(&old_path, &new_path)?;
        self.tasks.remove(old_name);
        self.tasks.insert(new_name.to_string(), new_path);
        Ok(self.read_task(new_name)?.meta)
    }

    /// The cave-relative directory of a task (the tasks directory), which
    /// its relative images resolve from.
    pub fn task_dir(&self, slug: &str) -> Result<String, CaveError> {
        self.task_path(slug)?;
        Ok(self.relative_dir(&self.agent_tasks_dir()))
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
        let body = agent_doc_body(&self.read_task_raw(slug)?);

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

        Ok(tera::Tera::one_off(&body, &context, false)?)
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
            "Do PDF things.\n",
            "Handle PDFs.",
        )
        .unwrap();

        let skills = cave.list_skills().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "pdf-processing");
        assert_eq!(skills[0].description, "Handle PDFs.");

        // Reading for editing yields the body + the description on the meta.
        let doc = cave.read_skill("pdf-processing").unwrap();
        assert_eq!(doc.content, "Do PDF things.\n");
        assert_eq!(doc.meta.description.as_deref(), Some("Handle PDFs."));

        // The tool body strips frontmatter too.
        assert_eq!(
            cave.skill_body("pdf-processing").unwrap(),
            "Do PDF things.\n"
        );
    }

    #[test]
    fn test_update_skill_preserves_unknown_frontmatter_keys() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join(".granit/agent/skills/spec-skill");
        std::fs::create_dir_all(&skills_dir).unwrap();
        // A dropped-in spec-compliant skill with keys the app doesn't manage.
        std::fs::write(
            skills_dir.join("SKILL.md"),
            "---\nname: spec-skill\ndescription: Old description.\nlicense: MIT\nmetadata:\n  author: example-org\n---\n\nOld body.\n",
        )
        .unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        cave.update_skill(
            "spec-skill",
            "spec-skill",
            "New body.\n",
            "New description.",
        )
        .unwrap();

        let raw = cave.read_skill_raw("spec-skill").unwrap();
        assert!(raw.contains("license: MIT"), "got: {raw}");
        assert!(raw.contains("author: example-org"), "got: {raw}");
        assert!(raw.contains("description: New description."), "got: {raw}");
        assert!(!raw.contains("Old description."), "got: {raw}");
        assert!(raw.ends_with("---\n\nNew body.\n"), "got: {raw}");
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
            .update_skill("old-name", "new-name", "new content", "a description")
            .unwrap();

        assert_eq!(meta.slug, "new-name");
        assert!(dir
            .path()
            .join(".granit/agent/skills/new-name/SKILL.md")
            .exists());
        assert!(!dir.path().join(".granit/agent/skills/old-name").exists());
        // The frontmatter name field is kept in sync with the directory.
        let raw = cave.read_skill_raw("new-name").unwrap();
        assert!(raw.contains("name: new-name"), "got: {raw}");
        assert_eq!(cave.skill_body("new-name").unwrap(), "new content\n");
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
            "Summarize: {{ input }}\n",
            "Summarize something.",
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
            "Review {{ input }} on {{ today }} for [[{{ active_note }}]]",
            "d",
        )
        .unwrap();
        cave.set_active_slug(Some("my-note".to_string()));

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let rendered = cave.render_task("review", "chapter two").unwrap();
        assert_eq!(
            rendered,
            format!("Review chapter two on {today} for [[my-note]]\n")
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
        cave.update_task("broken", "broken", "Note: {{ active_note }}", "d")
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

    const SPEC_SKILL: &str = "---\n# managed by hand\nname: spec-skill\ndescription: 'Old: description'\nlicense: MIT\nmetadata:\n  author: example-org\n---\n\n\nOld body.\n";

    fn spec_skill_cave() -> (tempfile::TempDir, Cave) {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join(".granit/agent/skills/spec-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), SPEC_SKILL).unwrap();
        std::fs::write(skill_dir.join("reference.md"), "kept").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        (dir, cave)
    }

    #[test]
    fn test_save_skill_splices_the_body_under_the_frontmatter_verbatim() {
        let (dir, cave) = spec_skill_cave();
        let path = dir.path().join(".granit/agent/skills/spec-skill/SKILL.md");
        let body = cave.read_skill("spec-skill").unwrap().content;
        assert_eq!(body, "Old body.\n");

        // The body as read writes nothing.
        assert!(!cave.save_skill("spec-skill", &body).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), SPEC_SKILL);

        assert!(cave.save_skill("spec-skill", "New {{ x }} body").unwrap());
        let raw = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            raw,
            SPEC_SKILL.replace("Old body.\n", "New {{ x }} body"),
            "frontmatter, comment and blank lines kept"
        );
        assert_eq!(cave.skill_body("spec-skill").unwrap(), "New {{ x }} body");
    }

    #[test]
    fn test_set_skill_description_splices_only_that_key() {
        let (dir, cave) = spec_skill_cave();
        let path = dir.path().join(".granit/agent/skills/spec-skill/SKILL.md");

        assert!(!cave
            .set_skill_description("spec-skill", "Old: description")
            .unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), SPEC_SKILL);

        assert!(cave
            .set_skill_description("spec-skill", "Handle PDFs: all of them")
            .unwrap());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            SPEC_SKILL.replace("'Old: description'", "'Handle PDFs: all of them'")
        );
        let doc = cave.read_skill("spec-skill").unwrap();
        assert_eq!(
            doc.meta.description.as_deref(),
            Some("Handle PDFs: all of them")
        );
        assert_eq!(doc.content, "Old body.\n");
    }

    #[test]
    fn test_rename_skill_moves_the_directory_and_splices_the_name() {
        let (dir, mut cave) = spec_skill_cave();
        cave.create_skill("taken").unwrap();
        let skills = dir.path().join(".granit/agent/skills");

        let meta = cave.rename_skill("spec-skill", "pdf-tools").unwrap();
        assert_eq!(meta.slug, "pdf-tools");
        assert_eq!(meta.description.as_deref(), Some("Old: description"));
        assert!(!skills.join("spec-skill").exists());
        assert_eq!(
            std::fs::read_to_string(skills.join("pdf-tools/reference.md")).unwrap(),
            "kept"
        );
        assert_eq!(
            std::fs::read_to_string(skills.join("pdf-tools/SKILL.md")).unwrap(),
            SPEC_SKILL.replace("name: spec-skill", "name: pdf-tools")
        );
        assert_eq!(
            cave.skill_dir("pdf-tools").unwrap(),
            ".granit/agent/skills/pdf-tools"
        );

        for (to, expected) in [("taken", "exists"), ("Bad Name", "invalid")] {
            let err = cave.rename_skill("pdf-tools", to).unwrap_err();
            match expected {
                "exists" => assert!(matches!(err, CaveError::SkillAlreadyExists(_)), "{err:?}"),
                _ => assert!(matches!(err, CaveError::InvalidSkillName(_)), "{err:?}"),
            }
        }
        assert!(skills.join("pdf-tools/SKILL.md").exists());
        assert!(matches!(
            cave.rename_skill("missing", "other").unwrap_err(),
            CaveError::SkillNotFound(_)
        ));
    }

    #[test]
    fn test_task_body_description_and_rename() {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join(".granit/agent/tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let raw = "---\r\ndescription: Sum up\r\nmodel: x\r\n---\r\n\r\nSummarize {{ input }}\r\n";
        std::fs::write(tasks.join("summarize.md"), raw).unwrap();
        std::fs::write(tasks.join("other.md"), "plain\n").unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let doc = cave.read_task("summarize").unwrap();
        assert!(!cave.save_task("summarize", &doc.content).unwrap());
        assert!(!cave.set_task_description("summarize", "Sum up").unwrap());
        assert_eq!(
            std::fs::read_to_string(tasks.join("summarize.md")).unwrap(),
            raw
        );

        assert!(cave
            .save_task("summarize", "Shorten {{ input }}\r\n")
            .unwrap());
        assert!(cave.set_task_description("summarize", "Shorten").unwrap());
        assert_eq!(
            std::fs::read_to_string(tasks.join("summarize.md")).unwrap(),
            "---\r\ndescription: Shorten\r\nmodel: x\r\n---\r\n\r\nShorten {{ input }}\r\n"
        );

        // A task without frontmatter gains a block for its description.
        assert!(cave.set_task_description("other", "Other things").unwrap());
        assert_eq!(
            std::fs::read_to_string(tasks.join("other.md")).unwrap(),
            "---\ndescription: Other things\n---\n\nplain\n"
        );
        assert_eq!(cave.read_task("other").unwrap().content, "plain\n");

        let meta = cave.rename_task("summarize", "shorten.md").unwrap();
        assert_eq!(meta.slug, "shorten");
        assert_eq!(meta.description.as_deref(), Some("Shorten"));
        assert!(!tasks.join("summarize.md").exists());
        assert!(tasks.join("shorten.md").exists());
        assert_eq!(cave.task_dir("shorten").unwrap(), ".granit/agent/tasks");
        assert!(matches!(
            cave.rename_task("shorten", "other").unwrap_err(),
            CaveError::TaskAlreadyExists(_)
        ));
        let names: Vec<_> = cave
            .list_tasks()
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(names, ["other", "shorten"]);
    }
}
