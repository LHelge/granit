//! The presentation window: a second Tauri window showing a note as slides.
//!
//! The window is a plain webview loading the backend-rendered page from the
//! `granit://` presentation route; it has no app code and only the window
//! permissions its page script needs (fullscreen toggle, close). One window
//! at a time, with a fixed label: starting a presentation while it is open
//! navigates it to the new note and refocuses it.
//!
//! The backend remembers what the window shows so the note and template
//! commands can keep it in sync: saving the shown note or its template
//! reloads the page (the slide index lives in the URL hash, so the position
//! survives), while renaming or deleting the shown note, or opening another
//! cave, closes the window.

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use super::AppState;
use crate::cave::{Cave, CaveError};
use crate::markdown::Markdown;
use crate::scheme;

/// Label of the single presentation window.
pub const PRESENTATION_WINDOW_LABEL: &str = "presentation";

/// What the presentation window is currently showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShownPresentation {
    pub slug: String,
    pub template: String,
}

/// Resolve a note name to its canonical slug and presentation template,
/// requiring both the `presentation` frontmatter field and the template
/// file to exist. There is deliberately no fallback to a default template.
pub(crate) fn resolve_presentation(cave: &Cave, name: &str) -> Result<(String, String), CaveError> {
    let slug = cave.resolve_slug(name)?;
    let raw = cave.read_note_raw(&slug)?;
    let template = Markdown::new(&raw)
        .presentation()
        .ok_or_else(|| CaveError::NoPresentation(slug.clone()))?;
    cave.presentation_path(&template)?;
    Ok((slug, template))
}

/// Render the presentation page for a note; the body of the page route.
pub(crate) fn render_presentation_page(state: &AppState, name: &str) -> Result<String, CaveError> {
    state.with_cave(|cave| {
        let (slug, template) = resolve_presentation(cave, name)?;
        let raw = cave.read_note_raw(&slug)?;
        let css_href = scheme::presentation_url(&format!("{template}.css"));
        Ok(Markdown::new(&raw)
            .with_image_base(cave.note_dir(&slug)?)
            .render_presentation(&slug, &css_href))
    })
}

/// Open the presentation window for a note, or point the open window at it.
#[tauri::command]
pub(crate) fn start_presentation(
    name: String,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), CaveError> {
    let (slug, template) = state.with_cave(|cave| resolve_presentation(cave, &name))?;
    let url = scheme::presentation_page_url(&slug)?;
    *state.lock_presentation() = Some(ShownPresentation {
        slug: slug.clone(),
        template,
    });

    if let Some(window) = app.get_webview_window(PRESENTATION_WINDOW_LABEL) {
        window.navigate(url)?;
        window.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        &app,
        PRESENTATION_WINDOW_LABEL,
        WebviewUrl::CustomProtocol(url),
    )
    .title(format!("{slug} - Presentation"))
    .inner_size(1280.0, 720.0)
    .min_inner_size(320.0, 180.0)
    .build()?;

    // Forget what the window showed once the user closes it.
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            *handle.state::<AppState>().lock_presentation() = None;
        }
    });
    Ok(())
}

/// Close the presentation window if it is open and forget what it showed.
pub(crate) fn close_presentation_window(app: &tauri::AppHandle, state: &AppState) {
    *state.lock_presentation() = None;
    if let Some(window) = app.get_webview_window(PRESENTATION_WINDOW_LABEL) {
        let _ = window.close();
    }
}

fn reload_presentation_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(PRESENTATION_WINDOW_LABEL) {
        let _ = window.eval("location.reload()");
    }
}

fn shown(state: &AppState) -> Option<ShownPresentation> {
    state.lock_presentation().clone()
}

/// A note was saved: reload the window if it shows that note.
pub(crate) fn note_saved(app: &tauri::AppHandle, state: &AppState, slug: &str) {
    if shown(state).is_some_and(|p| p.slug == slug) {
        reload_presentation_window(app);
    }
}

/// A note was renamed or deleted: close the window if it showed that note.
pub(crate) fn note_removed(app: &tauri::AppHandle, state: &AppState, slug: &str) {
    if shown(state).is_some_and(|p| p.slug == slug) {
        close_presentation_window(app, state);
    }
}

/// A presentation template was saved: reload the window if the shown note
/// uses it.
pub(crate) fn template_saved(app: &tauri::AppHandle, state: &AppState, template: &str) {
    if shown(state).is_some_and(|p| p.template == template) {
        reload_presentation_window(app);
    }
}

/// The agent changed the cave in ways the commands above never see: close
/// the window if its note is gone, otherwise reload it when `reload` is set.
pub(crate) fn sync_after_agent(app: &tauri::AppHandle, state: &AppState, reload: bool) {
    let Some(shown) = shown(state) else {
        return;
    };
    let exists = state
        .with_cave(|cave| Ok(cave.resolve_slug(&shown.slug).is_ok()))
        .unwrap_or(false);
    if !exists {
        close_presentation_window(app, state);
    } else if reload {
        reload_presentation_window(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use granit_types::AppConfig;

    fn cave_with(note: &str) -> (tempfile::TempDir, Cave) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("talk.md"), note).unwrap();
        let mut cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.seed_presentation_defaults().unwrap();
        (dir, cave)
    }

    #[test]
    fn test_resolve_presentation_requires_field_and_template() {
        let (_dir, cave) = cave_with("---\npresentation: default-dark\n---\n# Hi\n");
        assert_eq!(
            resolve_presentation(&cave, "TALK").unwrap(),
            ("talk".to_string(), "default-dark".to_string())
        );

        let (_dir, cave) = cave_with("# No field\n");
        assert!(matches!(
            resolve_presentation(&cave, "talk"),
            Err(CaveError::NoPresentation(_))
        ));

        let (_dir, cave) = cave_with("---\npresentation: corporate\n---\n# Hi\n");
        assert!(matches!(
            resolve_presentation(&cave, "talk"),
            Err(CaveError::PresentationNotFound(_))
        ));
        assert!(matches!(
            resolve_presentation(&cave, "missing"),
            Err(CaveError::NotFound(_))
        ));
    }

    #[test]
    fn test_render_presentation_page_links_template_and_resolves_images() {
        let (dir, cave) = cave_with(
            "---\npresentation: default-light\n---\n# One\n\n![](img/a.png)\n\n---\n\n# Two\n",
        );
        let state = AppState::new(AppConfig::default());
        state.set_cave(Some(cave));

        let page = render_presentation_page(&state, "talk").unwrap();
        assert!(page.contains("<title>talk</title>"), "got: {page}");
        assert!(
            page.contains(&scheme::presentation_url("default-light.css")),
            "got: {page}"
        );
        assert!(
            page.contains(&scheme::cave_file_url("img/a.png")),
            "got: {page}"
        );
        assert_eq!(page.matches("<section class=\"slide").count(), 2);
        drop(dir);

        // No cave: the cave error surfaces rather than a panic.
        let state = AppState::new(AppConfig::default());
        assert!(matches!(
            render_presentation_page(&state, "talk"),
            Err(CaveError::NoCaveOpen)
        ));
    }
}
