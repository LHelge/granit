use super::AppState;
use crate::cave::{CaveError, ContentMatch, Document, DocumentMeta, RenderedDocument};
use crate::markdown::Markdown;
use granit_types::{TagMap, TodoList};

pub(super) fn render_markdown_for_state(state: &AppState, content: &str) -> String {
    let md = Markdown::new(content);
    state
        .with_cave(|cave| Ok(md.render_with_links(|s| cave.resolve_link(s))))
        .unwrap_or_else(|_| md.render_html())
}

/// Spawn a background task to update the embedding for a note.
fn spawn_index_update(state: &AppState, slug: String) {
    if let Some(index) = state.vector_index() {
        tauri::async_runtime::spawn(async move {
            index.update_note(&slug).await;
        });
    }
}

/// Spawn a background task to remove the embedding for a deleted note.
fn spawn_index_remove(state: &AppState, slug: String) {
    if let Some(index) = state.vector_index() {
        tauri::async_runtime::spawn(async move {
            index.remove_note(&slug).await;
        });
    }
}

#[tauri::command]
pub(crate) fn create_note(
    name: String,
    folder: Option<String>,
    template: Option<String>,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    let meta = state.with_cave(|cave| {
        cave.create_note(
            &name,
            folder.as_deref().map(std::path::Path::new),
            template.as_deref(),
        )
    })?;
    spawn_index_update(state.inner(), meta.slug.clone());
    Ok(meta)
}

#[tauri::command]
pub(crate) fn create_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    state.with_cave(|cave| cave.create_template(&name))
}

#[tauri::command]
pub(crate) fn create_folder(path: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    state.with_cave(|cave| cave.create_folder(std::path::Path::new(&path)))
}

#[tauri::command]
pub(crate) fn delete_folder(path: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    state.with_cave(|cave| cave.delete_folder(std::path::Path::new(&path)))
}

#[tauri::command]
pub(crate) fn move_note(
    name: String,
    destination: Option<String>,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    state.with_cave(|cave| cave.move_note(&name, destination.as_deref().map(std::path::Path::new)))
}

#[tauri::command]
pub(crate) fn move_folder(
    source: String,
    destination: Option<String>,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    state.with_cave(|cave| {
        cave.move_folder(
            std::path::Path::new(&source),
            destination.as_deref().map(std::path::Path::new),
        )
    })
}

#[tauri::command]
pub(crate) fn list_notes(state: tauri::State<AppState>) -> Result<Vec<DocumentMeta>, CaveError> {
    state.with_cave(|cave| cave.list_notes())
}

/// List all heading anchor ids in the cave (`# Heading {#id}` targets), for
/// wiki-link completion alongside note slugs.
#[tauri::command]
pub(crate) fn list_anchors(state: tauri::State<AppState>) -> Result<Vec<String>, CaveError> {
    state.with_cave(|cave| Ok(cave.list_anchors()))
}

#[tauri::command]
pub(crate) fn list_templates(
    state: tauri::State<AppState>,
) -> Result<Vec<DocumentMeta>, CaveError> {
    state.with_cave(|cave| cave.list_templates())
}

#[tauri::command]
pub(crate) fn search_content(
    query: String,
    max_results: Option<usize>,
    state: tauri::State<AppState>,
) -> Result<Vec<ContentMatch>, CaveError> {
    state.with_cave(|cave| cave.search_content(&query, max_results))
}

#[tauri::command]
pub(crate) fn list_folders(state: tauri::State<AppState>) -> Result<Vec<String>, CaveError> {
    state.with_cave(|cave| cave.list_folders())
}

#[tauri::command]
pub(crate) fn read_note(
    name: String,
    state: tauri::State<AppState>,
) -> Result<Document, CaveError> {
    state.with_cave(|cave| cave.read_note(&name))
}

#[tauri::command]
pub(crate) fn read_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<Document, CaveError> {
    state.with_cave(|cave| cave.read_template(&name))
}

#[tauri::command]
pub(crate) fn open_daily_note(state: tauri::State<AppState>) -> Result<Document, CaveError> {
    let config = state.lock_config().clone();
    state.with_cave(|cave| {
        cave.open_daily_note(
            &config.daily_note_folder,
            config.daily_note_template_slug.as_deref(),
        )
    })
}

#[tauri::command]
pub(crate) fn open_daily_note_for_date(
    date: String,
    state: tauri::State<AppState>,
) -> Result<Document, CaveError> {
    let config = state.lock_config().clone();
    state.with_cave(|cave| {
        cave.open_daily_note_for_date(
            &date,
            &config.daily_note_folder,
            config.daily_note_template_slug.as_deref(),
        )
    })
}

#[tauri::command]
pub(crate) fn save_note(
    name: String,
    content: String,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    use tauri::Emitter;
    let meta = state.with_cave(|cave| cave.save_note(&name, &content))?;
    spawn_index_update(state.inner(), meta.slug.clone());
    let _ = app.emit("cave:notes-changed", ());
    Ok(meta)
}

#[tauri::command]
pub(crate) fn save_template(
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    state.with_cave(|cave| cave.save_template(&name, &content))
}

