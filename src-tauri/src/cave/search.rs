use super::{Cave, CaveError};
use granit_types::ContentMatch;

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
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (slug, abs_path) in &self.notes {
            if let Some(limit) = max_results {
                if results.len() >= limit {
                    break;
                }
            }
            let raw = std::fs::read_to_string(abs_path)?;
            let body = crate::markdown::strip_frontmatter(&raw);
            let snippets: Vec<String> = body
                .lines()
                .filter(|line| line.to_lowercase().contains(&query_lower))
                .map(|line| line.to_string())
                .collect();
            if !snippets.is_empty() {
                results.push(ContentMatch {
                    slug: slug.clone(),
                    snippets,
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
