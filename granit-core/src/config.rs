//! Changing the cave config from a settings UI, and what a change means for
//! the agent and the RAG vector index.
//!
//! [`CoreState::update_config`] is the one write path of the GPUI app's
//! settings: it saves the cave's `.granit/config.yml` (never with
//! `active_cave`), resets the agent when its config changed, and reports a
//! [`ConfigChange`] the host applies (theme, fonts, the RAG index through
//! [`rag_index_action`], as the Tauri app's `save_config` does).

use granit_types::{AppConfig, RagConfig};

use crate::cave::CaveError;
use crate::CoreState;

/// What a config change means for the RAG vector index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagIndexAction {
    /// No index-affecting RAG change.
    None,
    /// RAG was enabled, or is enabled and the embedding model changed:
    /// the index must be (re)built.
    Rebuild,
    /// RAG was disabled: the index should be dropped.
    Disable,
}

/// What changing the RAG config from `old` to `new` means for the index.
pub fn rag_index_action(old: &RagConfig, new: &RagConfig) -> RagIndexAction {
    if new.enabled && (!old.enabled || old.embedding_model != new.embedding_model) {
        RagIndexAction::Rebuild
    } else if old.enabled && !new.enabled {
        RagIndexAction::Disable
    } else {
        // A `top_n`-only change needs no rebuild: top_n is baked into the
        // agent at build time and the caller resets the agent anyway.
        RagIndexAction::None
    }
}

/// What [`CoreState::update_config`] changed.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigChange {
    /// The agent config changed, so the agent was reset (and its history
    /// with it).
    pub agent: bool,
    /// What the host should do with the RAG index.
    pub rag: RagIndexAction,
    /// The theme changed.
    pub theme: bool,
    /// A font setting (`markdown_font`, `reading_font`, `agent_font`) changed.
    pub fonts: bool,
    /// The config after the change.
    pub config: AppConfig,
}

impl CoreState {
    /// Change the config with `f`, save it to the open cave's
    /// `.granit/config.yml` and reset the agent when the agent config
    /// changed. Returns `None` when `f` changed nothing (nothing is written).
    ///
    /// The config is saved even when the agent config does not validate (an
    /// API key still being typed): the agent then fails to build and says
    /// why. With no cave open the change stays in memory. When saving fails,
    /// nothing changes.
    pub fn update_config(
        &self,
        f: impl FnOnce(&mut AppConfig),
    ) -> Result<Option<ConfigChange>, CaveError> {
        let old = self.lock_config().clone();
        let mut new = old.clone();
        f(&mut new);
        self.commit_config(&old, new)
    }

