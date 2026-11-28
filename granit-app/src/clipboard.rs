//! Rich-text clipboard writes. GPUI's clipboard holds strings, images and
//! paths only, with no HTML flavour, so [`write_html`] goes through
//! `arboard` (the crate behind the Tauri app's clipboard plugin), which puts
//! the HTML beside a plain-text alternative.
//!
//! The system clipboard is the [`RichClipboard`] global installed by
//! [`init`]; on Linux the app hosts what it copied, so one `arboard`
//! handle is kept for the app's lifetime. Without the global (tests that
//! install none) or when the system clipboard refuses the write, the plain
//! text goes on GPUI's clipboard instead.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, ClipboardItem, Global};
use log::warn;

/// Puts `html` with its plain-text alternative on a clipboard.
type HtmlWriter = Rc<dyn Fn(&str, &str) -> Result<(), String>>;

/// The clipboard [`write_html`] writes to.
pub struct RichClipboard {
    writer: HtmlWriter,
}

impl Global for RichClipboard {}

/// Install the system clipboard. Call once at startup.
pub fn init(cx: &mut App) {
    let handle: RefCell<Option<arboard::Clipboard>> = RefCell::new(None);
    let writer: HtmlWriter = Rc::new(move |html, plain| {
        let mut handle = handle.borrow_mut();
        if handle.is_none() {
            *handle = Some(arboard::Clipboard::new().map_err(|err| err.to_string())?);
        }
        let clipboard = handle.as_mut().expect("the clipboard was just opened");
        clipboard
            .set_html(html, Some(plain))
            .map_err(|err| err.to_string())
    });
    cx.set_global(RichClipboard { writer });
}

/// Replace the clipboard [`write_html`] writes to, for tests.
#[cfg(test)]
pub fn set_writer(writer: impl Fn(&str, &str) -> Result<(), String> + 'static, cx: &mut App) {
    cx.set_global(RichClipboard {
        writer: Rc::new(writer),
    });
}

/// Put `html` on the clipboard as rich text, with `plain` for targets that
/// take plain text only. Falls back to `plain` alone on GPUI's clipboard
/// when the system clipboard cannot take HTML.
pub fn write_html(html: &str, plain: &str, cx: &mut App) {
    if let Some(clipboard) = cx.try_global::<RichClipboard>() {
        let writer = clipboard.writer.clone();
        match writer(html, plain) {
            Ok(()) => return,
            Err(err) => warn!("Could not copy as rich text, copying plain text: {err}"),
        }
    }
    cx.write_to_clipboard(ClipboardItem::new_string(plain.to_string()));
}
