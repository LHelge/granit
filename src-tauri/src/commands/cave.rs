use crate::cave::{CaveError, ContentMatch, Note, NoteMeta, Template, TemplateMeta};
use crate::markdown;
use granit_types::{RenderedNote, TodoList};

use super::{with_cave, AppState};

fn render_markdown_for_state(state: &AppState, content: &str) -> String {
    let guard = state.lock_cave();
    let cave = guard.as_ref();
    match cave {
        Some(cave) => markdown::render_markdown_with_links(content, |s| cave.lookup_slug(s)),
        None => markdown::render_html(content),
    }
}

#[tauri::command]
pub(crate) fn create_note(
    name: String,
    folder: Option<String>,
    template: Option<String>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| {
        cave.create_note(
            &name,
            folder.as_deref().map(std::path::Path::new),
            template.as_deref(),
        )
    })
}

#[tauri::command]
pub(crate) fn create_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<TemplateMeta, CaveError> {
    with_cave(&state, |cave| cave.create_template(&name))
}

#[tauri::command]
pub(crate) fn create_folder(path: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave(&state, |cave| {
        cave.create_folder(std::path::Path::new(&path))
    })
}

#[tauri::command]
pub(crate) fn delete_folder(path: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave(&state, |cave| {
        cave.delete_folder(std::path::Path::new(&path))
    })
}

#[tauri::command]
pub(crate) fn move_note(
    name: String,
    destination: Option<String>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| {
        cave.move_note(&name, destination.as_deref().map(std::path::Path::new))
    })
}

#[tauri::command]
pub(crate) fn move_folder(
    source: String,
    destination: Option<String>,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    with_cave(&state, |cave| {
        cave.move_folder(
            std::path::Path::new(&source),
            destination.as_deref().map(std::path::Path::new),
        )
    })
}

#[tauri::command]
pub(crate) fn list_notes(state: tauri::State<AppState>) -> Result<Vec<NoteMeta>, CaveError> {
    with_cave(&state, |cave| cave.list_notes())
}

#[tauri::command]
pub(crate) fn list_templates(
    state: tauri::State<AppState>,
) -> Result<Vec<TemplateMeta>, CaveError> {
    with_cave(&state, |cave| cave.list_templates())
}

#[tauri::command]
pub(crate) fn search_content(
    query: String,
    max_results: Option<usize>,
    state: tauri::State<AppState>,
) -> Result<Vec<ContentMatch>, CaveError> {
    with_cave(&state, |cave| cave.search_content(&query, max_results))
}

#[tauri::command]
pub(crate) fn list_folders(state: tauri::State<AppState>) -> Result<Vec<String>, CaveError> {
    with_cave(&state, |cave| cave.list_folders())
}

#[tauri::command]
pub(crate) fn read_note(name: String, state: tauri::State<AppState>) -> Result<Note, CaveError> {
    with_cave(&state, |cave| cave.read_note(&name))
}

#[tauri::command]
pub(crate) fn read_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<Template, CaveError> {
    with_cave(&state, |cave| cave.read_template(&name))
}

#[tauri::command]
pub(crate) fn open_daily_note(state: tauri::State<AppState>) -> Result<Note, CaveError> {
    let config = state.lock_config().clone();
    with_cave(&state, |cave| {
        cave.open_daily_note(
            &config.daily_note_folder,
            config.daily_note_template_slug.as_deref(),
        )
    })
}

#[tauri::command]
pub(crate) fn save_note(
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| cave.save_note(&name, &content))
}

#[tauri::command]
pub(crate) fn save_template(
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<TemplateMeta, CaveError> {
    with_cave(&state, |cave| cave.save_template(&name, &content))
}

#[tauri::command]
pub(crate) fn rename_note(
    old_name: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| cave.rename_note(&old_name, &new_name))
}

#[tauri::command]
pub(crate) fn rename_template(
    old_name: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<TemplateMeta, CaveError> {
    with_cave(&state, |cave| cave.rename_template(&old_name, &new_name))
}

#[tauri::command]
pub(crate) fn update_note(
    old_name: String,
    new_name: String,
    content: String,
    tags: Option<Vec<String>>,
    icon: Option<String>,
    favorite: Option<bool>,
    state: tauri::State<AppState>,
) -> Result<NoteMeta, CaveError> {
    with_cave(&state, |cave| {
        cave.update_note(&old_name, &new_name, &content, tags, icon, favorite)
    })
}

#[tauri::command]
pub(crate) fn update_template(
    old_name: String,
    new_name: String,
    content: String,
    tags: Option<Vec<String>>,
    icon: Option<String>,
    state: tauri::State<AppState>,
) -> Result<TemplateMeta, CaveError> {
    with_cave(&state, |cave| {
        cave.update_template(&old_name, &new_name, &content, tags, icon)
    })
}

#[tauri::command]
pub(crate) fn rename_folder(
    source: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    with_cave(&state, |cave| {
        cave.rename_folder(std::path::Path::new(&source), &new_name)
    })
}

#[tauri::command]
pub(crate) fn delete_note(name: String, state: tauri::State<AppState>) -> Result<(), CaveError> {
    with_cave(&state, |cave| cave.delete_note(&name))
}

#[tauri::command]
pub(crate) fn delete_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    with_cave(&state, |cave| cave.delete_template(&name))
}

#[tauri::command]
pub(crate) fn list_todos(state: tauri::State<AppState>) -> Result<TodoList, CaveError> {
    with_cave(&state, |cave| cave.list_todos())
}

#[tauri::command]
pub(crate) fn toggle_todo(
    slug: String,
    line: usize,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    use tauri::Emitter;
    with_cave(&state, |cave| cave.toggle_todo(&slug, line))?;
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
    with_cave(&state, |cave| cave.toggle_todo_by_index(&slug, index))?;
    let _ = app.emit("cave:notes-changed", ());
    Ok(())
}

#[tauri::command]
pub(crate) fn render_note(
    name: String,
    state: tauri::State<AppState>,
) -> Result<RenderedNote, CaveError> {
    with_cave(&state, |cave| {
        let slug = cave.resolve_slug(&name)?;
        let raw = cave.read_note_raw(&slug)?;
        let mut rendered = markdown::render_note(&raw, &slug, |s| cave.lookup_slug(s));
        rendered.backlinks = cave.backlink_note_metas(&slug)?;
        Ok(rendered)
    })
}

#[tauri::command]
pub(crate) fn render_template(
    name: String,
    state: tauri::State<AppState>,
) -> Result<RenderedNote, CaveError> {
    with_cave(&state, |cave| {
        let raw = cave.read_template_raw(&name)?;
        Ok(markdown::render_note(&raw, &name, |s| cave.lookup_slug(s)))
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
    with_cave(&state, |cave| {
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
