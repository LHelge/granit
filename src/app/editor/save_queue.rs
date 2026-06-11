use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DocumentKind {
    Note,
    Template,
}

/// A fully-captured snapshot of editor state at the moment a save is requested.
///
/// All persist logic operates on snapshots instead of reading signals during
/// async work, so that rapid switches between notes cannot cause the wrong
/// content to be written for the wrong slug.
#[derive(Clone)]
pub(super) struct PersistSnapshot {
    pub kind: DocumentKind,
    pub slug: String,
    pub name: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub icon: Option<String>,
    pub favorite: Option<bool>,
    /// Whether this save was initiated explicitly (Save button / Ctrl-S).
    /// Explicit saves toggle `saving`/`editing` state and update the active
    /// document on success. Auto-saves do not.
    pub explicit: bool,
}

impl PersistSnapshot {
    pub fn doc_key(&self) -> String {
        match self.kind {
            DocumentKind::Note => format!("note:{}", self.slug),
            DocumentKind::Template => format!("template:{}", self.slug),
        }
    }
}

/// Ordered queue of pending save snapshots.
///
/// Only one save is in flight at any time. When a new snapshot is enqueued
/// for a `doc_key` that is already pending (but not yet started), the existing
/// entry is replaced with the newer snapshot (latest-wins per document), so a
/// burst of switches away from the same note collapses into a single write.
/// The `explicit` flag is preserved on replacement: if either the existing or
/// incoming snapshot is explicit, the merged entry remains explicit so UI
/// state (`saving`, `editing`) is always finalized.
pub(super) struct SaveQueue {
    pending: VecDeque<PersistSnapshot>,
    in_flight: bool,
}

impl SaveQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            in_flight: false,
        }
    }

    /// Enqueue `snapshot` and return `true` if the caller should start
    /// draining the queue (i.e. nothing is currently in flight).
    pub fn enqueue(&mut self, mut snapshot: PersistSnapshot) -> bool {
        let key = snapshot.doc_key();
        if let Some(existing) = self.pending.iter_mut().find(|s| s.doc_key() == key) {
            snapshot.explicit = snapshot.explicit || existing.explicit;
            *existing = snapshot;
        } else {
            self.pending.push_back(snapshot);
        }
        if self.in_flight {
            false
        } else {
            self.in_flight = true;
            true
        }
    }

    pub fn take_next(&mut self) -> Option<PersistSnapshot> {
        let next = self.pending.pop_front();
        if next.is_none() {
            self.in_flight = false;
        }
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn snapshot(kind: DocumentKind, slug: &str, content: &str, explicit: bool) -> PersistSnapshot {
        PersistSnapshot {
            kind,
            slug: slug.to_string(),
            name: slug.to_string(),
            content: content.to_string(),
            tags: None,
            icon: None,
            favorite: None,
            explicit,
        }
    }

    #[wasm_bindgen_test]
    fn first_enqueue_starts_draining_later_ones_do_not() {
        let mut queue = SaveQueue::new();
        assert!(queue.enqueue(snapshot(DocumentKind::Note, "a", "1", false)));
        assert!(!queue.enqueue(snapshot(DocumentKind::Note, "b", "2", false)));
    }

    #[wasm_bindgen_test]
    fn enqueue_collapses_pending_saves_per_document_latest_wins() {
        let mut queue = SaveQueue::new();
        queue.enqueue(snapshot(DocumentKind::Note, "a", "old", false));
        queue.enqueue(snapshot(DocumentKind::Note, "a", "new", false));

        let next = queue.take_next().unwrap();
        assert_eq!(next.content, "new");
        assert!(queue.take_next().is_none());
    }

    #[wasm_bindgen_test]
    fn enqueue_preserves_explicit_flag_when_collapsing() {
        let mut queue = SaveQueue::new();
        queue.enqueue(snapshot(DocumentKind::Note, "a", "old", true));
        queue.enqueue(snapshot(DocumentKind::Note, "a", "new", false));

        let next = queue.take_next().unwrap();
        assert!(next.explicit, "explicit flag must survive replacement");
    }

    #[wasm_bindgen_test]
    fn same_slug_different_kind_does_not_collapse() {
        let mut queue = SaveQueue::new();
        queue.enqueue(snapshot(DocumentKind::Note, "a", "note", false));
        queue.enqueue(snapshot(DocumentKind::Template, "a", "template", false));

        assert_eq!(queue.take_next().unwrap().content, "note");
        assert_eq!(queue.take_next().unwrap().content, "template");
    }

    #[wasm_bindgen_test]
    fn take_next_resets_in_flight_when_drained() {
        let mut queue = SaveQueue::new();
        queue.enqueue(snapshot(DocumentKind::Note, "a", "1", false));
        assert!(queue.take_next().is_some());
        assert!(queue.take_next().is_none());
        // Queue is idle again: the next enqueue must start a new drain.
        assert!(queue.enqueue(snapshot(DocumentKind::Note, "b", "2", false)));
    }
}
