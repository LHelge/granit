//! The Agent page (conversation limits, the system prompt, RAG and tools)
//! and the Providers page (with masked API keys).
//!
//! The provider and model the chat uses are picked in the chat panel.

use gpui::{div, App, Entity, IntoElement, ParentElement as _, SharedString, Styled as _};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    input::{Textarea, TextareaState},
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage},
    ActiveTheme as _, IconName, Sizable as _,
};
use granit_types::{
    AppConfig, ProviderConfig, ProviderEntry, ToolInfo, DEFAULT_SYSTEM_PROMPT_TEMPLATE,
    EMBEDDING_MODELS,
};

use super::fields::{text_field, TextFieldOptions};
use super::{getter, SettingsCtx};
use crate::assets::AppIcon;
use crate::config_ops;
use crate::host::Host;

/// The `EMBEDDING_MODELS` id standing for the built-in model.
const DEFAULT_EMBEDDING_MODEL: &str = "default";

/// The Agent page: conversation limits, system prompt, RAG and tools.
pub fn page(ctx: &SettingsCtx, prompt: &Entity<TextareaState>, cx: &App) -> SettingPage {
    let config = Host::core(cx).lock_config().clone();
    let tools = granit_core::agent::tools::tool_info_list();
    SettingPage::new("Agent")
        .icon(AppIcon::Bot)
        .resettable(false)
        .group(conversation_group(ctx))
        .group(prompt_group(ctx, prompt))
        .group(rag_group(ctx, &config))
        .group(tools_group(ctx, &config, &tools))
}

/// The Providers page: one group per configured provider.
pub fn providers_page(ctx: &SettingsCtx, cx: &App) -> SettingPage {
    let config = Host::core(cx).lock_config().clone();
    SettingPage::new("Providers")
        .icon(AppIcon::Wrench)
        .resettable(false)
        .group(providers_group(ctx, &config))
        .groups(
            config
                .agent
                .providers
                .iter()
                .enumerate()
                .map(|(index, entry)| provider_group(ctx, index, entry)),
        )
}

fn number(min: f64, max: f64, step: f64) -> NumberFieldOptions {
    NumberFieldOptions { min, max, step }
}

fn conversation_group(ctx: &SettingsCtx) -> SettingGroup {
    SettingGroup::new()
        .title("Conversation")
        .item(
            SettingItem::new(
                "Max history",
                SettingField::number_input(
                    number(1., 10_000., 1.),
                    getter(|config| config.agent.max_history as f64),
                    ctx.typed_setter(|config, value: f64| {
                        config.agent.max_history = value as usize
                    }),
                ),
            )
            .description("Messages kept in the conversation; older ones are dropped."),
        )
        .item(
            SettingItem::new(
                "Max turns",
                SettingField::number_input(
                    number(1., 100., 1.),
                    getter(|config| config.agent.max_turns as f64),
                    ctx.typed_setter(|config, value: f64| config.agent.max_turns = value as usize),
                ),
            )
            .description("Tool-call rounds the agent may take per message."),
        )
}

fn prompt_group(ctx: &SettingsCtx, prompt: &Entity<TextareaState>) -> SettingGroup {
    let editor = prompt.clone();
    let reset = prompt.clone();
    let ctx = ctx.clone();
    SettingGroup::new()
        .title("System prompt")
        .description(
            "Stored in .granit/agent/system.md, saved when you pause typing. Supports the Tera \
             variables mode, tools, icons, skills, rag, today, year, month, day and weekday.",
        )
        .item(SettingItem::render(move |_, _, cx| {
            Textarea::new(&editor)
                .w_full()
                .font_family(cx.theme().mono_font_family.clone())
        }))
        .item(SettingItem::render(move |_, _, _| {
            let reset = reset.clone();
            let ctx = ctx.clone();
            Button::new("reset-system-prompt")
                .outline()
                .small()
                .icon(IconName::Undo2)
                .label("Reset to the default prompt")
                .on_click(move |_, window, cx| {
                    // Undoable in the editor: the text is replaced, not reset.
                    reset.update(cx, |prompt, cx| {
                        prompt.replace_all(DEFAULT_SYSTEM_PROMPT_TEMPLATE, window, cx)
                    });
                    let result =
                        config_ops::write_system_prompt(DEFAULT_SYSTEM_PROMPT_TEMPLATE, cx);
                    ctx.report(result, cx);
                })
        }))
}

