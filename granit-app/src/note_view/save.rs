//! Save bookkeeping for the note open in the editor, kept free of GPUI so it
//! can be tested without a window.
//!
//! A [`NoteSession`] remembers the body last read from or written to disk and
//! whether the user has edited since. [`NoteSession::snapshot`] captures what
//! a save would write, as a [`SaveSnapshot`] of (slug, text): the write goes
//! through that snapshot, never the live editor, so a note switch between
//! taking it and writing it cannot send one note's text to another. A
//! snapshot is only produced when the note was edited *and* the text differs
//! from the saved body, so loading a note, or typing a character and deleting
//! it again, never touches the file.

use std::borrow::Cow;
use std::fmt::Display;
use std::time::Duration;

/// Quiet period after the last edit before the debounced save runs (the
/// web frontend's holdoff).
pub const AUTOSAVE_DELAY: Duration = Duration::from_secs(2);

/// What the user should see about the open note's save state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    /// Matches the file on disk as far as this session knows.
    Clean,
    /// Edited since the last save; a save is pending.
    Edited,
    /// The last save attempt failed; the edits are still in the editor.
    Failed(String),
}

/// Result of a flush.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlushOutcome {
    /// Nothing to write: not edited, or the text equals the saved body.
    Unchanged,
    /// The text was written.
    Saved,
    /// The write failed; the session stays dirty.
    Failed(String),
}

/// The line endings of a loaded body.
///
/// The editor splits lines on `\n` and inserts a bare `\n` on Enter (and
/// normalizes pasted text to `\n`), so in a note that uses CRLF throughout,
/// every new line would be written with a bare LF. A CRLF note is therefore
/// written back with CRLF on every line; anything else (LF, mixed, lone CR,
/// no line break at all) is written as the editor has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    /// `Crlf` when `body` has line breaks and every `\n` is preceded by `\r`.
    pub fn of(body: &str) -> Self {
        let bytes = body.as_bytes();
        let mut any = false;
        for (ix, &byte) in bytes.iter().enumerate() {
            if byte == b'\n' {
                if ix == 0 || bytes[ix - 1] != b'\r' {
                    return Self::Lf;
                }
                any = true;
            }
        }
        if any {
            Self::Crlf
        } else {
            Self::Lf
        }
    }

    /// `text` with this line ending: for `Crlf`, every bare `\n` becomes
    /// `\r\n`.
    pub fn apply<'a>(self, text: &'a str) -> Cow<'a, str> {
        if self == Self::Lf || !has_bare_lf(text) {
            return Cow::Borrowed(text);
        }
        let mut out = String::with_capacity(text.len() + text.len() / 32);
        let mut previous = None;
        for ch in text.chars() {
            if ch == '\n' && previous != Some('\r') {
                out.push('\r');
            }
            out.push(ch);
            previous = Some(ch);
        }
        Cow::Owned(out)
    }
}

fn has_bare_lf(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes
        .iter()
        .enumerate()
        .any(|(ix, &byte)| byte == b'\n' && (ix == 0 || bytes[ix - 1] != b'\r'))
}

/// What kind of document the editor holds. Notes and templates have
/// separate slug namespaces, so a slug alone does not name a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocKind {
    /// A note of the cave.
    #[default]
    Note,
    /// A note template in `.granit/templates/`.
    Template,
}

/// What one save writes: the document and its full body, captured when the
/// save was requested.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveSnapshot {
    pub kind: DocKind,
    pub slug: String,
    pub text: String,
}

/// Save state of one open note (or template).
#[derive(Debug)]
pub struct NoteSession {
    kind: DocKind,
    slug: String,
    /// The body as last read from or written to disk.
    saved: String,
    line_ending: LineEnding,
    /// Set by a user edit, cleared by a flush that leaves disk up to date.
    dirty: bool,
    /// Bumped on every user edit, so a debounce timer can tell whether a
    /// newer edit arrived while it was waiting.
    edit_generation: u64,
    status: SaveStatus,
}

impl NoteSession {
    /// Start a session for a note whose body was just read from disk.
    pub fn new(slug: impl Into<String>, body: impl Into<String>) -> Self {
        let saved = body.into();
        Self {
            kind: DocKind::Note,
            slug: slug.into(),
            line_ending: LineEnding::of(&saved),
            saved,
            dirty: false,
            edit_generation: 0,
            status: SaveStatus::Clean,
        }
    }

