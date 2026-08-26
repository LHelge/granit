//! System prompt assembly.
//!
//! The system prompt is a Tera template — either the user's
//! `.granit/agent/system.md` or the built-in default — rendered with a
//! context describing the agent's current configuration.

use chrono::Datelike;
use granit_types::{AgentDocInfo, AgentMode, NOTE_ICONS};

/// Configuration-derived values exposed to the system prompt template.
pub(crate) struct PromptContext {
    pub mode: AgentMode,
    /// Names of the tools that will be registered for this agent.
    pub tools: Vec<String>,
    /// Skills available in the cave (name + description); the full
    /// instructions are loaded on demand via the `use_skill` tool.
    pub skills: Vec<AgentDocInfo>,
}

/// Render the system prompt template `base` with the standard context
/// variables: `mode`, `tools`, `icons`, `skills`, and the date variables
/// `today`, `year`, `month`, `day`, `weekday`, `weekday_short`.
///
/// A template error falls back to the raw text: a typo in the user's
/// `system.md` must never prevent the agent from building.
pub(crate) fn assemble_system_prompt(base: &str, ctx: &PromptContext) -> String {
    let mut context = tera::Context::new();

    let now = chrono::Local::now();
    context.insert("today", &now.format("%Y-%m-%d").to_string());
    context.insert("year", &now.year());
    context.insert("month", &now.month());
    context.insert("day", &now.day());
    context.insert("weekday", &now.format("%A").to_string());
    context.insert("weekday_short", &now.format("%a").to_string());

    context.insert("mode", &ctx.mode);
    context.insert("tools", &ctx.tools);

    let icons: Vec<&str> = NOTE_ICONS.iter().map(|e| e.id).collect();
    context.insert("icons", &icons);

    context.insert("skills", &ctx.skills);

    match tera::Tera::one_off(base, &context, false) {
        Ok(rendered) => rendered,
        Err(e) => {
            log::warn!("system prompt template failed to render, using it as plain text: {e}");
            base.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use granit_types::DEFAULT_SYSTEM_PROMPT_TEMPLATE;

    fn ctx(mode: AgentMode) -> PromptContext {
        PromptContext {
            mode,
            tools: vec!["read_note".to_string(), "list_notes".to_string()],
            skills: Vec::new(),
        }
    }

    #[test]
    fn default_template_renders_agent_mode() {
        let prompt = assemble_system_prompt(DEFAULT_SYSTEM_PROMPT_TEMPLATE, &ctx(AgentMode::Agent));

        assert!(prompt.contains("Agent mode"), "got: {prompt}");
        assert!(!prompt.contains("Ask mode"), "got: {prompt}");
        // Icon IDs are joined into the agent-mode branch.
        assert!(prompt.contains("AlarmClock"), "got: {prompt}");
        // Date line is rendered from {{ today }}.
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert!(prompt.contains(&today), "got: {prompt}");
        // No leftover template syntax.
        assert!(!prompt.contains("{{"), "got: {prompt}");
        assert!(!prompt.contains("{%"), "got: {prompt}");
        // The skills block is absent while no skills exist.
        assert!(!prompt.contains("use_skill"), "got: {prompt}");
    }

    #[test]
    fn default_template_renders_ask_mode() {
        let prompt = assemble_system_prompt(DEFAULT_SYSTEM_PROMPT_TEMPLATE, &ctx(AgentMode::Ask));

        assert!(prompt.contains("Ask mode"), "got: {prompt}");
        assert!(!prompt.contains("Agent mode"), "got: {prompt}");
        assert!(!prompt.contains("AlarmClock"), "got: {prompt}");
    }

    #[test]
    fn default_template_lists_skills_when_present() {
        let mut context = ctx(AgentMode::Agent);
        context.skills = vec![AgentDocInfo {
            name: "research-summary".to_string(),
            description: "Write research summaries in my preferred style.".to_string(),
        }];
        let prompt = assemble_system_prompt(DEFAULT_SYSTEM_PROMPT_TEMPLATE, &context);

        assert!(
            prompt.contains("research-summary: Write research summaries"),
            "got: {prompt}"
        );
        assert!(prompt.contains("use_skill"), "got: {prompt}");
    }

    #[test]
    fn custom_template_can_use_context_variables() {
        let prompt = assemble_system_prompt(
            "Mode: {{ mode }}. Tools: {{ tools | join(sep=\", \") }}.",
            &ctx(AgentMode::Ask),
        );
        assert_eq!(prompt, "Mode: ask. Tools: read_note, list_notes.");
    }

    #[test]
    fn template_error_falls_back_to_raw_text() {
        let broken = "Hello {{ nonexistent_variable }} world {% if %}";
        let prompt = assemble_system_prompt(broken, &ctx(AgentMode::Agent));
        assert_eq!(prompt, broken);
    }
}