    /// Make `new` the config, replacing `old`: save it to the open cave and
    /// reset the agent when the agent config differs from `old`'s. For a
    /// host that changed the in-memory config already (edits it commits
    /// after a pause), `old` is the config before those edits. Returns
    /// `None` when `new` equals `old` (nothing is written).
    pub fn commit_config(
        &self,
        old: &AppConfig,
        mut new: AppConfig,
    ) -> Result<Option<ConfigChange>, CaveError> {
        new.active_cave = None;
        if new == *old {
            return Ok(None);
        }
        match self.with_cave(|cave| cave.save_config(&new)) {
            Ok(()) | Err(CaveError::NoCaveOpen) => {}
            Err(err) => return Err(err),
        }
        *self.lock_config() = new.clone();
        let agent = new.agent != old.agent;
        if agent {
            self.reset_agent();
        }
        Ok(Some(ConfigChange {
            agent,
            rag: rag_index_action(&old.agent.rag, &new.agent.rag),
            theme: new.theme != old.theme,
            fonts: new.markdown_font != old.markdown_font
                || new.reading_font != old.reading_font
                || new.agent_font != old.agent_font,
            config: new,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open::prepare_cave;

    fn rag(enabled: bool, top_n: usize, model: Option<&str>) -> RagConfig {
        RagConfig {
            enabled,
            top_n,
            embedding_model: model.map(str::to_string),
        }
    }

    #[test]
    fn rag_index_action_enabling_rag_rebuilds() {
        assert_eq!(
            rag_index_action(&rag(false, 5, None), &rag(true, 5, None)),
            RagIndexAction::Rebuild
        );
    }

    #[test]
    fn rag_index_action_disabling_rag_disables() {
        assert_eq!(
            rag_index_action(&rag(true, 5, None), &rag(false, 5, None)),
            RagIndexAction::Disable
        );
    }

    #[test]
    fn rag_index_action_model_change_while_enabled_rebuilds() {
        assert_eq!(
            rag_index_action(&rag(true, 5, None), &rag(true, 5, Some("BGESmallENV15"))),
            RagIndexAction::Rebuild
        );
    }

    #[test]
    fn rag_index_action_model_change_while_disabled_is_none() {
        assert_eq!(
            rag_index_action(&rag(false, 5, None), &rag(false, 5, Some("BGESmallENV15"))),
            RagIndexAction::None
        );
    }

    #[test]
    fn rag_index_action_top_n_only_change_is_none() {
        assert_eq!(
            rag_index_action(&rag(true, 5, None), &rag(true, 10, None)),
            RagIndexAction::None
        );
    }

    #[test]
    fn rag_index_action_unchanged_config_is_none() {
        assert_eq!(
            rag_index_action(&rag(true, 5, None), &rag(true, 5, None)),
            RagIndexAction::None
        );
        assert_eq!(
            rag_index_action(&rag(false, 5, None), &rag(false, 5, None)),
            RagIndexAction::None
        );
    }

    fn open_state(dir: &tempfile::TempDir) -> CoreState {
        let state = CoreState::new(AppConfig::default());
        state.install_cave(prepare_cave(dir.path().to_path_buf()).unwrap());
        state
    }

    fn saved_yaml(dir: &tempfile::TempDir) -> String {
        std::fs::read_to_string(dir.path().join(".granit/config.yml")).unwrap()
    }

    #[test]
    fn update_config_saves_to_the_cave_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let state = open_state(&dir);
        let generation = state.agent_generation();

        let change = state
            .update_config(|config| {
                config.daily_note_folder = "Journal".into();
                config.markdown_font.font_size = 18;
            })
            .unwrap()
            .expect("a change");

        assert!(!change.agent, "the agent config is untouched");
        assert!(change.fonts);
        assert!(!change.theme);
        assert_eq!(change.rag, RagIndexAction::None);
        assert_eq!(state.agent_generation(), generation, "the agent is kept");
        let cave = state.lock_cave();
        let saved = cave.as_ref().unwrap().load_config().unwrap();
        assert_eq!(saved, *state.lock_config());
        assert_eq!(saved.daily_note_folder, "Journal");
        assert_eq!(saved.markdown_font.font_size, 18);
    }

    #[test]
    fn update_config_never_writes_active_cave() {
        let dir = tempfile::tempdir().unwrap();
        let state = open_state(&dir);

        state
            .update_config(|config| {
                config.active_cave = Some("/somewhere/else".into());
                config.theme = "light".into();
            })
            .unwrap()
            .expect("a change");

        let yaml = saved_yaml(&dir);
        assert!(!yaml.contains("active_cave"), "{yaml}");
        assert!(yaml.contains("theme: light"), "{yaml}");
        assert!(state.lock_config().active_cave.is_none());
    }

    #[test]
    fn an_unchanged_config_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let state = open_state(&dir);
        let path = dir.path().join(".granit/config.yml");
        std::fs::write(&path, "theme: dracula\n").unwrap();
        state.install_cave(prepare_cave(dir.path().to_path_buf()).unwrap());
        let generation = state.agent_generation();

        // Re-picking what the stored name reads as is no change.
        let change = state
            .update_config(|config| config.set_theme_choice(granit_types::ThemeChoice::Dark))
            .unwrap();

        assert_eq!(change, None);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theme: dracula\n");
        assert_eq!(state.agent_generation(), generation);
    }

    #[test]
    fn an_agent_change_resets_the_agent_and_reports_the_index_action() {
        let dir = tempfile::tempdir().unwrap();
        let state = open_state(&dir);
        let generation = state.agent_generation();

        let change = state
            .update_config(|config| config.agent.rag.enabled = false)
            .unwrap()
            .expect("a change");

        assert!(change.agent);
        assert_eq!(change.rag, RagIndexAction::Disable);
        assert_eq!(state.agent_generation(), generation + 1);

        let change = state
            .update_config(|config| config.agent.rag.enabled = true)
            .unwrap()
            .expect("a change");
        assert_eq!(change.rag, RagIndexAction::Rebuild);
    }

    #[test]
    fn an_invalid_agent_config_is_still_saved() {
        let dir = tempfile::tempdir().unwrap();
        let state = open_state(&dir);

        state
            .update_config(|config| {
                config.agent.providers.push(granit_types::ProviderEntry {
                    name: None,
                    provider: granit_types::ProviderConfig::Anthropic {
                        api_key: String::new(),
                    },
                })
            })
            .unwrap()
            .expect("a change");

        assert!(state.lock_config().agent.validate().is_err());
        assert!(saved_yaml(&dir).contains("anthropic"));
    }

    #[test]
    fn edits_committed_together_reset_the_agent_once() {
        let dir = tempfile::tempdir().unwrap();
        let state = open_state(&dir);
        let generation = state.agent_generation();
        let baseline = state.lock_config().clone();

        // Three keystrokes in a field, made in memory only.
        for key in ["s", "sk", "sk-"] {
            state.lock_config().agent.tool_config.web_search.api_key = Some(key.into());
        }
        assert_eq!(state.agent_generation(), generation);
        assert!(!saved_yaml(&dir).contains("sk-"));

        let current = state.lock_config().clone();
        let change = state.commit_config(&baseline, current).unwrap().unwrap();
        assert!(change.agent);
        assert_eq!(state.agent_generation(), generation + 1);
        assert!(saved_yaml(&dir).contains("api_key: sk-"));
    }

    #[test]
    fn without_a_cave_the_change_stays_in_memory() {
        let state = CoreState::new(AppConfig::default());
        state
            .update_config(|config| config.theme = "dark".into())
            .unwrap()
            .expect("a change");
        assert_eq!(state.lock_config().theme, "dark");
    }
}
