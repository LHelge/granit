//! URLs of the `granit://` custom URI scheme.
//!
//! Rendered markdown links note images and the presentation page links its
//! template CSS and the mermaid bundle through these URLs. Serving them is
//! the host's job (`src-tauri/src/scheme.rs`); this module only builds them.
//! Routes are the first path segment:
//!
//! - `cave/<path>` — any file under the cave root.
//! - `presentation/<path>` — files under `.granit/presentations/`, or the
//!   rendered presentation page of a bare note slug.
//! - `asset/<name>` — assets bundled into the app.
//!
//! Platforms spell custom schemes differently: macOS and Linux use
//! `granit://localhost/cave/a.png`, while WebView2 on Windows (and Android)
//! maps custom schemes to `http://granit.localhost/cave/a.png`. [`base_url`]
//! hides that difference.

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

/// Scheme name the host registers.
pub const SCHEME: &str = "granit";

/// First path segment of the route serving files from the cave root.
pub const ROUTE_CAVE: &str = "cave";
/// First path segment of the route serving files from `.granit/presentations/`.
pub const ROUTE_PRESENTATION: &str = "presentation";
/// First path segment of the route serving bundled app assets.
pub const ROUTE_ASSET: &str = "asset";

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

/// Characters percent-encoded inside a path segment: everything except the
/// RFC 3986 unreserved set.
const SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// URL of `relative` under `route`, each path segment percent-encoded.
pub fn route_url(route: &str, relative: &str) -> String {
    let mut url = String::from(base_url());
    url.push_str(route);
    for segment in relative.split('/').filter(|s| !s.is_empty()) {
        url.push('/');
        url.push_str(&utf8_percent_encode(segment, SEGMENT_ENCODE_SET).to_string());
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
