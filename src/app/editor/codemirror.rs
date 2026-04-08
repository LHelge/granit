//! Wasm-bindgen bindings to the `window.GranitEditor` JavaScript API
//! provided by `dist/codemirror.js` (built from `js/editor.ts`).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = GranitEditor, js_name = create)]
    fn cm_create(element: &web_sys::HtmlElement, config: &JsValue) -> u32;

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = setContent)]
    fn cm_set_content(handle: u32, content: &str);

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = getContent)]
    fn cm_get_content(handle: u32) -> String;

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = focus)]
    fn cm_focus(handle: u32);

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = setFont)]
    fn cm_set_font(handle: u32, family: &str, size: &str);

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = setReadOnly)]
    fn cm_set_read_only(handle: u32, read_only: bool);

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = setSlugs)]
    fn cm_set_slugs(handle: u32, slugs: js_sys::Array);

    #[wasm_bindgen(js_namespace = GranitEditor, js_name = destroy)]
    fn cm_destroy(handle: u32);
}

/// Opaque handle to a CodeMirror editor instance.
#[derive(Clone, Copy)]
pub struct EditorHandle(u32);

/// Create a new CodeMirror editor inside `element`.
///
/// `on_change` fires when the document content changes (user edits).
/// `on_selection_change` fires when the selection changes, with the
/// currently selected text (empty string if no selection).
///
/// Both callbacks are leaked (via `Closure::into_js_value`) so they
/// live for the lifetime of the editor. Call `destroy()` to clean up.
pub fn create(
    element: &web_sys::HtmlElement,
    content: &str,
    font_family: &str,
    font_size: &str,
    slugs: &[String],
    on_change: impl Fn(String) + 'static,
    on_selection_change: impl Fn(String) + 'static,
) -> EditorHandle {
    let on_change_cb = Closure::wrap(Box::new(on_change) as Box<dyn Fn(String)>);
    let on_sel_cb = Closure::wrap(Box::new(on_selection_change) as Box<dyn Fn(String)>);

    let js_slugs: js_sys::Array = slugs
        .iter()
        .map(|s| wasm_bindgen::JsValue::from_str(s))
        .collect();

    let config = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &config,
        &"content".into(),
        &wasm_bindgen::JsValue::from_str(content),
    );
    let _ = js_sys::Reflect::set(
        &config,
        &"fontFamily".into(),
        &wasm_bindgen::JsValue::from_str(font_family),
    );
    let _ = js_sys::Reflect::set(
        &config,
        &"fontSize".into(),
        &wasm_bindgen::JsValue::from_str(font_size),
    );
    let _ = js_sys::Reflect::set(&config, &"slugs".into(), &js_slugs.into());
    let _ = js_sys::Reflect::set(&config, &"onChange".into(), on_change_cb.as_ref());
    let _ = js_sys::Reflect::set(&config, &"onSelectionChange".into(), on_sel_cb.as_ref());

    let handle = cm_create(element, &config.into());

    // Leak closures — they live until `destroy()` is called.
    on_change_cb.forget();
    on_sel_cb.forget();

    EditorHandle(handle)
}

/// Replace the editor document content (suppresses onChange).
pub fn set_content(handle: EditorHandle, content: &str) {
    cm_set_content(handle.0, content);
}

/// Read the current document content.
#[allow(dead_code)]
pub fn get_content(handle: EditorHandle) -> String {
    cm_get_content(handle.0)
}

/// Focus the editor.
pub fn focus(handle: EditorHandle) {
    cm_focus(handle.0);
}

/// Update the editor font family and size.
pub fn set_font(handle: EditorHandle, family: &str, size: &str) {
    cm_set_font(handle.0, family, size);
}

/// Toggle read-only mode.
#[allow(dead_code)]
pub fn set_read_only(handle: EditorHandle, read_only: bool) {
    cm_set_read_only(handle.0, read_only);
}

/// Update the slug list for wiki-link autocompletion.
pub fn set_slugs(handle: EditorHandle, slugs: &[String]) {
    let js_slugs: js_sys::Array = slugs
        .iter()
        .map(|s| wasm_bindgen::JsValue::from_str(s))
        .collect();
    cm_set_slugs(handle.0, js_slugs);
}

/// Destroy the editor instance and free resources.
pub fn destroy(handle: EditorHandle) {
    cm_destroy(handle.0);
}
