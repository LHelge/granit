//! The Appearance page: theme, editor font and chat font.

use gpui::SharedString;
use gpui_kit::component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage,
};
use granit_types::{AppConfig, FontConfig, ThemeChoice};

use super::fields::{font_field, number_field};
use super::{getter, SettingsCtx};
use crate::assets::AppIcon;

/// The smallest and largest font size offered.
const FONT_SIZES: (f64, f64) = (8., 48.);

pub fn page(ctx: &SettingsCtx, fonts: &[SharedString]) -> SettingPage {
    let themes = ThemeChoice::ALL
        .into_iter()
        .map(|choice| (choice.config_value().into(), choice.label().into()))
        .collect();
    SettingPage::new("Appearance")
        .icon(AppIcon::Palette)
        .resettable(false)
        .group(
            SettingGroup::new().title("Theme").item(
                SettingItem::new(
                    "Theme",
                    SettingField::dropdown(
                        themes,
                        getter(|config| config.theme_choice().config_value().into()),
                        ctx.setter(|config, value: SharedString| {
                            let choice = ThemeChoice::ALL
                                .into_iter()
                                .find(|choice| choice.config_value() == value.as_ref())
                                .unwrap_or_default();
                            config.set_theme_choice(choice);
                        }),
                    ),
                )
                .description("Catppuccin Latte or Frappé, or whichever matches the system."),
            ),
        )
        .group(font_group(
            ctx,
            fonts,
            "Note font",
            "The text of notes; code in them stays monospace.",
            |config| &config.reading_font,
            |config| &mut config.reading_font,
        ))
        .group(font_group(
            ctx,
            fonts,
            "Chat font",
            "The agent chat panel's text.",
            |config| &config.agent_font,
            |config| &mut config.agent_font,
        ))
}

/// The family and size of one font setting.
fn font_group(
    ctx: &SettingsCtx,
    fonts: &[SharedString],
    title: &'static str,
    description: &'static str,
    font: fn(&AppConfig) -> &FontConfig,
    font_mut: fn(&mut AppConfig) -> &mut FontConfig,
) -> SettingGroup {
    SettingGroup::new()
        .title(title)
        .description(description)
        .item(SettingItem::new(
            "Family",
            font_field(
                format!("font-family-{title}"),
                format!("{title} family"),
                fonts.to_vec(),
                getter(move |config| font(config).clone()),
                ctx.setter(move |config, value: SharedString| {
                    font_mut(config).font_family = value.to_string();
                }),
            ),
        ))
        .item(SettingItem::new(
            "Size",
            number_field(
                format!("font-size-{title}"),
                format!("{title} size"),
                NumberFieldOptions {
                    min: FONT_SIZES.0,
                    max: FONT_SIZES.1,
                    step: 1.,
                },
                getter(move |config| f64::from(font(config).font_size)),
                ctx.typed_setter(move |config, value: f64| {
                    font_mut(config).font_size = value.round() as u8;
                }),
            ),
        ))
}
