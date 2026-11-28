//! The Notes page: where daily notes go and the template they start from.

use gpui::{App, SharedString};
use gpui_kit::component::setting::{SettingField, SettingGroup, SettingItem, SettingPage};

use super::{getter, SettingsCtx};
use crate::assets::AppIcon;
use crate::host::Host;

pub fn page(ctx: &SettingsCtx, cx: &App) -> SettingPage {
    let core = Host::core(cx);
    let (folders, templates) = core
        .with_cave(|cave| Ok((cave.list_folders()?, cave.list_templates()?)))
        .unwrap_or_default();
    let current = core.lock_config().daily_note_folder.clone();

    let mut folder_options: Vec<(SharedString, SharedString)> = folders
        .into_iter()
        .map(|folder| (folder.clone().into(), folder.into()))
        .collect();
    // The configured folder stays selectable before it exists (the default
    // `Daily` is created with the first daily note).
    if !folder_options
        .iter()
        .any(|(value, _)| value.as_ref() == current)
    {
        folder_options.insert(0, (current.clone().into(), current.into()));
    }
    let mut template_options: Vec<(SharedString, SharedString)> =
        vec![(SharedString::default(), "No template".into())];
    template_options.extend(
        templates
            .into_iter()
            .map(|template| (template.slug.clone().into(), template.slug.into())),
    );

    SettingPage::new("Notes")
        .icon(AppIcon::NotebookPen)
        .resettable(false)
        .group(
            SettingGroup::new()
                .title("Daily notes")
                .item(
                    SettingItem::new(
                        "Folder",
                        SettingField::scrollable_dropdown(
                            folder_options,
                            getter(|config| config.daily_note_folder.clone().into()),
                            ctx.setter(|config, value: SharedString| {
                                config.daily_note_folder = value.to_string();
                            }),
                        ),
                    )
                    .description("Where daily notes are stored, relative to the cave."),
                )
                .item(
                    SettingItem::new(
                        "Template",
                        SettingField::scrollable_dropdown(
                            template_options,
                            getter(|config| {
                                config
                                    .daily_note_template_slug
                                    .clone()
                                    .unwrap_or_default()
                                    .into()
                            }),
                            ctx.setter(|config, value: SharedString| {
                                config.daily_note_template_slug =
                                    (!value.is_empty()).then(|| value.to_string());
                            }),
                        ),
                    )
                    .description("A template from .granit/templates for new daily notes."),
                ),
        )
}
