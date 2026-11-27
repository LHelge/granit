use super::{Cave, CaveError};
use granit_types::ContentMatch;

/// The matching lines of one note, from [`Cave::search_lines`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteMatches {
    pub slug: String,
    pub lines: Vec<LineMatch>,
}

/// A body line that contains the query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineMatch {
    /// The 0-based line number in the note's body (the text the editor
    /// shows, without frontmatter).
    pub line: usize,
    /// The line's text.
    pub text: String,
}

impl Cave {
    /// Search note bodies for a query string (case-insensitive substring match).
    ///
    /// Returns up to `max_results` matches, each with the note's slug and
    /// context snippets (all matching lines).
    pub fn search_content(
        &self,
        query: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<ContentMatch>, CaveError> {
        Ok(self
            .search_lines(query, max_results)?
            .into_iter()
            .map(|note| ContentMatch {
                slug: note.slug,
                snippets: note.lines.into_iter().map(|line| line.text).collect(),
            })
            .collect())
    }

    /// Like [`Cave::search_content`], with the body line number of every
    /// match, so a match can be opened where it is. Sorted by slug
    /// (case-insensitive), lines in order.
    pub fn search_lines(
        &self,
        query: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<NoteMatches>, CaveError> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (slug, abs_path) in &self.notes {
            if let Some(limit) = max_results {
                if results.len() >= limit {
                    break;
                }
            }
            let raw = std::fs::read_to_string(abs_path)?;
            let body = crate::markdown::Markdown::new(&raw).body();
            let lines: Vec<LineMatch> = body
                .lines()
                .enumerate()
                .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
                .map(|(line, text)| LineMatch {
                    line,
                    text: text.to_string(),
                })
                .collect();
            if !lines.is_empty() {
                results.push(NoteMatches {
                    slug: slug.clone(),
                    lines,
                });
            }
        }

        results.sort_by_key(|m| m.slug.to_lowercase());
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use crate::cave::Cave;

    #[test]
    fn test_search_content_finds_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Hello world\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("world", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slug, "note");
    }

    #[test]
    fn test_search_content_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Hello WORLD\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("world", None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_content_no_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "Hello world\n").unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("zzzzz", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_lines_counts_body_lines_without_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("note.md"),
            "---\ntags: [x]\n---\n# Title\n\nsome Needle\nhay\nneedle again\n",
        )
        .unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_lines("needle", None).unwrap();
        assert_eq!(results.len(), 1);
        let lines: Vec<(usize, &str)> = results[0]
            .lines
            .iter()
            .map(|m| (m.line, m.text.as_str()))
            .collect();
        assert_eq!(lines, [(2, "some Needle"), (4, "needle again")]);
    }

    #[test]
    fn test_search_content_respects_max_results() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("note-{i}.md")), "findme\n").unwrap();
        }
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();

        let results = cave.search_content("findme", Some(3)).unwrap();
        assert_eq!(results.len(), 3);
    }
}
