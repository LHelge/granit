//! The `granit://` custom URI scheme.
//!
//! Serves files from the open cave to the app's webviews, so rendered
//! markdown can reference local images and the presentation window can load
//! template CSS and its assets. Routes are the first path segment:
//!
//! - `cave/<path>` — any file under the cave root.
//! - `presentation/<path>` — files under `.granit/presentations/`; when no
//!   such file exists, `presentation/<slug>` is the rendered presentation
//!   page of the note `slug`.
//!
//! Requests are matched by path, not host, because platforms spell custom
//! schemes differently: macOS and Linux use `granit://localhost/cave/a.png`,
//! while WebView2 on Windows (and Android) maps custom schemes to
//! `http://granit.localhost/cave/a.png`. [`base_url`] hides that difference.
//!
//! Every served path is canonicalized and checked to stay under its route's
//! root, so `..` segments and symlinks that escape the cave are rejected.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use tauri::http::{header, Request, Response, StatusCode};

use crate::cave::{CaveError, PRESENTATIONS_DIR};
use crate::commands::{render_presentation_page, AppState};

/// Scheme name registered with Tauri.
pub const SCHEME: &str = "granit";

/// First path segment of the route serving files from the cave root.
pub const ROUTE_CAVE: &str = "cave";
/// First path segment of the route serving files from `.granit/presentations/`.
pub const ROUTE_PRESENTATION: &str = "presentation";
/// First path segment of the route serving bundled app assets.
pub const ROUTE_ASSET: &str = "asset";

/// The mermaid bundle (`build/mermaid.js`, staged by the build script) that
/// the presentation page loads to render diagrams. Empty when the JS build
/// had not run at compile time.
const MERMAID_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mermaid.js"));
/// File name of the mermaid bundle under the asset route.
pub const MERMAID_ASSET: &str = "mermaid.js";

/// Origin the webview uses for scheme URLs, with a trailing slash.
pub fn base_url() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "http://granit.localhost/"
    } else {
        "granit://localhost/"
    }
}

/// URL serving the file at `relative` (forward-slash path under the cave root).
pub fn cave_file_url(relative: &str) -> String {
    route_url(ROUTE_CAVE, relative)
}

/// URL serving the file at `relative` (forward-slash path under
/// `.granit/presentations/`).
pub fn presentation_url(relative: &str) -> String {
    route_url(ROUTE_PRESENTATION, relative)
}

/// URL of a bundled app asset (`mermaid.js`).
pub fn asset_url(name: &str) -> String {
    route_url(ROUTE_ASSET, name)
}

/// URL of the presentation page for the note `slug`.
pub fn presentation_page_url(slug: &str) -> Result<tauri::Url, CaveError> {
    tauri::Url::parse(&route_url(ROUTE_PRESENTATION, slug))
        .map_err(|e| CaveError::Window(format!("invalid presentation URL: {e}")))
}

/// Characters percent-encoded inside a path segment: everything except the
/// RFC 3986 unreserved set.
const SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

fn route_url(route: &str, relative: &str) -> String {
    let mut url = String::from(base_url());
    url.push_str(route);
    for segment in relative.split('/').filter(|s| !s.is_empty()) {
        url.push('/');
        url.push_str(&utf8_percent_encode(segment, SEGMENT_ENCODE_SET).to_string());
    }
    url
}

/// Split a request path (`/cave/a/b%20c.png`) into its route and the
/// percent-decoded relative path under that route.
///
/// Segments are decoded individually and re-joined, so an encoded slash
/// (`%2F`) cannot smuggle a separator past later validation as one segment.
/// Returns `None` for an empty route or an undecodable segment.
fn split_route(path: &str) -> Option<(&str, PathBuf)> {
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    let route = segments.next()?;
    let mut relative = PathBuf::new();
    for segment in segments {
        let decoded = percent_decode_str(segment).decode_utf8().ok()?;
        relative.push(decoded.as_ref());
    }
    Some((route, relative))
}

