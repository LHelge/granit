//! Setting fields GPUI Kit's `SettingField` does not offer: a text input
//! bound to a config value that can be masked (API keys), with a
//! placeholder, and a number input. Both commit a typed edit on Enter and
//! blur, which the Kit's own fields cannot. And a font picker
//! ([`font_field`]): a searchable list of fonts, each drawn in itself, with
//! a preview line.

use std::rc::Rc;

use crate::config_ops;
use crate::theme::{self, MONO_FONT, SANS_FONT};
use gpui::{
    div, prelude::FluentBuilder as _, px, AnyElement, App, AppContext as _, Axis, Entity,
    IntoElement, ParentElement as _, SharedString, Styled, Subscription, Window,
};
use gpui_kit::component::{
    input::{Input, InputEvent, InputState, NumberInput},
    searchable_list::{SearchableListItem, SearchableVec},
    select::{Select, SelectEvent, SelectState},
    setting::{NumberFieldOptions, SettingField},
    v_flex, ActiveTheme as _, Disableable as _, IndexPath, Sizable as _, Theme,
};
use granit_types::FontConfig;

/// The inputs [`text_field`] created on this thread, by key, for tests.
#[cfg(test)]
pub mod probe {
    use std::cell::RefCell;

    use gpui::{Entity, SharedString};
    use gpui_kit::component::input::InputState;

    thread_local! {
        static INPUTS: RefCell<Vec<(SharedString, Entity<InputState>)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(key: SharedString, input: Entity<InputState>) {
        INPUTS.with(|inputs| inputs.borrow_mut().push((key, input)));
    }

    /// The last input created for `key`.
    pub fn input(key: &str) -> Option<Entity<InputState>> {
        INPUTS.with(|inputs| {
            inputs
                .borrow()
                .iter()
                .rev()
                .find(|(k, _)| k.as_ref() == key)
                .map(|(_, input)| input.clone())
        })
    }

    thread_local! {
        static FONTS: RefCell<Vec<(SharedString, Entity<super::FontSelect>)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record_font(key: SharedString, select: Entity<super::FontSelect>) {
        FONTS.with(|fonts| fonts.borrow_mut().push((key, select)));
    }

    /// The last font picker created for `key`.
    pub fn font(key: &str) -> Option<Entity<super::FontSelect>> {
        FONTS.with(|fonts| {
            fonts
                .borrow()
                .iter()
                .rev()
                .find(|(k, _)| k.as_ref() == key)
                .map(|(_, select)| select.clone())
        })
    }
}

/// The input of a [`text_field`], kept across renders by its key.
struct BoundInput {
    input: Entity<InputState>,
    _subscription: Subscription,
}

/// How a [`text_field`] looks.
#[derive(Clone, Copy, Default)]
pub struct TextFieldOptions {
    /// Draw the text as dots, with a button that shows it (API keys).
    pub masked: bool,
    pub placeholder: &'static str,
}

/// A text input showing `get` and calling `set` on every edit (pass a
/// [`super::SettingsCtx::typed_setter`], which defers the save); Enter and
/// leaving the field commit at once. `key` names
/// its state, so it must be unique in the view (e.g. include a provider's
/// index). The input follows the value when it changes elsewhere.
pub fn text_field(
    key: impl Into<SharedString>,
    options: TextFieldOptions,
    get: impl Fn(&App) -> String + 'static,
    set: impl Fn(String, &mut App) + 'static,
) -> SettingField<SharedString> {
    let key: SharedString = key.into();
    let get = Rc::new(get);
    let set = Rc::new(set);
    SettingField::render(move |render, window, cx| {
        let value = SharedString::from(get(cx));
        let state = window.use_keyed_state(key.clone(), cx, {
            let value = value.clone();
            let set = set.clone();
            #[cfg(test)]
            let key = key.clone();
            move |window, cx| {
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .masked(options.masked)
                        .placeholder(options.placeholder)
                        .default_value(value)
                });
                #[cfg(test)]
                probe::record(key.clone(), input.clone());
                let _subscription = cx.subscribe(
                    &input,
                    move |_, input, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            let text = input.read(cx).value().to_string();
                            set(text, cx);
                        }
                        // Leaving the field or Enter commits a typed edit now.
                        InputEvent::Blur | InputEvent::PressEnter { .. } => {
                            config_ops::commit_edits(cx)
                        }
                        InputEvent::Focus => {}
                    },
                );
                BoundInput {
                    input,
                    _subscription,
                }
            }
        });
        let input = state.read(cx).input.clone();
        if input.read(cx).value() != value {
            input.update(cx, |input, cx| input.set_value(value.clone(), window, cx));
        }
        Input::new(&input)
            .when(options.masked, |input| input.mask_toggle())
            .disabled(render.is_disabled())
            .with_size(render.size())
            .map(|this| {
                if render.layout() == Axis::Horizontal {
                    this.w_64()
                } else {
                    this.w_full()
                }
            })
    })
}

