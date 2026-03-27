use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use super::types::{AppConfig, Note, NoteMeta};

// ── Tauri IPC binding ──────────────────────────────────────────────

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub(crate) async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// ── Argument structs ───────────────────────────────────────────────

#[derive(Serialize)]
struct NameArg {
    name: String,
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
struct SaveNoteArgs {
    name: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenameNoteArgs {
    old_name: String,
    new_name: String,
}

#[derive(Serialize)]
struct SaveConfigArgs {
    agent: super::types::AgentConfig,
}

// ── IPC helpers ────────────────────────────────────────────────────

pub async fn fetch_config() -> Option<AppConfig> {
    let val = invoke("get_config", JsValue::NULL).await.ok()?;
    serde_wasm_bindgen::from_value(val).ok()
}

pub async fn open_cave(path: &str) -> Option<AppConfig> {
    let args = serde_wasm_bindgen::to_value(&OpenCaveArgs {
        path: path.to_string(),
    })
    .ok()?;
    let val = invoke("open_cave", args).await.ok()?;
    serde_wasm_bindgen::from_value(val).ok()
}

pub async fn fetch_notes() -> Vec<NoteMeta> {
    let Ok(result) = invoke("list_notes", JsValue::NULL).await else {
        return Vec::new();
    };
    serde_wasm_bindgen::from_value(result).unwrap_or_default()
}

pub async fn create_note(name: &str) -> Option<NoteMeta> {
    let args = serde_wasm_bindgen::to_value(&NameArg {
        name: name.to_string(),
    })
    .ok()?;
    let val = invoke("create_note", args).await.ok()?;
    serde_wasm_bindgen::from_value(val).ok()
}

pub async fn read_note(name: &str) -> Option<Note> {
    let args = serde_wasm_bindgen::to_value(&NameArg {
        name: name.to_string(),
    })
    .ok()?;
    let val = invoke("read_note", args).await.ok()?;
    serde_wasm_bindgen::from_value(val).ok()
}

pub async fn save_note(name: &str, content: &str) -> Result<NoteMeta, String> {
    let args = serde_wasm_bindgen::to_value(&SaveNoteArgs {
        name: name.to_string(),
        content: content.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("save_note", args)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| "Save failed".to_string()))?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn rename_note(old_name: &str, new_name: &str) -> Result<NoteMeta, String> {
    let args = serde_wasm_bindgen::to_value(&RenameNoteArgs {
        old_name: old_name.to_string(),
        new_name: new_name.to_string(),
    })
    .map_err(|e| format!("{e}"))?;
    let val = invoke("rename_note", args)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| "Rename failed".to_string()))?;
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("{e}"))
}

pub async fn save_config(agent: super::types::AgentConfig) -> Option<AppConfig> {
    let args = serde_wasm_bindgen::to_value(&SaveConfigArgs { agent }).ok()?;
    let val = invoke("save_config", args).await.ok()?;
    serde_wasm_bindgen::from_value(val).ok()
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