fn rag_group(ctx: &SettingsCtx, config: &AppConfig) -> SettingGroup {
    let disabled = !config.agent.rag.enabled;
    let models = EMBEDDING_MODELS
        .iter()
        .map(|(id, label)| (SharedString::from(*id), SharedString::from(*label)))
        .collect();
    SettingGroup::new()
        .title("Embeddings (RAG)")
        .item(
            SettingItem::new(
                "Note context",
                SettingField::switch(
                    getter(|config| config.agent.rag.enabled),
                    ctx.setter(|config, value: bool| config.agent.rag.enabled = value),
                ),
            )
            .description("In Ask mode, add the most relevant notes to every message."),
        )
        .item(
            SettingItem::new(
                "Notes per message",
                SettingField::number_input(
                    number(1., 20., 1.),
                    getter(|config| config.agent.rag.top_n as f64),
                    ctx.typed_setter(|config, value: f64| config.agent.rag.top_n = value as usize),
                ),
            )
            .disabled(disabled),
        )
        .item(
            SettingItem::new(
                "Embedding model",
                SettingField::scrollable_dropdown(
                    models,
                    getter(|config| {
                        config
                            .agent
                            .rag
                            .embedding_model
                            .clone()
                            .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.to_string())
                            .into()
                    }),
                    ctx.setter(|config, value: SharedString| {
                        config.agent.rag.embedding_model =
                            (value.as_ref() != DEFAULT_EMBEDDING_MODEL).then(|| value.to_string());
                    }),
                ),
            )
            .description("Changing the model embeds every note again.")
            .disabled(disabled),
        )
}

fn tools_group(ctx: &SettingsCtx, config: &AppConfig, tools: &[ToolInfo]) -> SettingGroup {
    let mut group = SettingGroup::new()
        .title("Tools")
        .description("Tools the agent may call.");
    for tool in tools {
        let name = tool.name.clone();
        let enabled = !config.agent.disabled_tools.contains(&name);
        group = group.item(
            SettingItem::new(
                SharedString::from(name.clone()),
                SettingField::switch(
                    {
                        let name = name.clone();
                        getter(move |config| !config.agent.disabled_tools.contains(&name))
                    },
                    {
                        let name = name.clone();
                        ctx.setter(move |config, on: bool| {
                            let disabled = &mut config.agent.disabled_tools;
                            disabled.retain(|tool| tool != &name);
                            if !on {
                                disabled.push(name.clone());
                            }
                        })
                    },
                ),
            )
            .description(tool.description.clone()),
        );
        match name.as_str() {
            "web_search" => {
                group = group
                    .item(
                        SettingItem::new(
                            "Brave API key",
                            text_field(
                                "web-search-key",
                                TextFieldOptions {
                                    masked: true,
                                    placeholder: "BSA-…",
                                },
                                getter(|config| {
                                    config
                                        .agent
                                        .tool_config
                                        .web_search
                                        .api_key
                                        .clone()
                                        .unwrap_or_default()
                                }),
                                ctx.typed_setter(|config, key: String| {
                                    config.agent.tool_config.web_search.api_key =
                                        (!key.trim().is_empty()).then_some(key);
                                }),
                            ),
                        )
                        .description("web_search is offered only with a key.")
                        .disabled(!enabled),
                    )
                    .item(
                        SettingItem::new(
                            "Max search results",
                            SettingField::number_input(
                                number(1., 20., 1.),
                                getter(|config| {
                                    config.agent.tool_config.web_search.max_results as f64
                                }),
                                ctx.typed_setter(|config, value: f64| {
                                    config.agent.tool_config.web_search.max_results =
                                        value as usize;
                                }),
                            ),
                        )
                        .disabled(!enabled),
                    );
            }
            "web_fetch" => {
                group = group.item(
                    SettingItem::new(
                        "Max fetched characters",
                        SettingField::number_input(
                            number(1_000., 1_000_000., 1_000.),
                            getter(|config| {
                                config.agent.tool_config.web_fetch.max_output_chars as f64
                            }),
                            ctx.typed_setter(|config, value: f64| {
                                config.agent.tool_config.web_fetch.max_output_chars =
                                    value as usize;
                            }),
                        ),
                    )
                    .disabled(!enabled),
                );
            }
            _ => {}
        }
    }
    group
}