/// Resolve `relative` under `root`, returning the canonical path of an
/// existing regular file that lies inside `root`.
///
/// Rejects empty paths, absolute paths, `..` components, and any
/// file whose canonical location (after following symlinks) is outside the
/// canonical root. A missing file or root also yields `None`.
pub(crate) fn resolve_scoped_path(root: &Path, relative: &Path) -> Option<PathBuf> {
    let mut components = relative.components().peekable();
    components.peek()?;
    if !components.all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    let canonical_root = std::fs::canonicalize(root).ok()?;
    let candidate = std::fs::canonicalize(canonical_root.join(relative)).ok()?;
    (candidate.starts_with(&canonical_root) && candidate.is_file()).then_some(candidate)
}

/// Content type for a served file, chosen by extension.
pub(crate) fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "txt" | "md" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

/// Serve one request against the app state. Never panics: every failure,
/// including no open cave, becomes a 404.
pub(crate) fn respond(
    state: &AppState,
    request: &Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    match serve(state, request.uri().path()) {
        Some((body, content_type)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(Cow::Owned(body)),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(Cow::Borrowed(b"Not found" as &[u8])),
    }
    .expect("static response headers are valid")
}

/// Route a request path to its content. `None` means 404.
fn serve(state: &AppState, path: &str) -> Option<(Vec<u8>, &'static str)> {
    let (route, relative) = split_route(path)?;
    if route == ROUTE_ASSET {
        return serve_asset(&relative);
    }
    let cave_root = state.active_cave_path()?;
    match route {
        ROUTE_CAVE => serve_file(&cave_root, &relative),
        // A template file wins over a note of the same name; the page is
        // only served for a bare slug (one segment).
        ROUTE_PRESENTATION => serve_file(&cave_root.join(PRESENTATIONS_DIR), &relative)
            .or_else(|| serve_page(state, &relative)),
        _ => None,
    }
}

/// Bundled assets need no cave: they are compiled into the binary.
fn serve_asset(relative: &Path) -> Option<(Vec<u8>, &'static str)> {
    match relative.to_str()? {
        MERMAID_ASSET => Some((MERMAID_JS.to_vec(), "text/javascript; charset=utf-8")),
        _ => None,
    }
}

fn serve_file(root: &Path, relative: &Path) -> Option<(Vec<u8>, &'static str)> {
    let file = resolve_scoped_path(root, relative)?;
    let body = std::fs::read(&file).ok()?;
    Some((body, content_type(&file)))
}

fn serve_page(state: &AppState, relative: &Path) -> Option<(Vec<u8>, &'static str)> {
    let mut components = relative.components();
    let Some(Component::Normal(slug)) = components.next() else {
        return None;
    };
    if components.next().is_some() {
        return None;
    }
    let page = render_presentation_page(state, slug.to_str()?).ok()?;
    Some((page.into_bytes(), "text/html; charset=utf-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(format!("{}{}", base_url().trim_end_matches('/'), path))
            .body(Vec::new())
            .unwrap()
    }

    fn cave_with_files() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::create_dir_all(dir.path().join(PRESENTATIONS_DIR)).unwrap();
        std::fs::write(dir.path().join("logo.png"), b"png-bytes").unwrap();
        std::fs::write(dir.path().join("sub/pic ture.jpg"), b"jpg-bytes").unwrap();
        std::fs::write(
            dir.path().join(PRESENTATIONS_DIR).join("default-dark.css"),
            b"body{}",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("talk.md"),
            "---\npresentation: default-dark\n---\n# One\n\n---\n\n# Two\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("plain.md"), "# Not presentable\n").unwrap();
        std::fs::write(
            dir.path().join("orphan.md"),
            "---\npresentation: corporate\n---\n# Missing template\n",
        )
        .unwrap();
        dir
    }

    /// App state with the cave at `dir` open.
    fn state_for(dir: &tempfile::TempDir) -> AppState {
        let state = AppState::new(granit_types::AppConfig::default());
        state.set_cave(Some(
            crate::cave::Cave::open(dir.path().to_path_buf()).unwrap(),
        ));
        state
    }

    fn respond_in(dir: &tempfile::TempDir, path: &str) -> Response<Cow<'static, [u8]>> {
        respond(&state_for(dir), &request(path))
    }

    // ── URL building ────────────────────────────────────────────────

    #[test]
    fn test_cave_file_url_encodes_segments() {
        let url = cave_file_url("sub/pic ture#1.jpg");
        assert_eq!(url, format!("{}cave/sub/pic%20ture%231.jpg", base_url()));
    }

    #[test]
    fn test_route_url_drops_empty_segments() {
        assert_eq!(
            cave_file_url("/a//b.png"),
            format!("{}cave/a/b.png", base_url())
        );
    }

    // ── Path scoping ────────────────────────────────────────────────

    #[test]
    fn test_resolve_scoped_path_accepts_normal_file() {
        let dir = cave_with_files();
        let resolved = resolve_scoped_path(dir.path(), Path::new("sub/pic ture.jpg")).unwrap();
        assert!(resolved.ends_with("sub/pic ture.jpg"));
    }

    #[test]
    fn test_resolve_scoped_path_rejects_traversal_and_absolute() {
        let dir = cave_with_files();
        let outside = dir.path().parent().unwrap().join("outside.txt");
        std::fs::write(&outside, b"secret").unwrap();

        for bad in ["../outside.txt", "sub/../../outside.txt", "..", ".", ""] {
            assert!(
                resolve_scoped_path(dir.path(), Path::new(bad)).is_none(),
                "{bad:?} should be rejected"
            );
        }
        assert!(resolve_scoped_path(dir.path(), &outside).is_none());
        // A `.` in the middle folds away and stays inside the root.
        assert!(resolve_scoped_path(dir.path(), Path::new("sub/./pic ture.jpg")).is_some());
        let _ = std::fs::remove_file(outside);
    }

    #[test]
    fn test_resolve_scoped_path_rejects_missing_file_and_directory() {
        let dir = cave_with_files();
        assert!(resolve_scoped_path(dir.path(), Path::new("nope.png")).is_none());
        assert!(resolve_scoped_path(dir.path(), Path::new("sub")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_scoped_path_rejects_escaping_symlink() {
        let dir = cave_with_files();
        let secret_dir = tempfile::tempdir().unwrap();
        let secret = secret_dir.path().join("secret.txt");
        std::fs::write(&secret, b"secret").unwrap();
        std::os::unix::fs::symlink(&secret, dir.path().join("link.txt")).unwrap();

        assert!(resolve_scoped_path(dir.path(), Path::new("link.txt")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_scoped_path_accepts_symlink_inside_root() {
        let dir = cave_with_files();
        std::os::unix::fs::symlink(dir.path().join("logo.png"), dir.path().join("alias.png"))
            .unwrap();

        let resolved = resolve_scoped_path(dir.path(), Path::new("alias.png")).unwrap();
        assert!(resolved.ends_with("logo.png"));
    }

    // ── Content types ───────────────────────────────────────────────

    #[test]
    fn test_content_type_by_extension() {
        assert_eq!(content_type(Path::new("a.CSS")), "text/css; charset=utf-8");
        assert_eq!(content_type(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(content_type(Path::new("a.woff2")), "font/woff2");
        assert_eq!(content_type(Path::new("a.bin")), "application/octet-stream");
        assert_eq!(content_type(Path::new("noext")), "application/octet-stream");
    }

    // ── Request handling ────────────────────────────────────────────

    #[test]
    fn test_respond_serves_cave_file_with_decoded_path() {
        let dir = cave_with_files();
        let response = respond_in(&dir, "/cave/sub/pic%20ture.jpg");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(response.body().as_ref(), b"jpg-bytes");
    }

    #[test]
    fn test_respond_serves_presentation_file_from_presentations_dir() {
        let dir = cave_with_files();
        let response = respond_in(&dir, "/presentation/default-dark.css");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/css; charset=utf-8"
        );
        assert_eq!(response.body().as_ref(), b"body{}");
    }

    #[test]
    fn test_respond_presentation_route_cannot_reach_cave_root() {
        let dir = cave_with_files();
        let response = respond_in(&dir, "/presentation/../../logo.png");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = respond_in(&dir, "/presentation/logo.png");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_respond_404_for_missing_file_unknown_route_and_no_cave() {
        let dir = cave_with_files();
        assert_eq!(
            respond_in(&dir, "/cave/missing.png").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            respond_in(&dir, "/other/logo.png").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(respond_in(&dir, "/").status(), StatusCode::NOT_FOUND);
        assert_eq!(
            respond(
                &AppState::new(granit_types::AppConfig::default()),
                &request("/cave/logo.png")
            )
            .status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_respond_rejects_encoded_traversal() {
        let dir = cave_with_files();
        let outside = dir.path().parent().unwrap().join("outside2.txt");
        std::fs::write(&outside, b"secret").unwrap();

        for path in [
            "/cave/%2E%2E/outside2.txt",
            "/cave/sub%2F..%2F..%2Foutside2.txt",
        ] {
            assert_eq!(
                respond_in(&dir, path).status(),
                StatusCode::NOT_FOUND,
                "{path}"
            );
        }
        let _ = std::fs::remove_file(outside);
    }

    // ── Presentation page route ─────────────────────────────────────

    #[test]
    fn test_presentation_page_is_served_for_a_presentable_note() {
        let dir = cave_with_files();
        let response = respond_in(&dir, "/presentation/talk");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/html; charset=utf-8"
        );
        let page = String::from_utf8(response.body().to_vec()).unwrap();
        assert!(
            page.contains("<section class=\"slide active\""),
            "got: {page}"
        );
        assert!(
            page.contains(&presentation_url("default-dark.css")),
            "got: {page}"
        );
        // Case-insensitive like every note lookup.
        assert_eq!(
            respond_in(&dir, "/presentation/TALK").status(),
            StatusCode::OK
        );
    }

    #[test]
    fn test_presentation_page_error_paths_are_404() {
        let dir = cave_with_files();
        for path in [
            "/presentation/missing-note",
            "/presentation/plain",
            "/presentation/orphan",
            "/presentation/talk/extra",
            "/presentation/",
        ] {
            assert_eq!(
                respond_in(&dir, path).status(),
                StatusCode::NOT_FOUND,
                "{path}"
            );
        }
    }

    #[test]
    fn test_mermaid_asset_is_served_without_a_cave() {
        let state = AppState::new(granit_types::AppConfig::default());
        let response = respond(&state, &request("/asset/mermaid.js"));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
        assert_eq!(response.body().as_ref(), MERMAID_JS);
        assert_eq!(
            asset_url(MERMAID_ASSET),
            format!("{}asset/mermaid.js", base_url())
        );
        for path in ["/asset/other.js", "/asset/", "/asset/../mermaid.js"] {
            assert_eq!(
                respond(&state, &request(path)).status(),
                StatusCode::NOT_FOUND,
                "{path}"
            );
        }
    }

    #[test]
    fn test_presentation_page_url_round_trips_through_the_route() {
        let url = presentation_page_url("my talk").unwrap();
        assert_eq!(
            url.as_str(),
            format!("{}presentation/my%20talk", base_url())
        );
        let (route, relative) = split_route(url.path()).unwrap();
        assert_eq!(route, ROUTE_PRESENTATION);
        assert_eq!(relative, Path::new("my talk"));
    }
}
