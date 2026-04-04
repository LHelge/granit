use granit_types::{
    AppConfig, FontConfig, Note, NoteMeta, RenderedNote, SidebarConfig, ToolCallInfo, ToolInfo,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// ── Tauri IPC binding ──────────────────────────────────────────────

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub(crate) async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// ── Generic invoke helpers ─────────────────────────────────────────

fn js_err_to_string(e: JsValue) -> String {
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

/// Invoke a Tauri command with typed args and a typed return value.
async fn invoke_cmd<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Result<R, String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(|e| format!("{e}"))?;
    let val = invoke(cmd, args).await.map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

/// Invoke a Tauri command with no args and a typed return value.
async fn invoke_no_args<R: DeserializeOwned>(cmd: &str) -> Result<R, String> {
    let val = invoke(cmd, JsValue::NULL).await.map_err(js_err_to_string)?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

/// Invoke a Tauri command with typed args and no return value.
async fn invoke_unit<A: Serialize>(cmd: &str, args: &A) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(|e| format!("{e}"))?;
    invoke(cmd, args).await.map_err(js_err_to_string)?;
    Ok(())
}

// ── Config ─────────────────────────────────────────────────────────

pub async fn fetch_config() -> Result<AppConfig, String> {
    invoke_no_args("get_config").await
}

pub async fn list_system_fonts() -> Result<Vec<String>, String> {
    invoke_no_args("list_system_fonts").await
}

pub async fn save_config(
    agent: granit_types::AgentConfig,
    markdown_font: FontConfig,
    reading_font: FontConfig,
    agent_font: FontConfig,
    daily_note_folder: String,
) -> Result<AppConfig, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        agent: granit_types::AgentConfig,
        markdown_font: FontConfig,
        reading_font: FontConfig,
        agent_font: FontConfig,
        daily_note_folder: String,
    }
    invoke_cmd(
        "save_config",
        &Args {
            agent,
            markdown_font,
            reading_font,
            agent_font,
            daily_note_folder,
        },
    )
    .await
}

pub async fn save_sidebar_state(
    sidebar: SidebarConfig,
    agent_panel: SidebarConfig,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        sidebar: SidebarConfig,
        agent_panel: SidebarConfig,
    }
    invoke_unit(
        "save_sidebar_state",
        &Args {
            sidebar,
            agent_panel,
        },
    )
    .await
}

// ── Provider / Model selection ──────────────────────────────────────

pub async fn list_providers() -> Result<Vec<granit_types::ProviderInfo>, String> {
    invoke_no_args("list_providers").await
}

pub async fn select_provider(index: usize) -> Result<AppConfig, String> {
    invoke_cmd("select_provider", &HashMap::from([("index", index)])).await
}

pub async fn list_models() -> Result<Vec<granit_types::ModelInfo>, String> {
    invoke_no_args("list_models").await
}

pub async fn select_model(model_id: &str) -> Result<AppConfig, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args<'a> {
        model_id: &'a str,
    }
    invoke_cmd("select_model", &Args { model_id }).await
}

// ── Cave ───────────────────────────────────────────────────────────

pub async fn open_cave(path: &str) -> Result<AppConfig, String> {
    invoke_cmd("open_cave", &HashMap::from([("path", path)])).await
}

pub async fn fetch_notes() -> Result<Vec<NoteMeta>, String> {
    invoke_no_args("list_notes").await
}

pub async fn fetch_folders() -> Result<Vec<String>, String> {
    invoke_no_args("list_folders").await
}

// ── Notes ──────────────────────────────────────────────────────────

pub async fn create_note(name: &str, folder: Option<&str>) -> Result<NoteMeta, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        name: &'a str,
        folder: Option<&'a str>,
    }
    invoke_cmd("create_note", &Args { name, folder }).await
}

pub async fn read_note(name: &str) -> Result<Note, String> {
    invoke_cmd("read_note", &HashMap::from([("name", name)])).await
}

pub async fn open_daily_note() -> Result<Note, String> {
    invoke_no_args("open_daily_note").await
}

pub async fn render_note(name: &str) -> Result<RenderedNote, String> {
    invoke_cmd("render_note", &HashMap::from([("name", name)])).await
}

pub async fn set_active_note(slug: Option<&str>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        slug: Option<&'a str>,
    }
    invoke_unit("set_active_note", &Args { slug }).await
}

pub async fn update_note(
    old_name: &str,
    new_name: &str,
    content: &str,
    tags: Option<Vec<String>>,
    icon: Option<String>,
) -> Result<NoteMeta, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args<'a> {
        old_name: &'a str,
        new_name: &'a str,
        content: &'a str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
    }
    invoke_cmd(
        "update_note",
        &Args {
            old_name,
            new_name,
            content,
            tags,
            icon,
        },
    )
    .await
}