/// The provider list's header: what is wrong with it, and adding one.
fn providers_group(ctx: &SettingsCtx, config: &AppConfig) -> SettingGroup {
    let problem = config.agent.validate().err();
    let ctx = ctx.clone();
    SettingGroup::new()
        .title("Providers")
        .description("The chat panel picks the provider and model to use.")
        .item(SettingItem::render(move |_, _, cx| {
            let ctx = ctx.clone();
            let danger = cx.theme().danger;
            gpui::div()
                .flex()
                .flex_col()
                .gap_2()
                .children(problem.clone().map(|problem| {
                    div()
                        .text_sm()
                        .text_color(danger)
                        .child(format!("The agent will not start: {problem}"))
                }))
                .child(
                    Button::new("add-provider")
                        .outline()
                        .small()
                        .icon(IconName::Plus)
                        .label("Add provider")
                        .on_click(move |_, _, cx| {
                            ctx.update(cx, |config| config.agent.add_provider())
                        }),
                )
                .into_any_element()
        }))
}

/// Read the provider at `index` with `f` (a default when it is gone).
fn read_provider<T: Default + 'static>(
    index: usize,
    f: impl Fn(&ProviderEntry) -> T + 'static,
) -> impl Fn(&App) -> T + 'static {
    getter(move |config| {
        config
            .agent
            .providers
            .get(index)
            .map(&f)
            .unwrap_or_default()
    })
}

/// Change the provider at `index` with `f`, if it is still there (a typed
/// field: committed when typing pauses).
fn write_provider<T: 'static>(
    ctx: &SettingsCtx,
    index: usize,
    f: impl Fn(&mut ProviderEntry, T) + 'static,
) -> impl Fn(T, &mut App) + 'static {
    ctx.typed_setter(move |config, value| {
        if let Some(entry) = config.agent.providers.get_mut(index) {
            f(entry, value);
        }
    })
}

fn provider_group(ctx: &SettingsCtx, index: usize, entry: &ProviderEntry) -> SettingGroup {
    let types = ProviderConfig::TYPES
        .iter()
        .map(|(id, label)| (SharedString::from(*id), SharedString::from(*label)))
        .collect();
    let mut group = SettingGroup::new()
        .title(format!("Provider {}: {}", index + 1, entry.display_name()))
        .item(SettingItem::new(
            "Type",
            SettingField::dropdown(
                types,
                read_provider(index, |entry| {
                    SharedString::from(entry.provider.provider_type().to_string())
                }),
                ctx.setter(move |config, kind: SharedString| {
                    config.agent.set_provider_type(index, &kind);
                }),
            ),
        ))
        .item(SettingItem::new(
            "Name",
            text_field(
                format!("provider-{index}-name"),
                TextFieldOptions {
                    masked: false,
                    placeholder: "Optional",
                },
                read_provider(index, |entry| entry.name.clone().unwrap_or_default()),
                write_provider(ctx, index, |entry, name: String| {
                    entry.name = (!name.trim().is_empty()).then_some(name);
                }),
            ),
        ));
    if entry.provider.base_url().is_some() {
        let placeholder = match entry.provider {
            ProviderConfig::Ollama { .. } => "http://localhost:11434",
            _ => "https://api.openai.com/v1",
        };
        group = group.item(SettingItem::new(
            "Base URL",
            text_field(
                format!("provider-{index}-base-url"),
                TextFieldOptions {
                    masked: false,
                    placeholder,
                },
                read_provider(index, |entry| {
                    entry.provider.base_url().unwrap_or_default().to_string()
                }),
                write_provider(ctx, index, |entry, url: String| {
                    entry.provider.set_base_url(&url)
                }),
            ),
        ));
    }
    if entry.provider.api_key().is_some() {
        group = group.item(SettingItem::new(
            "API key",
            text_field(
                format!("provider-{index}-api-key"),
                TextFieldOptions {
                    masked: true,
                    placeholder: "API key",
                },
                read_provider(index, |entry| {
                    entry.provider.api_key().unwrap_or_default().to_string()
                }),
                write_provider(ctx, index, |entry, key: String| {
                    entry.provider.set_api_key(&key)
                }),
            ),
        ));
    }
    let ctx = ctx.clone();
    group.item(SettingItem::render(move |_, _, _| {
        let ctx = ctx.clone();
        Button::new(SharedString::from(format!("remove-provider-{index}")))
            .ghost()
            .small()
            .icon(AppIcon::Trash)
            .label("Remove provider")
            .on_click(move |_, _, cx| ctx.update(cx, |config| config.agent.remove_provider(index)))
    }))
}