#[tauri::command]
pub(crate) fn rename_note(
    old_name: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    // Capture inbound-link sources before the rename: their bodies are rewritten
    // to point at the new slug, so their embeddings must be refreshed too.
    let (meta, affected) = state.with_cave(|cave| {
        let affected = cave.backlink_slugs(&old_name).unwrap_or_default();
        let meta = cave.rename_note(&old_name, &new_name)?;
        Ok((meta, affected))
    })?;
    // Only after the rename succeeded: a failed rename (e.g. AlreadyExists)
    // must not drop the embedding of a note that still exists.
    spawn_index_remove(state.inner(), old_name.clone());
    spawn_index_update(state.inner(), meta.slug.clone());
    for slug in affected {
        spawn_index_update(state.inner(), slug);
    }
    Ok(meta)
}

#[tauri::command]
pub(crate) fn rename_template(
    old_name: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    state.with_cave(|cave| cave.rename_template(&old_name, &new_name))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_note(
    old_name: String,
    new_name: String,
    content: String,
    tags: Option<Vec<String>>,
    icon: Option<String>,
    favorite: Option<bool>,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    use tauri::Emitter;
    let renaming = old_name != new_name;
    let (meta, affected) = state.with_cave(|cave| {
        // Only a rename rewrites inbound links in other notes; capture those
        // sources first so their embeddings can be refreshed afterwards.
        let affected = if renaming {
            cave.backlink_slugs(&old_name).unwrap_or_default()
        } else {
            Vec::new()
        };
        let meta = cave.update_note(&old_name, &new_name, &content, tags, icon, favorite)?;
        Ok((meta, affected))
    })?;
    // Only after the update succeeded: a failed rename-within-update must not
    // drop the embedding of a note that still exists under its old slug.
    if renaming {
        spawn_index_remove(state.inner(), old_name.clone());
    }
    spawn_index_update(state.inner(), meta.slug.clone());
    for slug in affected {
        spawn_index_update(state.inner(), slug);
    }
    let _ = app.emit("cave:notes-changed", ());
    Ok(meta)
}

#[tauri::command]
pub(crate) fn update_template(
    old_name: String,
    new_name: String,
    content: String,
    tags: Option<Vec<String>>,
    icon: Option<String>,
    state: tauri::State<AppState>,
) -> Result<DocumentMeta, CaveError> {
    state.with_cave(|cave| cave.update_template(&old_name, &new_name, &content, tags, icon))
}

#[tauri::command]
pub(crate) fn rename_folder(
    source: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    state.with_cave(|cave| cave.rename_folder(std::path::Path::new(&source), &new_name))
}

#[tauri::command]
pub(crate) fn delete_note(name: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    state.with_cave(|cave| cave.delete_note(&name))?;
    spawn_index_remove(state.inner(), name);
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    state.with_cave(|cave| cave.delete_template(&name))
}

#[tauri::command]
pub(crate) fn list_todos(state: tauri::State<AppState>) -> Result<TodoList, CaveError> {
    state.with_cave(|cave| cave.list_todos())
}

#[tauri::command]
pub(crate) fn list_tags(state: tauri::State<AppState>) -> Result<TagMap, CaveError> {
    state.with_cave(|cave| cave.list_tags())
}

#[tauri::command]
pub(crate) fn toggle_todo(
    slug: String,
    line: usize,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    use tauri::Emitter;
    state.with_cave(|cave| cave.toggle_todo(&slug, line))?;
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_todo_by_index(
    slug: String,
    index: usize,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    use tauri::Emitter;
    state.with_cave(|cave| cave.toggle_todo_by_index(&slug, index))?;
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
pub(crate) fn render_note(
    name: String,
    state: tauri::State<AppState>,
) -> Result<RenderedDocument, CaveError> {
    state.with_cave(|cave| {
        let slug = cave.resolve_slug(&name)?;
        let raw = cave.read_note_raw(&slug)?;
        let mut rendered = Markdown::new(&raw).render(&slug, |s| cave.resolve_link(s));
        rendered.backlinks = cave.backlink_note_metas(&slug)?;
        Ok(rendered)
    })
}

#[tauri::command]
pub(crate) fn render_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<RenderedDocument, CaveError> {
    state.with_cave(|cave| {
        let raw = cave.read_template_raw(&name)?;
        Ok(Markdown::new(&raw).render(&name, |s| cave.resolve_link(s)))
    })
}

#[tauri::command]
pub(crate) fn render_markdown(content: String, state: tauri::State<AppState>) -> String {
    render_markdown_for_state(state.inner(), &content)
}

#[tauri::command]
pub(crate) fn set_active_note(
    slug: Option<String>,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    state.with_cave(|cave| {
        cave.set_active_slug(slug);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use granit_types::AppConfig;

    fn test_app_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    #[test]
    fn test_render_markdown_for_state_without_open_cave_leaves_wiki_links_plain() {
        let state = test_app_state();

        let html = render_markdown_for_state(&state, "See [[target]]");

        assert!(html.contains("[[target]]"));
        assert!(!html.contains("href=\"target\""));
    }

    #[test]
    fn test_render_markdown_for_state_with_open_cave_resolves_wiki_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
        let cave = crate::cave::Cave::open(dir.path().to_path_buf()).unwrap();

        let state = test_app_state();
        state.set_cave(Some(cave));

        let html = render_markdown_for_state(&state, "See [[target]]");

        assert!(html.contains("href=\"target\""), "got: {html}");
        assert!(!html.contains("broken-link"), "got: {html}");
    }
}