    /// The session as one for a document of `kind` (a note by default).
    pub fn of_kind(mut self, kind: DocKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn kind(&self) -> DocKind {
        self.kind
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// Whether `snapshot` was taken of this document (under its current
    /// name).
    fn is_for(&self, snapshot: &SaveSnapshot) -> bool {
        snapshot.kind == self.kind && snapshot.slug == self.slug
    }

    /// The body as last read from or written to disk.
    pub fn saved(&self) -> &str {
        &self.saved
    }

    pub fn status(&self) -> &SaveStatus {
        &self.status
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The note was renamed to `slug`; pending edits go to the new name.
    pub fn retarget(&mut self, slug: impl Into<String>) {
        self.slug = slug.into();
    }

    /// Record a user edit. Returns the edit's generation for the debounce
    /// timer to check with [`Self::is_latest_edit`].
    pub fn mark_edited(&mut self) -> u64 {
        self.dirty = true;
        self.edit_generation += 1;
        if !matches!(self.status, SaveStatus::Failed(_)) {
            self.status = SaveStatus::Edited;
        }
        self.edit_generation
    }

    /// Whether `generation` is still the most recent edit.
    pub fn is_latest_edit(&self, generation: u64) -> bool {
        self.edit_generation == generation
    }

    /// What a save of the editor's `text` writes, or `None` when there is
    /// nothing to write (not edited, or equal to the saved body; the session
    /// is then clean).
    pub fn snapshot(&mut self, text: &str) -> Option<SaveSnapshot> {
        if !self.dirty {
            return None;
        }
        let text = self.line_ending.apply(text);
        if text == self.saved {
            self.dirty = false;
            self.status = SaveStatus::Clean;
            return None;
        }
        Some(SaveSnapshot {
            kind: self.kind,
            slug: self.slug.clone(),
            text: text.into_owned(),
        })
    }

    /// Record the result of writing `snapshot`. A failure keeps the session
    /// dirty so the next flush retries. A snapshot of another note (taken
    /// before a switch) only updates this session if it is for this note.
    pub fn finish<E: Display>(
        &mut self,
        snapshot: &SaveSnapshot,
        result: Result<(), E>,
    ) -> FlushOutcome {
        match result {
            Ok(()) => {
                if self.is_for(snapshot) {
                    self.saved.clone_from(&snapshot.text);
                    self.dirty = false;
                    self.status = SaveStatus::Clean;
                }
                FlushOutcome::Saved
            }
            Err(err) => {
                let message = err.to_string();
                if self.is_for(snapshot) {
                    self.status = SaveStatus::Failed(message.clone());
                }
                FlushOutcome::Failed(message)
            }
        }
    }

    /// Snapshot `text` and write it through `save(snapshot)`.
    #[cfg(test)]
    pub fn flush<E: Display>(
        &mut self,
        text: &str,
        save: impl FnOnce(&SaveSnapshot) -> Result<(), E>,
    ) -> FlushOutcome {
        let Some(snapshot) = self.snapshot(text) else {
            return FlushOutcome::Unchanged;
        };
        let result = save(&snapshot);
        self.finish(&snapshot, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Records every write and fails when `fail` is set.
    #[derive(Default)]
    struct Disk {
        writes: RefCell<Vec<(String, String)>>,
        fail: bool,
    }

    impl Disk {
        fn save(&self, snapshot: &SaveSnapshot) -> Result<(), String> {
            if self.fail {
                return Err("disk full".into());
            }
            self.writes
                .borrow_mut()
                .push((snapshot.slug.clone(), snapshot.text.clone()));
            Ok(())
        }

        fn write_count(&self) -> usize {
            self.writes.borrow().len()
        }
    }

    #[test]
    fn loading_a_note_does_not_write() {
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "body");
        assert_eq!(
            session.flush("body", |s| disk.save(s)),
            FlushOutcome::Unchanged
        );
        assert_eq!(disk.write_count(), 0);
        assert_eq!(session.status(), &SaveStatus::Clean);
    }

    #[test]
    fn unedited_note_is_not_written_even_if_text_differs() {
        // The editor may normalize text on load; without a user edit the
        // file must stay untouched.
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "$$x$$");
        session.flush("$$\nx\n$$", |s| disk.save(s));
        assert_eq!(disk.write_count(), 0);
    }

    #[test]
    fn edit_then_flush_writes_once() {
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "body");
        session.mark_edited();
        assert_eq!(session.status(), &SaveStatus::Edited);
        assert_eq!(
            session.flush("body!", |s| disk.save(s)),
            FlushOutcome::Saved
        );
        assert_eq!(
            disk.writes.borrow().as_slice(),
            &[("a".to_string(), "body!".to_string())]
        );
        assert!(!session.is_dirty());
        assert_eq!(session.status(), &SaveStatus::Clean);
        assert_eq!(session.saved(), "body!");

        // A second flush without a new edit writes nothing.
        session.flush("body!", |s| disk.save(s));
        assert_eq!(disk.write_count(), 1);
    }

    #[test]
    fn edit_reverted_before_flush_does_not_write() {
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "body");
        session.mark_edited();
        session.mark_edited();
        assert_eq!(
            session.flush("body", |s| disk.save(s)),
            FlushOutcome::Unchanged
        );
        assert_eq!(disk.write_count(), 0);
        assert!(!session.is_dirty());
    }

    #[test]
    fn failed_save_stays_dirty_and_retries() {
        let mut disk = Disk {
            fail: true,
            ..Default::default()
        };
        let mut session = NoteSession::new("a", "body");
        session.mark_edited();
        assert_eq!(
            session.flush("edited", |s| disk.save(s)),
            FlushOutcome::Failed("disk full".into())
        );
        assert!(session.is_dirty());
        assert_eq!(session.status(), &SaveStatus::Failed("disk full".into()));

        // A further edit keeps the failure visible until a save succeeds.
        session.mark_edited();
        assert_eq!(session.status(), &SaveStatus::Failed("disk full".into()));

        disk.fail = false;
        assert_eq!(
            session.flush("edited", |s| disk.save(s)),
            FlushOutcome::Saved
        );
        assert_eq!(session.status(), &SaveStatus::Clean);
    }

    #[test]
    fn a_snapshot_keeps_the_slug_it_was_taken_for() {
        let mut session = NoteSession::new("a", "one");
        session.mark_edited();
        let snapshot = session.snapshot("two").unwrap();
        // The note is renamed before the write lands.
        session.retarget("b");
        assert_eq!(snapshot.slug, "a");
        assert_eq!(
            session.finish(&snapshot, Ok::<(), String>(())),
            FlushOutcome::Saved
        );
        assert!(session.is_dirty(), "the write was for the old name");
        assert_eq!(session.snapshot("two").unwrap().slug, "b");
    }

    #[test]
    fn a_snapshot_names_its_kind_and_only_finishes_that_document() {
        let mut template = NoteSession::new("day", "# {{ date }}").of_kind(DocKind::Template);
        template.mark_edited();
        let snapshot = template.snapshot("# {{ date }}!").unwrap();
        assert_eq!(snapshot.kind, DocKind::Template);

        // A note of the same name is another file.
        let mut note = NoteSession::new("day", "x");
        note.mark_edited();
        note.finish(&snapshot, Ok::<(), String>(()));
        assert!(note.is_dirty(), "the write was the template's");
        assert_eq!(
            template.finish(&snapshot, Ok::<(), String>(())),
            FlushOutcome::Saved
        );
        assert!(!template.is_dirty());
    }

    #[test]
    fn debounce_generation_tracks_latest_edit() {
        let mut session = NoteSession::new("a", "");
        let first = session.mark_edited();
        let second = session.mark_edited();
        assert!(!session.is_latest_edit(first));
        assert!(session.is_latest_edit(second));
    }

    #[test]
    fn line_endings_are_detected_from_the_body() {
        assert_eq!(LineEnding::of(""), LineEnding::Lf);
        assert_eq!(LineEnding::of("one line"), LineEnding::Lf);
        assert_eq!(LineEnding::of("a\nb\n"), LineEnding::Lf);
        assert_eq!(LineEnding::of("a\r\nb\r\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::of("\r\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::of("a\r\nb\n"), LineEnding::Lf, "mixed");
        assert_eq!(LineEnding::of("\na"), LineEnding::Lf);
        assert_eq!(LineEnding::of("a\rb"), LineEnding::Lf, "lone CR");
    }

    #[test]
    fn crlf_notes_keep_crlf_on_new_lines() {
        // Enter at the end of `a` in `a\r\nb\r\n`: the editor inserts a bare
        // LF before the `\r`.
        assert_eq!(LineEnding::Crlf.apply("a\n\r\nb\r\n"), "a\r\n\r\nb\r\n");
        assert_eq!(LineEnding::Crlf.apply("x\ny"), "x\r\ny");
        assert_eq!(LineEnding::Crlf.apply("\n"), "\r\n");
        assert!(matches!(LineEnding::Crlf.apply("a\r\n"), Cow::Borrowed(_)));
        assert_eq!(LineEnding::Lf.apply("a\nb"), "a\nb");

        let disk = Disk::default();
        let mut session = NoteSession::new("c", "a\r\nb\r\n");
        session.mark_edited();
        session.flush("a\n\r\nb\r\n", |s| disk.save(s));
        assert_eq!(disk.writes.borrow()[0].1, "a\r\n\r\nb\r\n");
        // Unedited CRLF text compares equal after conversion.
        session.mark_edited();
        assert_eq!(
            session.flush("a\r\n\r\nb\r\n", |s| disk.save(s)),
            FlushOutcome::Unchanged
        );
    }

    #[test]
    fn empty_note_round_trips() {
        let disk = Disk::default();
        let mut session = NoteSession::new("empty", "");
        session.mark_edited();
        session.flush("x", |s| disk.save(s));
        session.mark_edited();
        session.flush("", |s| disk.save(s));
        assert_eq!(
            disk.writes.borrow().last(),
            Some(&("empty".to_string(), String::new()))
        );
    }
}