pub async fn rename_note(old_name: &str, new_name: &str) -> Result<NoteMeta, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args<'a> {
        old_name: &'a str,
        new_name: &'a str,
    }
    invoke_cmd("rename_note", &Args { old_name, new_name }).await
}

pub async fn delete_note(slug: &str) -> Result<(), String> {
    invoke_unit("delete_note", &HashMap::from([("name", slug)])).await
}

pub async fn move_note(slug: &str, destination: Option<&str>) -> Result<NoteMeta, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        name: &'a str,
        destination: Option<&'a str>,
    }
    invoke_cmd(
        "move_note",
        &Args {
            name: slug,
            destination,
        },
    )
    .await
}

// ── Folders ────────────────────────────────────────────────────────

pub async fn create_folder(path: &str) -> Result<(), String> {
    invoke_unit("create_folder", &HashMap::from([("path", path)])).await
}

pub async fn delete_folder(path: &str) -> Result<(), String> {
    invoke_unit("delete_folder", &HashMap::from([("path", path)])).await
}

pub async fn rename_folder(source: &str, new_name: &str) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args<'a> {
        source: &'a str,
        new_name: &'a str,
    }
    invoke_unit("rename_folder", &Args { source, new_name }).await
}

pub async fn move_folder(source: &str, destination: Option<&str>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        source: &'a str,
        destination: Option<&'a str>,
    }
    invoke_unit(
        "move_folder",
        &Args {
            source,
            destination,
        },
    )
    .await
}

// ── Agent ──────────────────────────────────────────────────────────

pub async fn render_markdown(content: &str) -> Result<String, String> {
    let args = serde_wasm_bindgen::to_value(&HashMap::from([("content", content)]))
        .map_err(|e| format!("{e}"))?;
    let val = invoke("render_markdown", args)
        .await
        .map_err(js_err_to_string)?;
    val.as_string()
        .ok_or_else(|| "invalid response".to_string())
}

pub async fn send_message(msg: &str) -> Result<(), String> {
    invoke_unit("send_message", &HashMap::from([("msg", msg)])).await
}

pub async fn clear_chat() -> Result<(), String> {
    invoke_unit("clear_chat", &()).await
}

pub async fn list_tools() -> Result<Vec<ToolInfo>, String> {
    invoke_no_args("list_tools").await
}

// ── Themes ─────────────────────────────────────────────────────────

pub async fn set_active_theme(id: &str) -> Result<AppConfig, String> {
    invoke_cmd("set_active_theme", &HashMap::from([("id", id)])).await
}

pub async fn open_url(url: &str) -> Result<(), String> {
    invoke_unit("plugin:opener|open_url", &HashMap::from([("url", url)])).await
}

// ── Folder picker ──────────────────────────────────────────────────

pub async fn pick_folder() -> Option<String> {
    let tauri =
        js_sys::Reflect::get(&web_sys::window()?.into(), &JsValue::from_str("__TAURI__")).ok()?;
    let dialog = js_sys::Reflect::get(&tauri, &JsValue::from_str("dialog")).ok()?;
    let open_fn = js_sys::Reflect::get(&dialog, &JsValue::from_str("open")).ok()?;
    let open_fn = js_sys::Function::from(open_fn);

    #[derive(Serialize)]
    struct OpenDialogOptions {
        directory: bool,
        multiple: bool,
    }

    let opts = serde_wasm_bindgen::to_value(&OpenDialogOptions {
        directory: true,
        multiple: false,
    })
    .ok()?;

    let promise = open_fn.call1(&JsValue::NULL, &opts).ok()?;
    let result: JsValue = JsFuture::from(js_sys::Promise::from(promise)).await.ok()?;
    result.as_string()
}

// ── Event listening (agent streaming) ──────────────────────────────

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

/// Register a closure called when the agent invokes a tool.
pub async fn listen_tool_call(cb: impl Fn(ToolCallInfo) + 'static) -> Option<EventHandle> {
    listen_event("agent:tool-call", move |payload: JsValue| {
        let inner = js_sys::Reflect::get(&payload, &JsValue::from_str("payload"))
            .ok()
            .unwrap_or(payload);
        if let Ok(info) = serde_wasm_bindgen::from_value::<ToolCallInfo>(inner) {
            cb(info);
        }
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

/// Listen for a Tauri event that carries no meaningful payload.
/// Calls `cb` on every occurrence. Returns an [`EventHandle`].
pub async fn listen_event_simple(event: &str, cb: impl Fn() + 'static) -> Option<EventHandle> {
    listen_event(event, move |_| cb()).await
}
