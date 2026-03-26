---
applyTo: "src/**"
---

# Leptos Frontend

Instructions for working in the Leptos 0.8 frontend (`src/`).

## IPC Pattern

Call Tauri backend commands via the `invoke` FFI binding:

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}
```

To call a command:
```rust
use serde::Serialize;

#[derive(Serialize)]
struct OpenCaveArgs { path: String }

let args = serde_wasm_bindgen::to_value(&OpenCaveArgs { path }).unwrap();
let result = invoke("open_cave", args).await;
let cave: Cave = serde_wasm_bindgen::from_value(result).unwrap();
```

## Reactive Signals

Leptos 0.8 uses `signal()` for reactive state:

```rust
// Create a signal (getter + setter)
let (notes, set_notes) = signal(Vec::<Note>::new());

// Read in a reactive context (tracks automatically)
view! { <p>{move || notes.get().len()}</p> }

// Update from an event handler or async block
set_notes.set(new_notes);
```

Use `RwSignal` when you need both read and write in one handle:
```rust
let cave = RwSignal::new(None::<Cave>);
cave.set(Some(loaded_cave));
```

## Async IPC Calls

Use `leptos::task::spawn_local` for async operations:

```rust
let on_click = move |_| {
    spawn_local(async move {
        let result = invoke("list_notes", JsValue::NULL).await;
        let notes: Vec<Note> = serde_wasm_bindgen::from_value(result).unwrap();
        set_notes.set(notes);
    });
};
```

## Component Structure

Break the UI into focused components. Each component is a function returning `impl IntoView`:

```rust
#[component]
fn NoteList(notes: ReadSignal<Vec<Note>>) -> impl IntoView {
    view! {
        <ul class="space-y-2">
            {move || notes.get().into_iter().map(|note| {
                view! { <li class="p-2 rounded hover:bg-gray-100">{note.title}</li> }
            }).collect_view()}
        </ul>
    }
}
```

## Tailwind CSS

Use utility classes directly in `view!` macros. Common patterns:

```rust
view! {
    // Layout
    <div class="flex h-screen">
        <aside class="w-64 border-r p-4">/* sidebar */</aside>
        <main class="flex-1 p-6">/* content */</main>
    </div>

    // Buttons
    <button class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
        "Save"
    </button>

    // Text input
    <input type="text" class="w-full border rounded px-3 py-2 focus:outline-none focus:ring-2" />
}
```

## Event Handling

```rust
// Input binding
let (text, set_text) = signal(String::new());
view! {
    <input
        type="text"
        prop:value=move || text.get()
        on:input=move |ev| set_text.set(event_target_value(&ev))
    />
}

// Form submission (prevent default)
view! {
    <form on:submit=move |ev| {
        ev.prevent_default();
        // handle submit
    }>
    </form>
}
```

## Key Gotcha

Leptos closures in `view!` need `move ||` to capture signals. If you get borrow errors, check that signals are `Copy` (they are in Leptos 0.8) and that closures use `move`.
