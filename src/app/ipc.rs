use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use granit_types::{AppConfig, FontConfig, Note, NoteMeta, RenderedNote};

// ── Tauri IPC binding ──────────────────────────────────────────────

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub(crate) async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// ── Argument structs ───────────────────────────────────────────────

#[derive(Serialize)]
struct CreateNoteArgs {
    name: String,
    folder: Option<String>,
}

#[derive(Serialize)]
struct FolderPathArg {
    path: String,
}

#[derive(Serialize)]
struct OpenCaveArgs {
    path: String,
}

#[derive(Serialize)]
struct OpenDialogOptions {
    directory: bool,
    multiple: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateNoteArgs {
    old_name: String,
    new_name: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveConfigArgs {
    agent: granit_types::AgentConfig,
    markdown_font: FontConfig,
    reading_font: FontConfig,
    agent_font: FontConfig,
}

// ── Helpers ───────────────────────────────────────────────────────

fn js_err_to_string(e: JsValue) -> String {
    // Tauri 2 returns errors as a plain string or as `{ message: "..." }`.
    if let Some(s) = e.as_string() {
        return s;
    }
    js_sys::Reflect::get(&e, &JsValue::from_str("message"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| {
            js_sys::JSON::stringify(&e)
                .ok()
                .and_then(|s| s.as_string())
                .unwrap_or_else(|| "Unknown IPC error".to_string())
        })
}

// ── IPC helpers ────────────────────────────────────────────────────

pub async fn fetch_config() -> Result<AppConfig, String> {
    let val = invoke("get_config", JsValue::NULL)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn list_system_fonts() -> Result<Vec<String>, String> {
    let val = invoke("list_system_fonts", JsValue::NULL)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn open_cave(path: &str) -> Result<AppConfig, String> {
    let args = serde_wasm_bindgen::to_value(&OpenCaveArgs {
        path: path.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("open_cave", args).await.map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn fetch_notes() -> Result<Vec<NoteMeta>, String> {
    let val = invoke("list_notes", JsValue::NULL)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn create_note(name: &str, folder: Option<&str>) -> Result<NoteMeta, String> {
    let args = serde_wasm_bindgen::to_value(&CreateNoteArgs {
        name: name.to_string(),
        folder: folder.map(str::to_string),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("create_note", args)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

#[allow(dead_code)]
pub async fn create_folder(path: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&FolderPathArg {
        path: path.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    invoke("create_folder", args)
        .await
        .map_err(js_err_to_string)?;
    Ok(())
}

pub async fn delete_note(slug: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
    }
    let args = serde_wasm_bindgen::to_value(&Args {
        name: slug.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    invoke("delete_note", args)
        .await
        .map_err(js_err_to_string)?;
    Ok(())
}

pub async fn delete_folder(path: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&FolderPathArg {
        path: path.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    invoke("delete_folder", args)
        .await
        .map_err(js_err_to_string)?;
    Ok(())
}

pub async fn move_note(slug: &str, destination: Option<&str>) -> Result<NoteMeta, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
        destination: Option<String>,
    }
    let args = serde_wasm_bindgen::to_value(&Args {
        name: slug.to_string(),
        destination: destination.map(str::to_string),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("move_note", args).await.map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn move_folder(source: &str, destination: Option<&str>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        source: String,
        destination: Option<String>,
    }
    let args = serde_wasm_bindgen::to_value(&Args {
        source: source.to_string(),
        destination: destination.map(str::to_string),
    })
    .map_err(|e| format!("{e}"))?;
    invoke("move_folder", args)
        .await
        .map_err(js_err_to_string)?;
    Ok(())
}

pub async fn read_note(name: &str) -> Result<Note, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
    }
    let args = serde_wasm_bindgen::to_value(&Args {
        name: name.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("read_note", args).await.map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn render_note(name: &str) -> Result<RenderedNote, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
    }
    let args = serde_wasm_bindgen::to_value(&Args {
        name: name.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("render_note", args)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn update_note(
    old_name: &str,
    new_name: &str,
    content: &str,
) -> Result<NoteMeta, String> {
    let args = serde_wasm_bindgen::to_value(&UpdateNoteArgs {
        old_name: old_name.to_string(),
        new_name: new_name.to_string(),
        content: content.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("update_note", args)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn save_config(
    agent: granit_types::AgentConfig,
    markdown_font: FontConfig,
    reading_font: FontConfig,
    agent_font: FontConfig,
) -> Result<AppConfig, String> {
    let args = serde_wasm_bindgen::to_value(&SaveConfigArgs {
        agent,
        markdown_font,
        reading_font,
        agent_font,
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("save_config", args)
        .await
        .map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

#[derive(Serialize)]
struct SecretKeyArg {
    key: String,
}

#[derive(Serialize)]
struct SetSecretArgs {
    key: String,
    value: String,
}

/// Check whether a secret key is configured. Returns `Some(true)` if set, `None` if not.
pub async fn get_secret(key: &str) -> Result<Option<bool>, String> {
    let args = serde_wasm_bindgen::to_value(&SecretKeyArg {
        key: key.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("get_secret", args).await.map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

/// Write a secret key to the global secrets.env file.
pub async fn set_secret(key: &str, value: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&SetSecretArgs {
        key: key.to_string(),
        value: value.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    invoke("set_secret", args)
        .await
        .map(|_| ())
        .map_err(js_err_to_string)
}

pub async fn pick_folder() -> Option<String> {
    let tauri =
        js_sys::Reflect::get(&web_sys::window()?.into(), &JsValue::from_str("__TAURI__")).ok()?;
    let dialog = js_sys::Reflect::get(&tauri, &JsValue::from_str("dialog")).ok()?;
    let open_fn = js_sys::Reflect::get(&dialog, &JsValue::from_str("open")).ok()?;
    let open_fn = js_sys::Function::from(open_fn);

    let opts = serde_wasm_bindgen::to_value(&OpenDialogOptions {
        directory: true,
        multiple: false,
    })
    .ok()?;

    let promise = open_fn.call1(&JsValue::NULL, &opts).ok()?;
    let result: JsValue = JsFuture::from(js_sys::Promise::from(promise)).await.ok()?;
    result.as_string()
}

// ── Agent IPC ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct RenderMarkdownArgs {
    content: String,
}

/// Render a markdown string to HTML via the backend.
pub async fn render_markdown(content: &str) -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&RenderMarkdownArgs {
        content: content.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("render_markdown", args)
        .await
        .map_err(js_err_to_string)?;
    val.as_string()
        .ok_or_else(|| "invalid response".to_string())
}

#[derive(Serialize)]
struct SendMessageArgs {
    msg: String,
}

/// Invoke the backend `send_message` command. Returns immediately; streaming
/// tokens arrive via Tauri events (`agent:stream-chunk`, `agent:stream-done`,
/// `agent:stream-error`).
pub async fn send_message(msg: &str) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&SendMessageArgs {
        msg: msg.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    invoke("send_message", args)
        .await
        .map(|_| ())
        .map_err(js_err_to_string)
}

/// Register a closure to be called for each streaming text chunk.
/// Returns an [`EventHandle`] — drop it to remove the listener.
pub async fn listen_stream_chunk(cb: impl Fn(String) + 'static) -> Option<EventHandle> {
    listen_event("agent:stream-chunk", move |payload: JsValue| {
        // Tauri 2 event payload: { payload: <data> }
        let text = js_sys::Reflect::get(&payload, &JsValue::from_str("payload"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        cb(text);
    })
    .await
}

/// Register a closure called when streaming is done.
pub async fn listen_stream_done(cb: impl Fn() + 'static) -> Option<EventHandle> {
    listen_event("agent:stream-done", move |_| cb()).await
}

/// Register a closure called on a streaming error.
pub async fn listen_stream_error(cb: impl Fn(String) + 'static) -> Option<EventHandle> {
    listen_event("agent:stream-error", move |payload: JsValue| {
        let msg = js_sys::Reflect::get(&payload, &JsValue::from_str("payload"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Unknown error".to_string());
        cb(msg);
    })
    .await
}

/// Handle returned by `listen_event`. Dropping this calls the unlisten
/// function and frees the JS closure — no leaked memory.
pub struct EventHandle {
    _closure: Closure<dyn Fn(JsValue)>,
    unlisten: js_sys::Function,
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        let _ = self.unlisten.call0(&JsValue::NULL);
    }
}

/// Internal helper: call `window.__TAURI__.event.listen(event, handler)`.
/// Returns an [`EventHandle`] that keeps the closure alive. Dropping
/// the handle calls the Tauri unlisten function and frees the closure.
async fn listen_event(event: &str, cb: impl Fn(JsValue) + 'static) -> Option<EventHandle> {
    let win: JsValue = web_sys::window()?.into();
    let tauri = js_sys::Reflect::get(&win, &JsValue::from_str("__TAURI__")).ok()?;
    let event_ns = js_sys::Reflect::get(&tauri, &JsValue::from_str("event")).ok()?;
    let listen_fn = js_sys::Reflect::get(&event_ns, &JsValue::from_str("listen")).ok()?;
    let listen_fn = js_sys::Function::from(listen_fn);

    let handler = Closure::wrap(Box::new(cb) as Box<dyn Fn(JsValue)>);
    let promise = listen_fn
        .call2(&event_ns, &JsValue::from_str(event), handler.as_ref())
        .ok()?;

    let unlisten = JsFuture::from(js_sys::Promise::from(promise)).await.ok()?;
    Some(EventHandle {
        _closure: handler,
        unlisten: js_sys::Function::from(unlisten),
    })
}