/// The input of a [`number_field`], kept across renders by its key.
struct BoundNumber {
    input: Entity<InputState>,
    /// The value last read from or given to the config: the input follows
    /// the config only when this changes, so typing an intermediate like
    /// `1.` is never overwritten.
    value: f64,
    _subscription: Subscription,
}

/// A number input (with steppers) showing `get` and calling `set` with
/// every edit that parses, clamped to `options`' range (pass a
/// [`super::SettingsCtx::typed_setter`], which defers the save). Enter and
/// leaving the field commit at once, as in [`text_field`]. The Kit's
/// `SettingField::number_input` offers no hook for either. `key` names its
/// state, so it must be unique in the view; `label` is the input's
/// accessible name (the setting's title, as the Kit's field uses).
///
/// Styles set on the returned field are not applied: the Kit's `render`
/// fields drop them (none of ours sets any).
pub fn number_field(
    key: impl Into<SharedString>,
    label: impl Into<SharedString>,
    options: NumberFieldOptions,
    get: impl Fn(&App) -> f64 + 'static,
    set: impl Fn(f64, &mut App) + 'static,
) -> SettingField<SharedString> {
    let key: SharedString = key.into();
    let label: SharedString = label.into();
    let get = Rc::new(get);
    let set = Rc::new(set);
    SettingField::render(move |render, window, cx| {
        let value = get(cx);
        let state = window.use_keyed_state(key.clone(), cx, {
            let set = set.clone();
            let options = options.clone();
            #[cfg(test)]
            let key = key.clone();
            move |window, cx| {
                // Stepping and bounds belong to the input itself: `+`/`-`
                // and Up/Down step, and out-of-range text is clamped on blur.
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value(value.to_string())
                        .step(options.step)
                        .min(options.min)
                        .max(options.max)
                });
                #[cfg(test)]
                probe::record(key.clone(), input.clone());
                let _subscription = cx.subscribe(
                    &input,
                    move |state: &mut BoundNumber, input, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            let text = input.read(cx).value().to_string();
                            // Unparsable intermediates ("-", "") wait for the
                            // next keystroke.
                            let Ok(parsed) = text.parse::<f64>() else {
                                return;
                            };
                            let clamped = parsed.clamp(options.min, options.max);
                            if clamped != state.value {
                                state.value = clamped;
                                set(clamped, cx);
                            }
                        }
                        InputEvent::Blur | InputEvent::PressEnter { .. } => {
                            config_ops::commit_edits(cx)
                        }
                        InputEvent::Focus => {}
                    },
                );
                BoundNumber {
                    input,
                    value,
                    _subscription,
                }
            }
        });
        // The config changed elsewhere (or a commit rounded the value).
        if state.read(cx).value != value {
            state.update(cx, |state, cx| {
                state.value = value;
                state.input.update(cx, |input, cx| {
                    input.set_value(SharedString::from(value.to_string()), window, cx)
                });
            });
        }
        let input = state.read(cx).input.clone();
        NumberInput::new(&input)
            .accessibility_label(label.clone())
            .disabled(render.is_disabled())
            .with_size(render.size())
            .map(|this| {
                if render.layout() == Axis::Horizontal {
                    this.w_32()
                } else {
                    this.w_full()
                }
            })
    })
}

/// The text of a [`font_field`]'s preview line.
pub const FONT_PREVIEW: &str = "The quick brown fox jumps over the lazy dog";

/// A font family offered by a [`font_field`]: the value stored in the
/// config, its label and the family it is drawn in (the theme's fonts for
/// the CSS generics).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontChoice {
    value: SharedString,
    label: SharedString,
    family: SharedString,
}

impl FontChoice {
    /// The label, drawn in the font itself.
    fn sample(&self) -> impl IntoElement {
        div()
            .font_family(self.family.clone())
            .child(self.label.clone())
    }
}

impl SearchableListItem for FontChoice {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(self.sample().into_any_element())
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.sample()
    }

    fn value(&self) -> &SharedString {
        &self.value
    }

    /// The label or the stored name contains the query, ignoring case.
    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.label.to_lowercase().contains(&query) || self.value.to_lowercase().contains(&query)
    }
}

