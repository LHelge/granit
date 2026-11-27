//! Cave operations of the UI, on top of the core.
//!
//! Each mutation runs on the core cave, keeps the RAG index current the way
//! the Tauri commands in `src-tauri/src/commands/cave.rs` do, and announces
//! itself as a [`CaveEvent`]. File operations are small and synchronous, so
//! they run on the calling (foreground) thread; only opening a cave, which
//! scans the whole directory, goes to the background.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::{App, AppContext as _, Task};
use granit_core::cave::{CaveError, Document, DocumentMeta};
use granit_core::open::{prepare_cave, sync_vector_index};
use granit_types::TodoItem;
use log::warn;

use crate::events::{CaveEvent, CaveEvents};
use crate::host::Host;
use crate::runtime;

/// The name new notes start with; the core numbers duplicates
/// (`untitled-2`, …).
pub const NEW_NOTE_NAME: &str = "untitled";

/// The name new folders start with.
pub const NEW_FOLDER_NAME: &str = "new-folder";

/// Why a cave did not open.
#[derive(Debug, thiserror::Error)]
pub enum OpenCaveError {
    #[error(transparent)]
    Cave(#[from] CaveError),
    /// `before_install` refused the switch (unsaved edits could not be
    /// written).
    #[error("{0}")]
    Refused(String),
}

/// Open the cave at `path` and make it the app's cave: prepare and seed it
/// off the UI thread, install it into the core, remember it as the last-open
/// cave (when the host remembers one), start the RAG index, and announce
/// [`CaveEvent::Opened`]. `before_install` runs on the foreground right
/// before the core swaps caves (to write pending edits to the old one); an
/// error from it cancels the switch. On failure the previous cave stays
/// open.
pub fn open_cave(
    path: PathBuf,
    before_install: impl FnOnce(&mut App) -> Result<(), String> + 'static,
    cx: &mut App,
) -> Task<Result<(), OpenCaveError>> {
    cx.spawn(async move |cx| {
        let prepared = cx
            .background_spawn({
                let path = path.clone();
                async move { prepare_cave(path) }
            })
            .await?;
        cx.update(|cx| {
            before_install(cx).map_err(OpenCaveError::Refused)?;
            let core = Host::core(cx);
            core.install_cave(prepared);
            if let Some(store) = Host::last_cave(cx) {
                // Best-effort: the cave is open either way.
                if let Err(err) = store.save(&path) {
                    warn!("Failed to remember the open cave: {err}");
                }
            }
            match Host::embedding_models(cx) {
                Some(dirs) => sync_vector_index(&core, &dirs, &runtime::handle()),
                None => core.set_vector_index(None),
            }
            CaveEvents::emit(CaveEvent::Opened, cx);
            Ok(())
        })
    })
}

/// The cave's notes and folders, for the explorer.
pub fn list_tree(cx: &App) -> Result<(Vec<DocumentMeta>, Vec<String>), CaveError> {
    Host::core(cx).with_cave(|cave| Ok((cave.list_notes()?, cave.list_folders()?)))
}

/// Read a note.
pub fn read_note(slug: &str, cx: &App) -> Result<Document, CaveError> {
    Host::core(cx).with_cave(|cave| cave.read_note(slug))
}

/// Tell the core which note is open, so agent tools can see it.
pub fn set_active_note(slug: Option<String>, cx: &App) {
    // No open cave means no active note to track.
    let _ = Host::core(cx).with_cave(|cave| {
        cave.set_active_slug(slug);
        Ok(())
    });
}

/// Create a note named [`NEW_NOTE_NAME`] in `folder` (`None`: the cave
/// root): empty, or rendered from the note template `template` (its body
/// through Tera, its tags, icon and presentation copied), as the Tauri
/// app's `create_note` does.
pub fn create_note(
    folder: Option<&str>,
    template: Option<&str>,
    cx: &mut App,
) -> Result<DocumentMeta, CaveError> {
    let meta = Host::core(cx)
        .with_cave(|cave| cave.create_note(NEW_NOTE_NAME, folder.map(Path::new), template))?;
    schedule_index_update(meta.slug.clone(), cx);
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(meta)
}

/// Create the note a broken wiki-link points at, named after its target, in
/// the cave root.
pub fn create_linked_note(name: &str, cx: &mut App) -> Result<DocumentMeta, CaveError> {
    let meta = Host::core(cx).with_cave(|cave| cave.create_note(name, None, None))?;
    schedule_index_update(meta.slug.clone(), cx);
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(meta)
}

/// Write the body of the note `slug`, keeping its frontmatter. Writes
/// nothing when the body is unchanged.
pub fn save_note(slug: &str, body: &str, cx: &mut App) -> Result<DocumentMeta, CaveError> {
    let meta = Host::core(cx).with_cave(|cave| cave.save_note(slug, body))?;
    schedule_index_update(meta.slug.clone(), cx);
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(meta)
}

/// The note templates in `.granit/templates/`, sorted by slug.
pub fn list_templates(cx: &App) -> Result<Vec<DocumentMeta>, CaveError> {
    Host::core(cx).with_cave(|cave| cave.list_templates())
}

/// The templates offered for a new note: every template except the
/// daily-note one, which daily notes use on their own (as in the Tauri
/// app's "New note from template" menu).
pub fn templates_for_new_notes(cx: &App) -> Result<Vec<DocumentMeta>, CaveError> {
    let daily = Host::core(cx)
        .lock_config()
        .daily_note_template_slug
        .clone();
    let mut templates = list_templates(cx)?;
    templates.retain(|template| daily.as_deref() != Some(template.slug.as_str()));
    Ok(templates)
}

/// Read a template: its body (without frontmatter), for the editor.
pub fn read_template(slug: &str, cx: &App) -> Result<Document, CaveError> {
    Host::core(cx).with_cave(|cave| cave.read_template(slug))
}

/// Create an empty template named [`NEW_NOTE_NAME`] (numbered when taken)
/// and announce [`CaveEvent::TemplatesChanged`].
pub fn create_template(cx: &mut App) -> Result<DocumentMeta, CaveError> {
    let meta = Host::core(cx).with_cave(|cave| cave.create_template(NEW_NOTE_NAME))?;
    CaveEvents::emit(CaveEvent::TemplatesChanged, cx);
    Ok(meta)
}

/// Write the body of the template `slug`, keeping its frontmatter. Writes
/// nothing when the body is unchanged. Announces nothing: no other view
/// shows a template's body.
pub fn save_template(slug: &str, body: &str, cx: &mut App) -> Result<DocumentMeta, CaveError> {
    Host::core(cx).with_cave(|cave| cave.save_template(slug, body))
}

/// Rename the template `from` to `to`. Notes made from it are not touched;
/// a daily-note template setting naming it is not followed (as in the
/// Tauri app).
pub fn rename_template(from: &str, to: &str, cx: &mut App) -> Result<DocumentMeta, CaveError> {
    let meta = Host::core(cx).with_cave(|cave| cave.rename_template(from, to))?;
    CaveEvents::emit(
        CaveEvent::TemplateRenamed {
            from: from.to_string(),
            to: meta.slug.clone(),
        },
        cx,
    );
    Ok(meta)
}

/// Delete the template `slug`.
pub fn delete_template(slug: &str, cx: &mut App) -> Result<(), CaveError> {
    Host::core(cx).with_cave(|cave| cave.delete_template(slug))?;
    CaveEvents::emit(CaveEvent::TemplateRemoved(slug.to_string()), cx);
    Ok(())
}

/// Open the daily note of `date` (`YYYY-MM-DD`; `None`: today), creating it
/// in the configured daily-note folder from the configured template when it
/// does not exist yet, as the Tauri app's `open_daily_note_for_date` does.
/// Announces [`CaveEvent::NotesChanged`] only when the note was created.
pub fn open_daily_note(date: Option<&str>, cx: &mut App) -> Result<DocumentMeta, CaveError> {
    let core = Host::core(cx);
    let (folder, template) = {
        let config = core.lock_config();
        (
            config.daily_note_folder.clone(),
            config.daily_note_template_slug.clone(),
        )
    };
    let date = date.map_or_else(granit_core::calendar::today_slug, str::to_string);
    let (doc, created) = core.with_cave(|cave| {
        let created = !cave.note_paths().contains_key(&date);
        let doc = cave.open_daily_note_for_date(&date, &folder, template.as_deref())?;
        Ok((doc, created))
    })?;
    if created {
        schedule_index_update(doc.meta.slug.clone(), cx);
        CaveEvents::emit(CaveEvent::NotesChanged, cx);
    }
    Ok(doc.meta)
}

/// The slugs of the daily notes in the configured daily-note folder, for
/// the calendar.
pub fn daily_note_slugs(cx: &App) -> Result<HashSet<String>, CaveError> {
    let core = Host::core(cx);
    let folder = core.lock_config().daily_note_folder.clone();
    core.with_cave(|cave| Ok(cave.daily_note_slugs(&folder)))
}

/// Toggle a listed todo (`checked`: its state when listed) in its note,
/// keeping the note's frontmatter and line endings. The todo is found again
/// first ([`granit_core::cave::Cave::locate_todo`]), so a note that changed
/// since the list was built still toggles the right line.
pub fn toggle_todo(item: &TodoItem, checked: bool, cx: &mut App) -> Result<(), CaveError> {
    Host::core(cx).with_cave(|cave| {
        let line = cave.locate_todo(&item.slug, item.line, &item.text, checked)?;
        cave.toggle_todo(&item.slug, line)
    })?;
    schedule_index_update(item.slug.clone(), cx);
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(())
}

/// Create a folder named [`NEW_FOLDER_NAME`] in `parent` (`None`: the cave
/// root) and return its path.
pub fn create_folder(parent: Option<&str>, cx: &mut App) -> Result<String, CaveError> {
    let path = join(parent, NEW_FOLDER_NAME);
    Host::core(cx).with_cave(|cave| cave.create_folder(Path::new(&path)))?;
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(path)
}

/// Rename the note `from` to `to`, rewriting the links to it.
pub fn rename_note(from: &str, to: &str, cx: &mut App) -> Result<DocumentMeta, CaveError> {
    // Inbound-link sources are rewritten to the new slug, so their
    // embeddings must be refreshed too.
    let (meta, affected) = Host::core(cx).with_cave(|cave| {
        let affected = cave.backlink_slugs(from).unwrap_or_default();
        let meta = cave.rename_note(from, to)?;
        Ok((meta, affected))
    })?;
    schedule_index_removal(from.to_string(), cx);
    schedule_index_update(meta.slug.clone(), cx);
    for slug in affected {
        schedule_index_update(slug, cx);
    }
    CaveEvents::emit(
        CaveEvent::NoteRenamed {
            from: from.to_string(),
            to: meta.slug.clone(),
        },
        cx,
    );
    Ok(meta)
}

/// Rename the folder at `path` to `new_name` (same parent) and return its
/// new path.
pub fn rename_folder(path: &str, new_name: &str, cx: &mut App) -> Result<String, CaveError> {
    Host::core(cx).with_cave(|cave| cave.rename_folder(Path::new(path), new_name))?;
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(join(parent_of(path), new_name))
}

/// Move the note `slug` into `folder` (`None`: the cave root).
pub fn move_note(
    slug: &str,
    folder: Option<&str>,
    cx: &mut App,
) -> Result<DocumentMeta, CaveError> {
    let meta = Host::core(cx).with_cave(|cave| cave.move_note(slug, folder.map(Path::new)))?;
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(meta)
}

/// Move the folder at `path` into `parent` (`None`: the cave root) and
/// return its new path.
pub fn move_folder(path: &str, parent: Option<&str>, cx: &mut App) -> Result<String, CaveError> {
    Host::core(cx).with_cave(|cave| cave.move_folder(Path::new(path), parent.map(Path::new)))?;
    CaveEvents::emit(CaveEvent::NotesChanged, cx);
    Ok(join(parent, name_of(path)))
}

/// Delete the note `slug`.
pub fn delete_note(slug: &str, cx: &mut App) -> Result<(), CaveError> {
    Host::core(cx).with_cave(|cave| cave.delete_note(slug))?;
    schedule_index_removal(slug.to_string(), cx);
    CaveEvents::emit(CaveEvent::NotesRemoved(vec![slug.to_string()]), cx);
    Ok(())
}

/// Delete the folder at `path` with everything in it.
pub fn delete_folder(path: &str, cx: &mut App) -> Result<(), CaveError> {
    let prefix = format!("{path}/");
    let removed = Host::core(cx).with_cave(|cave| {
        let removed: Vec<String> = cave
            .list_notes()?
            .into_iter()
            .filter(|meta| meta.relative_path.starts_with(&prefix))
            .map(|meta| meta.slug)
            .collect();
        cave.delete_folder(Path::new(path))?;
        Ok(removed)
    })?;
    for slug in &removed {
        schedule_index_removal(slug.clone(), cx);
    }
    CaveEvents::emit(CaveEvent::NotesRemoved(removed), cx);
    Ok(())
}

/// Queue a debounced embedding update for a created or changed note.
fn schedule_index_update(slug: String, cx: &App) {
    if let Some(index) = Host::core(cx).vector_index() {
        index.schedule_note_update(slug);
    }
}

/// Drop the embedding of a note that no longer exists under `slug`.
fn schedule_index_removal(slug: String, cx: &App) {
    if let Some(index) = Host::core(cx).vector_index() {
        runtime::spawn(async move { index.remove_note(&slug).await });
    }
}

/// `name` inside the folder `parent` (`None`: the cave root).
fn join(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(parent) if !parent.is_empty() => format!("{parent}/{name}"),
        _ => name.to_string(),
    }
}

/// The folder containing the cave-relative `path`, `None` at the root.
pub fn parent_of(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(parent, _)| parent)
}

/// The last component of the cave-relative `path`.
fn name_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_join_and_split_on_slashes() {
        assert_eq!(join(None, "a"), "a");
        assert_eq!(join(Some(""), "a"), "a");
        assert_eq!(join(Some("x/y"), "a"), "x/y/a");
        assert_eq!(parent_of("x/y/a"), Some("x/y"));
        assert_eq!(parent_of("a"), None);
        assert_eq!(name_of("x/y/a"), "a");
        assert_eq!(name_of("a"), "a");
    }
}
