//! Save bookkeeping for the note open in the editor, kept free of GPUI so it
//! can be tested without a window.
//!
//! A [`NoteSession`] remembers the body last read from or written to disk and
//! whether the user has edited since. [`NoteSession::flush`] writes only when
//! the note was edited *and* the editor text differs from that body, so
//! loading a note, or typing a character and deleting it again, never touches
//! the file.

use std::fmt::Display;
use std::time::Duration;

/// Quiet period after the last edit before the debounced save runs.
pub const AUTOSAVE_DELAY: Duration = Duration::from_millis(1000);

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

/// Result of a [`NoteSession::flush`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlushOutcome {
    /// Nothing to write: not edited, or the text equals the saved body.
    Unchanged,
    /// The text was written.
    Saved,
    /// The write failed; the session stays dirty.
    Failed(String),
}

/// Save state of one open note.
#[derive(Debug)]
pub struct NoteSession {
    slug: String,
    /// The body as last read from or written to disk.
    saved: String,
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
        Self {
            slug: slug.into(),
            saved: body.into(),
            dirty: false,
            edit_generation: 0,
            status: SaveStatus::Clean,
        }
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }

    pub fn status(&self) -> &SaveStatus {
        &self.status
    }

    #[cfg(test)]
    pub fn is_dirty(&self) -> bool {
        self.dirty
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

    /// Write `text` through `save(slug, text)` if the note was edited and the
    /// text differs from the saved body. A failed write keeps the session
    /// dirty so the next flush retries.
    pub fn flush<E: Display>(
        &mut self,
        text: &str,
        save: impl FnOnce(&str, &str) -> Result<(), E>,
    ) -> FlushOutcome {
        if !self.dirty {
            return FlushOutcome::Unchanged;
        }
        if text == self.saved {
            self.dirty = false;
            self.status = SaveStatus::Clean;
            return FlushOutcome::Unchanged;
        }
        match save(&self.slug, text) {
            Ok(()) => {
                self.saved = text.to_string();
                self.dirty = false;
                self.status = SaveStatus::Clean;
                FlushOutcome::Saved
            }
            Err(err) => {
                let message = err.to_string();
                self.status = SaveStatus::Failed(message.clone());
                FlushOutcome::Failed(message)
            }
        }
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
        fn save(&self, slug: &str, text: &str) -> Result<(), String> {
            if self.fail {
                return Err("disk full".into());
            }
            self.writes
                .borrow_mut()
                .push((slug.to_string(), text.to_string()));
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
            session.flush("body", |s, t| disk.save(s, t)),
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
        session.flush("$$\nx\n$$", |s, t| disk.save(s, t));
        assert_eq!(disk.write_count(), 0);
    }

    #[test]
    fn edit_then_flush_writes_once() {
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "body");
        session.mark_edited();
        assert_eq!(session.status(), &SaveStatus::Edited);
        assert_eq!(
            session.flush("body!", |s, t| disk.save(s, t)),
            FlushOutcome::Saved
        );
        assert_eq!(
            disk.writes.borrow().as_slice(),
            &[("a".to_string(), "body!".to_string())]
        );
        assert!(!session.is_dirty());
        assert_eq!(session.status(), &SaveStatus::Clean);

        // A second flush without a new edit writes nothing.
        session.flush("body!", |s, t| disk.save(s, t));
        assert_eq!(disk.write_count(), 1);
    }

    #[test]
    fn edit_reverted_before_flush_does_not_write() {
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "body");
        session.mark_edited();
        session.mark_edited();
        assert_eq!(
            session.flush("body", |s, t| disk.save(s, t)),
            FlushOutcome::Unchanged
        );
        assert_eq!(disk.write_count(), 0);
        assert!(!session.is_dirty());
    }

    #[test]
    fn reverting_to_the_last_saved_text_is_compared_against_that_save() {
        let disk = Disk::default();
        let mut session = NoteSession::new("a", "one");
        session.mark_edited();
        session.flush("two", |s, t| disk.save(s, t));
        session.mark_edited();
        session.flush("two", |s, t| disk.save(s, t));
        assert_eq!(disk.write_count(), 1);
        session.mark_edited();
        session.flush("one", |s, t| disk.save(s, t));
        assert_eq!(disk.write_count(), 2);
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
            session.flush("edited", |s, t| disk.save(s, t)),
            FlushOutcome::Failed("disk full".into())
        );
        assert!(session.is_dirty());
        assert_eq!(session.status(), &SaveStatus::Failed("disk full".into()));

        // A further edit keeps the failure visible until a save succeeds.
        session.mark_edited();
        assert_eq!(session.status(), &SaveStatus::Failed("disk full".into()));

        disk.fail = false;
        assert_eq!(
            session.flush("edited", |s, t| disk.save(s, t)),
            FlushOutcome::Saved
        );
        assert_eq!(session.status(), &SaveStatus::Clean);
    }

    #[test]
    fn debounce_generation_tracks_latest_edit() {
        let mut session = NoteSession::new("a", "");
        let first = session.mark_edited();
        let second = session.mark_edited();
        assert!(!session.is_latest_edit(first));
        assert!(session.is_latest_edit(second));
    }

    /// Write a one-note cave with frontmatter into a temp dir.
    fn fixture_cave() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".granit")).unwrap();
        let note = dir.path().join("note.md");
        std::fs::write(
            &note,
            "---\ntags:\n  - tag\ncreated_at: \"2026-01-01T00:00:00Z\"\n\
             modified_at: \"2026-01-01T00:00:00Z\"\nicon: Beer\nfavorite: true\n---\n\
             # Heading\n\n- [ ] todo\n",
        )
        .unwrap();
        (dir, note)
    }

    #[test]
    fn cave_flush_without_edit_leaves_file_byte_identical() {
        let (dir, note) = fixture_cave();
        let before = std::fs::read(&note).unwrap();
        let mut cave = granit_lib::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let body = cave.read_note("note").unwrap().content;
        assert_eq!(body, "# Heading\n\n- [ ] todo\n");

        let mut session = NoteSession::new("note", body.clone());
        session.flush(&body, |slug, text| cave.save_note(slug, text).map(|_| ()));
        // Typing a character and deleting it again.
        session.mark_edited();
        session.mark_edited();
        session.flush(&body, |slug, text| cave.save_note(slug, text).map(|_| ()));
        assert_eq!(std::fs::read(&note).unwrap(), before);
    }

    #[test]
    fn cave_save_replaces_body_and_keeps_frontmatter() {
        let (dir, note) = fixture_cave();
        let mut cave = granit_lib::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let body = cave.read_note("note").unwrap().content;

        let mut session = NoteSession::new("note", body.clone());
        session.mark_edited();
        let edited = body.replace("- [ ]", "- [x]");
        assert_eq!(
            session.flush(&edited, |slug, text| cave.save_note(slug, text).map(|_| ())),
            FlushOutcome::Saved
        );

        let raw = std::fs::read_to_string(&note).unwrap();
        let (frontmatter, saved_body) = raw
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("---\n"))
            .expect("frontmatter kept");
        assert_eq!(saved_body, edited);
        // Every frontmatter line is unchanged except the modification
        // time, which the cave layer stamps on save.
        let without_modified = |fm: &str| -> Vec<String> {
            fm.lines()
                .filter(|line| !line.starts_with("modified_at:"))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(
            without_modified(frontmatter),
            without_modified(
                "tags:\n  - tag\ncreated_at: \"2026-01-01T00:00:00Z\"\nicon: Beer\nfavorite: true\n"
            )
        );
        assert!(!frontmatter.contains("modified_at: \"2026-01-01"));
        assert_eq!(cave.read_note("note").unwrap().content, edited);
    }

    #[test]
    fn cave_save_of_missing_note_reports_failure() {
        let (dir, note) = fixture_cave();
        let mut cave = granit_lib::cave::Cave::open(dir.path().to_path_buf()).unwrap();
        let mut session = NoteSession::new("note", "x");
        std::fs::remove_file(&note).unwrap();
        session.mark_edited();
        let outcome = session.flush("y", |slug, text| cave.save_note(slug, text).map(|_| ()));
        assert!(matches!(outcome, FlushOutcome::Failed(_)), "{outcome:?}");
        assert!(session.is_dirty());
    }

    #[test]
    fn empty_note_round_trips() {
        let disk = Disk::default();
        let mut session = NoteSession::new("empty", "");
        session.mark_edited();
        session.flush("x", |s, t| disk.save(s, t));
        session.mark_edited();
        session.flush("", |s, t| disk.save(s, t));
        assert_eq!(
            disk.writes.borrow().last(),
            Some(&("empty".to_string(), String::new()))
        );
    }
}