/// The families a [`font_field`] offers: the theme's UI and monospace
/// fonts (stored as the CSS generics `sans-serif` and `monospace`, which
/// the Tauri app reads too), the stored family when it is neither of
/// those nor installed (another generic such as `serif`, or a font since
/// removed), then the installed `fonts`.
pub fn font_choices(fonts: &[SharedString], current: &str, theme: &Theme) -> Vec<FontChoice> {
    let resolve = |value: &str| {
        let font = FontConfig {
            font_family: value.to_string(),
            font_size: 0,
        };
        theme::resolve_font(&font, theme).0
    };
    let choice = |value: &str, label: &str| FontChoice {
        value: value.to_string().into(),
        label: label.to_string().into(),
        family: resolve(value),
    };
    let mut choices = vec![
        choice(SANS_FONT, "Theme font"),
        choice(MONO_FONT, "Theme monospace font"),
    ];
    let current = current.trim();
    let known = current.is_empty()
        || choices.iter().any(|c| c.value.as_ref() == current)
        || fonts.iter().any(|font| font.as_ref() == current);
    if !known {
        choices.push(choice(current, current));
    }
    choices.extend(fonts.iter().map(|font| choice(font, font)));
    choices
}

/// The Kit select a [`font_field`] draws.
pub type FontSelect = SelectState<SearchableVec<FontChoice>>;

/// The select of a [`font_field`], kept across renders by its key.
struct BoundFont {
    select: Entity<FontSelect>,
    /// The family last read from the config: the selection follows the
    /// config only when this changes, so it never clears a search query
    /// being typed.
    value: SharedString,
    _subscription: Subscription,
}

/// A font picker: a searchable list of `fonts` (and the theme's fonts, see
/// [`font_choices`]), each drawn in itself, with the chosen family shown in
/// itself and a preview line in the configured family and size below.
/// `get` reads the font setting, `set` stores a picked family (pass a
/// [`super::SettingsCtx::setter`]: a pick saves at once and the note or
/// chat redraws in it). `key` names its state, so it must be unique in the
/// view; `label` is the select's accessible name.
pub fn font_field(
    key: impl Into<SharedString>,
    label: impl Into<SharedString>,
    fonts: Vec<SharedString>,
    get: impl Fn(&App) -> FontConfig + 'static,
    set: impl Fn(SharedString, &mut App) + 'static,
) -> SettingField<SharedString> {
    let key: SharedString = key.into();
    let label: SharedString = label.into();
    let fonts = Rc::new(fonts);
    let set = Rc::new(set);
    SettingField::render(move |render, window, cx| {
        let font = get(cx);
        let value = SharedString::from(font.font_family.trim().to_string());
        let state = window.use_keyed_state(key.clone(), cx, {
            let fonts = fonts.clone();
            let set = set.clone();
            let value = value.clone();
            #[cfg(test)]
            let key = key.clone();
            move |window, cx| {
                let choices = font_choices(&fonts, &value, cx.theme());
                let selected = choices
                    .iter()
                    .position(|choice| choice.value == value)
                    .map(|row| IndexPath::default().row(row));
                let select = cx.new(|cx| {
                    SelectState::new(SearchableVec::new(choices), selected, window, cx)
                        .searchable(true)
                });
                #[cfg(test)]
                probe::record_font(key.clone(), select.clone());
                let _subscription = cx.subscribe(
                    &select,
                    move |_, _, event: &SelectEvent<SearchableVec<FontChoice>>, cx| {
                        let SelectEvent::Confirm(Some(family)) = event else {
                            return;
                        };
                        set(family.clone(), cx);
                    },
                );
                BoundFont {
                    select,
                    value,
                    _subscription,
                }
            }
        });
        // The config changed elsewhere: show its family as chosen, adding
        // it to the list when it is not there.
        if state.read(cx).value != value {
            let choices = font_choices(&fonts, &value, cx.theme());
            state.update(cx, |state, cx| {
                state.value = value.clone();
                state.select.update(cx, |select, cx| {
                    select.set_items(SearchableVec::new(choices), window, cx);
                    select.set_selected_value(&value, window, cx);
                });
            });
        }
        let select = state.read(cx).select.clone();
        let (family, size) = theme::resolve_font(&font, cx.theme());
        v_flex()
            .gap_1()
            .map(|this| {
                if render.layout() == Axis::Horizontal {
                    this.w_64()
                } else {
                    this.w_full()
                }
            })
            .child(
                Select::new(&select)
                    .accessibility_label(label.clone())
                    .search_placeholder("Search fonts…")
                    .menu_max_h(px(320.))
                    .disabled(render.is_disabled())
                    .with_size(render.size()),
            )
            .child(
                div()
                    .truncate()
                    .font_family(family)
                    .text_size(size)
                    .text_color(cx.theme().muted_foreground)
                    .child(FONT_PREVIEW),
            )
    })
}
