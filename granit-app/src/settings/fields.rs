//! Setting fields GPUI Kit's `SettingField` does not offer: a text input
//! bound to a config value that can be masked (API keys), with a
//! placeholder, and a number input. Both commit a typed edit on Enter and
//! blur, which the Kit's own fields cannot.

use std::rc::Rc;

use crate::config_ops;
use gpui::{
    prelude::FluentBuilder as _, App, AppContext as _, Axis, Entity, SharedString, Styled,
    Subscription,
};
use gpui_kit::component::{
    input::{Input, InputEvent, InputState, NumberInput},
    setting::{NumberFieldOptions, SettingField},
    Disableable as _, Sizable as _,
};

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
