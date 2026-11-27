//! Agent settings and prompts of the UI, on top of the core.
//!
//! The counterpart of the Tauri commands in `src-tauri/src/commands/config.rs`
//! (`select_provider`, `select_model`, `select_mode`, `list_models`) and
//! `commands/cave.rs` (`list_tasks`, `render_task`): each selection changes
//! the cave's config, saves it to `.granit/config.yml` and resets the agent,
//! so the next message builds a new one (with empty history).

use gpui::App;
use granit_types::{AgentDocInfo, AgentMode, AppConfig, ProviderConfig};

use crate::config_ops;
use crate::host::Host;

/// Change the config with `f` through the one config write path
/// ([`config_ops::update_config_quietly`]: settings edits still pending are
/// committed first, then the change is saved and the agent reset when its
/// config changed). The chat applies its own reset, so no `ConfigEvent` is
/// sent for the pick. `f` validates before it changes anything; on its
/// error nothing changes. `f` returns whether it changed anything: when
/// not, nothing is saved, the agent (and its history) is kept, and this
/// returns `Ok(false)`.
fn update_agent_config(
    cx: &mut App,
    f: impl FnOnce(&mut AppConfig) -> Result<bool, String>,
) -> Result<bool, String> {
    // Run `f` on a copy first, so a validation error or a no-op changes
    // nothing and commits nothing.
    let mut next = Host::core(cx).lock_config().clone();
    if !f(&mut next)? {
        return Ok(false);
    }
    config_ops::update_config_quietly(cx, |config| *config = next)
}

/// Use the provider at `index`; its default model until one is picked.
/// Returns whether the selection changed.
pub fn select_provider(index: usize, cx: &mut App) -> Result<bool, String> {
    update_agent_config(cx, |config| {
        if index >= config.agent.providers.len() {
            return Err(format!("Provider index {index} out of range"));
        }
        if config.agent.selected_provider == index {
            return Ok(false);
        }
        config.agent.selected_provider = index;
        config.agent.selected_model = None;
        Ok(true)
    })
}

/// Use the model `id` of the selected provider. Returns whether the
/// selection changed.
pub fn select_model(id: String, cx: &mut App) -> Result<bool, String> {
    update_agent_config(cx, |config| {
        if config.agent.selected_model.as_deref() == Some(id.as_str()) {
            return Ok(false);
        }
        config.agent.selected_model = Some(id);
        Ok(true)
    })
}

/// Switch between Ask and Agent mode. Returns whether the mode changed.
pub fn select_mode(mode: AgentMode, cx: &mut App) -> Result<bool, String> {
    update_agent_config(cx, |config| {
        if config.agent.mode == mode {
            return Ok(false);
        }
        config.agent.mode = mode;
        Ok(true)
    })
}

/// Remember whether the chat panel is shown, in the cave's config. Does not
/// touch the agent.
pub fn set_chat_visible(visible: bool, cx: &mut App) -> Result<(), String> {
    config_ops::update_config_quietly(cx, |config| config.agent_panel.visible = visible).map(|_| ())
}

/// The selected provider, for listing its models.
pub fn selected_provider(cx: &App) -> Option<ProviderConfig> {
    let core = Host::core(cx);
    let config = core.lock_config();
    config
        .agent
        .providers
        .get(config.agent.selected_provider)
        .map(|entry| entry.provider.clone())
}

/// The `/task` prompts of the open cave (none without one).
pub fn list_tasks(cx: &App) -> Vec<AgentDocInfo> {
    Host::core(cx)
        .with_cave(|cave| cave.list_tasks())
        .unwrap_or_default()
}

/// Render the task `name` with `input` into the prompt to send.
pub fn render_task(name: &str, input: &str, cx: &App) -> Result<String, String> {
    Host::core(cx)
        .with_cave(|cave| cave.render_task(name, input))
        .map_err(|err| err.to_string())
}
