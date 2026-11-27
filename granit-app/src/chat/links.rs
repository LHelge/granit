//! Resolving the chat's `wiki:` links against the cave.
//!
//! The rewriter turns `[[target]]` into a link to `wiki:target`. The view's
//! `.link()` hook runs on every draw and must not lock anything, so the
//! panel resolves targets ahead of time into a [`LinkTargets`] snapshot
//! (when they arrive, and again after every cave change) and the hook only
//! looks them up: a resolved target is re-pointed at `wiki:<slug>` or
//! `wiki:<slug>#<anchor>`, a broken one is drawn as plain text.

use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::base::text::Link;
use granit_core::cave::Cave;

use super::wiki_links::WIKI_SCHEME;

/// Wiki-link targets by lowercase name, with what they resolve to (`None`:
/// broken). Shared with the view's link hook as an immutable snapshot.
#[derive(Clone, Debug, Default)]
pub struct LinkTargets(Arc<HashMap<String, Option<String>>>);

impl LinkTargets {
    /// Resolve `targets` not seen yet (all of them when `refresh`) with
    /// `resolve`, which is the cave's [`Cave::resolve_link`].
    pub fn resolve(
        &mut self,
        targets: impl IntoIterator<Item = String>,
        refresh: bool,
        resolve: impl Fn(&str) -> Option<String>,
    ) {
        let mut map = (*self.0).clone();
        let mut names: Vec<String> = targets.into_iter().map(|t| t.to_lowercase()).collect();
        if refresh {
            names.extend(map.keys().cloned());
        }
        names.retain(|name| refresh || !map.contains_key(name));
        if names.is_empty() {
            return;
        }
        for name in names {
            let href = resolve(&name);
            map.insert(name, href);
        }
        self.0 = Arc::new(map);
    }

    /// Resolve against an open cave.
    pub fn resolve_in(&mut self, targets: Vec<String>, refresh: bool, cave: &Cave) {
        self.resolve(targets, refresh, |name| cave.resolve_link(name));
    }

    /// Forget every target.
    pub fn clear(&mut self) {
        self.0 = Arc::default();
    }

    /// What `target` resolves to, when it has been resolved and exists.
    pub fn href(&self, target: &str) -> Option<&str> {
        self.0.get(&target.to_lowercase())?.as_deref()
    }

    /// The view's link hook: re-point a `wiki:` link at its resolved note,
    /// or draw it as text when it is broken (or not resolved yet). Other
    /// links are left alone.
    pub fn retarget(&self, link: &Link) -> Option<Link> {
        let Some(target) = link.url.strip_prefix(WIKI_SCHEME) else {
            return Some(link.clone());
        };
        let href = self.href(target)?;
        Some(link.clone().url(format!("{WIKI_SCHEME}{href}")))
    }
}

/// Where a clicked link goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// A note of the cave, optionally at a heading anchor.
    Note {
        slug: String,
        anchor: Option<String>,
    },
    /// Anything else, opened by the system.
    External(String),
}

/// Classify the URL of a clicked link (after [`LinkTargets::retarget`]).
pub fn classify(url: &str) -> LinkTarget {
    match url.strip_prefix(WIKI_SCHEME) {
        Some(href) => {
            let (slug, anchor) = match href.split_once('#') {
                Some((slug, anchor)) => (slug, Some(anchor.to_string())),
                None => (href, None),
            };
            LinkTarget::Note {
                slug: slug.to_string(),
                anchor,
            }
        }
        None => LinkTarget::External(url.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets() -> LinkTargets {
        let mut targets = LinkTargets::default();
        targets.resolve(
            ["Note".to_string(), "Volvo".to_string(), "ghost".to_string()],
            false,
            |name| match name {
                "note" => Some("Note".to_string()),
                "volvo" => Some("cars#volvo".to_string()),
                _ => None,
            },
        );
        targets
    }

    #[test]
    fn resolved_links_point_at_their_note_and_broken_ones_are_text() {
        let targets = targets();
        let link = |url: &str| targets.retarget(&Link::new(url.to_string())).map(|l| l.url);
        assert_eq!(link("wiki:note").as_deref(), Some("wiki:Note"));
        assert_eq!(link("wiki:Volvo").as_deref(), Some("wiki:cars#volvo"));
        assert_eq!(link("wiki:ghost"), None);
        assert_eq!(link("wiki:never-seen"), None);
        assert_eq!(
            link("https://example.com").as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn refreshing_re_resolves_known_targets() {
        let mut targets = targets();
        targets.resolve(Vec::new(), true, |name| Some(format!("new-{name}")));
        assert_eq!(targets.href("ghost"), Some("new-ghost"));
    }

    #[test]
    fn clicked_urls_are_classified() {
        assert_eq!(
            classify("wiki:cars#volvo"),
            LinkTarget::Note {
                slug: "cars".into(),
                anchor: Some("volvo".into())
            }
        );
        assert_eq!(
            classify("wiki:a"),
            LinkTarget::Note {
                slug: "a".into(),
                anchor: None
            }
        );
        assert_eq!(
            classify("https://x.y"),
            LinkTarget::External("https://x.y".into())
        );
    }
}
