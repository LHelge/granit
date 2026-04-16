use granit_types::{
    AppConfig, AppMetadata, AttachedNote, ContentMatch, Document, DocumentMeta, IpcError,
    RenderedDocument, SidebarConfig, TodoList, ToolCallInfo, ToolInfo, AGENT_STREAM_CHUNK,
    AGENT_STREAM_DONE, AGENT_STREAM_ERROR, AGENT_TOOL_CALL,
};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// ── Tauri IPC binding ──────────────────────────────────────────────

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub(crate) async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// ── Generic invoke helpers ─────────────────────────────────────────

/// Convert a JS error value returned by `invoke` into an [`IpcError`].
///
/// Backend command results serialize their error as `{ code, message }`
/// (see `granit_types::IpcError`). If deserialization fails — e.g. the
/// error comes from Tauri itself rather than a command — fall back to a
/// best-effort string extraction under an `IpcTransport` code.
fn js_err_to_ipc(e: JsValue) -> IpcError {
    if let Ok(parsed) = serde_wasm_bindgen::from_value::<IpcError>(e.clone()) {
        return parsed;
    }
    let msg = if let Some(s) = e.as_string() {
        s
    } else {
        js_sys::Reflect::get(&e, &JsValue::from_str("message"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| {
                js_sys::JSON::stringify(&e)
                    .ok()
                    .and_then(|s| s.as_string())
                    .unwrap_or_else(|| "Unknown IPC error".to_string())
            })
    };
    IpcError::new("IpcTransport", msg)
}

fn serialize_err(e: impl std::fmt::Display) -> IpcError {
    IpcError::new("IpcSerialize", e.to_string())
}

/// Default timeout for all Tauri invoke calls. Commands that legitimately
/// run longer (e.g. agent streaming) use their own event-based flow and
/// do not go through these helpers.
const IPC_TIMEOUT_MS: u32 = 30_000;

/// Race a future against an IPC timeout. On timeout, returns an
/// [`IpcError`] with code `IpcTimeout`.
async fn with_timeout<T, F>(cmd: &str, fut: F) -> Result<T, IpcError>
where
    F: std::future::Future<Output = Result<T, IpcError>>,
{
    use futures::future::{select, Either};
    use gloo_timers::future::TimeoutFuture;

    let fut = std::pin::pin!(fut);
    let timeout = TimeoutFuture::new(IPC_TIMEOUT_MS);
    match select(fut, timeout).await {
        Either::Left((res, _)) => res,
        Either::Right(_) => Err(IpcError::new(
            "IpcTimeout",
            format!("IPC command '{cmd}' timed out after {IPC_TIMEOUT_MS}ms"),
        )),
    }
}

/// Invoke a Tauri command with typed args and a typed return value.
async fn invoke_cmd<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Result<R, IpcError> {
    with_timeout(cmd, async {
        let args = serde_wasm_bindgen::to_value(args).map_err(serialize_err)?;
        let val = invoke(cmd, args).await.map_err(js_err_to_ipc)?;
        serde_wasm_bindgen::from_value(val).map_err(serialize_err)
    })
    .await
}

/// Invoke a Tauri command with no args and a typed return value.
async fn invoke_no_args<R: DeserializeOwned>(cmd: &str) -> Result<R, IpcError> {
    with_timeout(cmd, async {
        let val = invoke(cmd, JsValue::NULL).await.map_err(js_err_to_ipc)?;
        serde_wasm_bindgen::from_value(val).map_err(serialize_err)
    })
    .await
}

/// Invoke a Tauri command with typed args and no return value.
async fn invoke_unit<A: Serialize>(cmd: &str, args: &A) -> Result<(), IpcError> {
    with_timeout(cmd, async {
        let args = serde_wasm_bindgen::to_value(args).map_err(serialize_err)?;
        invoke(cmd, args).await.map_err(js_err_to_ipc)?;
        Ok(())
    })
    .await
}

// ── ipc! macro ─────────────────────────────────────────────────────
//
// Declarative wrapper generator that collapses the per-command
// boilerplate (Args struct, field renames, invoke_*) into a single
// line per command. The payload body is passed as a `serde_json::json!`
// block, keeping JSON field names explicit at the call site — no
// surprises from `#[serde(rename_all)]`.
//
// Four shapes are supported:
//
//   ipc! { pub async fn fetch_x() -> T = "cmd"; }          // no args
//   ipc! { pub async fn act()        = "cmd"; }            // no args, no return
//   ipc! { pub async fn f(a: A) -> T = "cmd" { "a": a }; } // typed args + return
//   ipc! { pub async fn g(a: A)      = "cmd" { "a": a }; } // typed args, no return
macro_rules! ipc {
    // Empty terminator.
    () => {};

    // No-args + typed return.
    (
        $(#[$m:meta])*
        $vis:vis async fn $name:ident() -> $ret:ty = $cmd:literal ;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        $vis async fn $name() -> Result<$ret, IpcError> {
            invoke_no_args($cmd).await
        }
        ipc! { $($rest)* }
    };

    // No-args, no return.
    (
        $(#[$m:meta])*
        $vis:vis async fn $name:ident() = $cmd:literal ;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        $vis async fn $name() -> Result<(), IpcError> {
            invoke_unit($cmd, &()).await
        }
        ipc! { $($rest)* }
    };

    // Args + typed return. `$body` is the serde_json::json! object body.
    (
        $(#[$m:meta])*
        $vis:vis async fn $name:ident($($arg:ident : $ty:ty),+ $(,)?) -> $ret:ty = $cmd:literal $body:tt ;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        $vis async fn $name($($arg : $ty),+) -> Result<$ret, IpcError> {
            invoke_cmd($cmd, &serde_json::json!($body)).await
        }
        ipc! { $($rest)* }
    };

    // Args, no return.
    (
        $(#[$m:meta])*
        $vis:vis async fn $name:ident($($arg:ident : $ty:ty),+ $(,)?) = $cmd:literal $body:tt ;
        $($rest:tt)*
    ) => {
        $(#[$m])*
        $vis async fn $name($($arg : $ty),+) -> Result<(), IpcError> {
            invoke_unit($cmd, &serde_json::json!($body)).await
        }
        ipc! { $($rest)* }
    };
}

// ── Command wrappers ───────────────────────────────────────────────

ipc! {
    // Config
    pub async fn fetch_config() -> AppConfig = "get_config";
    pub async fn fetch_app_metadata() -> AppMetadata = "get_app_metadata";
    pub async fn list_system_fonts() -> Vec<String> = "list_system_fonts";
    pub async fn save_config(config: AppConfig) -> AppConfig = "save_config" { "config": config };
    pub async fn save_sidebar_state(sidebar: SidebarConfig, agent_panel: SidebarConfig)
        = "save_sidebar_state" { "sidebar": sidebar, "agentPanel": agent_panel };

    // Provider / Model selection
    pub async fn select_provider(index: usize) -> AppConfig
        = "select_provider" { "index": index };
    pub async fn list_models() -> Vec<granit_types::ModelInfo> = "list_models";
    pub async fn select_model(model_id: &str) -> AppConfig
        = "select_model" { "modelId": model_id };

    // Cave
    pub async fn open_cave(path: &str) -> AppConfig = "open_cave" { "path": path };
    pub async fn fetch_notes() -> Vec<DocumentMeta> = "list_notes";
    pub async fn search_content(query: &str) -> Vec<ContentMatch>
        = "search_content" { "query": query, "max_results": 40 };
    pub async fn fetch_folders() -> Vec<String> = "list_folders";
    pub async fn fetch_templates() -> Vec<DocumentMeta> = "list_templates";

    // Notes
    pub async fn create_note(name: &str, folder: Option<&str>, template: Option<&str>) -> DocumentMeta
        = "create_note" { "name": name, "folder": folder, "template": template };
    pub async fn read_note(name: &str) -> Document = "read_note" { "name": name };
    pub async fn open_daily_note() -> Document = "open_daily_note";
    pub async fn open_daily_note_for_date(date: &str) -> Document
        = "open_daily_note_for_date" { "date": date };
    pub async fn create_template(name: &str) -> DocumentMeta
        = "create_template" { "name": name };
    pub async fn read_template(name: &str) -> Document
        = "read_template" { "name": name };
    pub async fn render_template(name: &str) -> RenderedDocument
        = "render_template" { "name": name };
    pub async fn render_note(name: &str) -> RenderedDocument
        = "render_note" { "name": name };
    pub async fn set_active_note(slug: Option<&str>) = "set_active_note" { "slug": slug };
    pub async fn update_note(
        old_name: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
        favorite: Option<bool>,
    ) -> DocumentMeta = "update_note" {
        "oldName": old_name,
        "newName": new_name,
        "content": content,
        "tags": tags,
        "icon": icon,
        "favorite": favorite,
    };
    pub async fn rename_note(old_name: &str, new_name: &str) -> DocumentMeta
        = "rename_note" { "oldName": old_name, "newName": new_name };
    pub async fn delete_note(slug: &str) = "delete_note" { "name": slug };
    pub async fn update_template(
        old_name: &str,
        new_name: &str,
        content: &str,
        tags: Option<Vec<String>>,
        icon: Option<String>,
    ) -> DocumentMeta = "update_template" {
        "oldName": old_name,
        "newName": new_name,
        "content": content,
        "tags": tags,
        "icon": icon,
    };
    pub async fn delete_template(slug: &str) = "delete_template" { "name": slug };
    pub async fn move_note(slug: &str, destination: Option<&str>) -> DocumentMeta
        = "move_note" { "name": slug, "destination": destination };

    // Folders
    pub async fn create_folder(path: &str) = "create_folder" { "path": path };
    pub async fn delete_folder(path: &str) = "delete_folder" { "path": path };
    pub async fn rename_folder(source: &str, new_name: &str)
        = "rename_folder" { "source": source, "newName": new_name };
    pub async fn move_folder(source: &str, destination: Option<&str>)
        = "move_folder" { "source": source, "destination": destination };

    // Agent
    pub async fn send_message(msg: &str, attached_notes: Vec<AttachedNote>)
        = "send_message" { "msg": msg, "attachedNotes": attached_notes };
    pub async fn clear_chat() = "clear_chat";
    pub async fn list_tools() -> Vec<ToolInfo> = "list_tools";
    pub async fn open_url(url: &str) = "plugin:opener|open_url" { "url": url };

    // Todos
    pub async fn list_todos() -> TodoList = "list_todos";
    pub async fn toggle_todo(slug: &str, line: usize)
        = "toggle_todo" { "slug": slug, "line": line };
    pub async fn toggle_todo_by_index(slug: &str, index: usize)
        = "toggle_todo_by_index" { "slug": slug, "index": index };

    // Tags
    pub async fn list_tags() -> granit_types::TagMap = "list_tags";
}

// render_markdown returns a plain JS string, not a JSON-encoded value,
// so it can't go through invoke_cmd. Keep it as a bespoke wrapper.
pub async fn render_markdown(content: &str) -> Result<String, IpcError> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "content": content }))
        .map_err(serialize_err)?;
    let val = invoke("render_markdown", args)
        .await
        .map_err(js_err_to_ipc)?;
    val.as_string()
        .ok_or_else(|| IpcError::new("IpcSerialize", "invalid response"))
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
    listen_event(AGENT_STREAM_CHUNK, move |payload: JsValue| {
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
    listen_event(AGENT_STREAM_DONE, move |_| cb()).await
}

/// Register a closure called on a streaming error.
pub async fn listen_stream_error(cb: impl Fn(String) + 'static) -> Option<EventHandle> {
    listen_event(AGENT_STREAM_ERROR, move |payload: JsValue| {
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
    listen_event(AGENT_TOOL_CALL, move |payload: JsValue| {
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

/// Register a Tauri event listener and keep the [`EventHandle`] alive
/// for the current reactive owner's lifetime.
///
/// The standard pattern — spawning a `'static` future that holds the
/// handle and then suspends on `pending()` — is awkward to repeat. This
/// helper consolidates it. Cancellation of the Leptos Effect on
/// component unmount drops the future, which drops the handle, which
/// triggers the Tauri unlisten.
pub fn spawn_event_listener_simple(event: &'static str, cb: impl Fn() + 'static) {
    leptos::task::spawn_local(async move {
        let _handle = listen_event_simple(event, cb).await;
        std::future::pending::<()>().await;
    });
}
