//! Setting fields GPUI Kit's `SettingField` does not offer: a text input
//! bound to a config value that can be masked (API keys), with a
//! placeholder.

use std::rc::Rc;

use crate::config_ops;
use gpui::{
    prelude::FluentBuilder as _, App, AppContext as _, Axis, Entity, SharedString, Styled,
    Subscription,
};
use gpui_kit::component::{
    input::{Input, InputEvent, InputState},
    setting::SettingField,
    Sizable as _,
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
                            if let Err(err) = config_ops::flush_config(cx) {
                                log::warn!("Could not save the settings: {err}");
                                config_ops::ConfigEvents::entity(cx)
                                    .update(cx, |_, cx| cx.emit(config_ops::ConfigSaveFailed(err)));
                            }
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
