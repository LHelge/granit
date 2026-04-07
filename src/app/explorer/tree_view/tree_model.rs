use granit_types::NoteMeta;

/// A node in the display tree built from flat `NoteMeta` list.
#[derive(Clone, Debug)]
pub(super) enum TreeNode {
    Note(NoteMeta),
    Folder {
        name: String,
        /// Relative path from cave root, e.g. `"projects/2026"`.
        path: String,
        children: Vec<TreeNode>,
    },
}

/// Build a display tree from a flat list of NoteMeta and folder paths.
/// Each `relative_path` like `"a/b/note.md"` is split on `/` to produce the hierarchy.
/// `folders` ensures empty directories also appear in the tree.
pub(super) fn build_tree(notes: Vec<NoteMeta>, folders: Vec<String>) -> Vec<TreeNode> {
    let mut roots: Vec<TreeNode> = Vec::new();

    // Ensure all folder paths exist in the tree (including empty ones).
    let mut sorted_folders = folders;
    sorted_folders.sort_by_key(|a| a.to_lowercase());
    for folder_path in sorted_folders {
        let parts: Vec<&str> = folder_path.split('/').collect();
        ensure_folder(&mut roots, &parts, 0);
    }

    // Sort so folders and notes appear deterministically.
    let mut sorted = notes;
    sorted.sort_by(|a, b| {
        a.relative_path
            .to_lowercase()
            .cmp(&b.relative_path.to_lowercase())
    });

    for meta in sorted {
        let relative_path = meta.relative_path.clone();
        let parts: Vec<&str> = relative_path.split('/').collect();
        insert_node(&mut roots, &parts, 0, meta);
    }

    roots
}

/// Ensure a folder path exists in the tree, creating empty folder nodes as needed.
fn ensure_folder(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize) {
    if depth >= parts.len() {
        return;
    }
    let folder_name = parts[depth].to_string();
    let folder_path = parts[0..=depth].join("/");
    if let Some(TreeNode::Folder { children, .. }) = nodes
        .iter_mut()
        .find(|n| matches!(n, TreeNode::Folder { name, .. } if *name == folder_name))
    {
        ensure_folder(children, parts, depth + 1);
    } else {
        let mut children = Vec::new();
        ensure_folder(&mut children, parts, depth + 1);
        nodes.push(TreeNode::Folder {
            name: folder_name,
            path: folder_path,
            children,
        });
    }
}

fn insert_node(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize, meta: NoteMeta) {
    if depth == parts.len().saturating_sub(1) {
        // Leaf — a note.
        nodes.push(TreeNode::Note(meta));
        return;
    }
    // Intermediate — a folder.
    let folder_name = parts[depth].to_string();
    let folder_path = parts[0..=depth].join("/");
    if let Some(TreeNode::Folder { children, .. }) = nodes
        .iter_mut()
        .find(|n| matches!(n, TreeNode::Folder { name, .. } if *name == folder_name))
    {
        insert_node(children, parts, depth + 1, meta);
    } else {
        let mut children = Vec::new();
        insert_node(&mut children, parts, depth + 1, meta);
        nodes.push(TreeNode::Folder {
            name: folder_name,
            path: folder_path,
            children,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn note(slug: &str, relative_path: &str) -> NoteMeta {
        NoteMeta {
            slug: slug.into(),
            relative_path: relative_path.into(),
            icon: None,
            favorite: None,
        }
    }

    fn folder<'a>(nodes: &'a [TreeNode], path: &str) -> &'a TreeNode {
        nodes.iter()
            .find(|node| matches!(node, TreeNode::Folder { path: node_path, .. } if node_path == path))
            .unwrap_or_else(|| panic!("missing folder: {path}"))
    }

    fn note_slugs(nodes: &[TreeNode]) -> Vec<&str> {
        nodes.iter()
            .filter_map(|node| match node {
                TreeNode::Note(meta) => Some(meta.slug.as_str()),
                TreeNode::Folder { .. } => None,
            })
            .collect()
    }

    #[wasm_bindgen_test]
    fn build_tree_creates_missing_folder_hierarchy_from_notes() {
        let tree = build_tree(vec![note("deep-note", "projects/2026/deep-note.md")], vec![]);

        let TreeNode::Folder { children, .. } = folder(&tree, "projects") else {
            unreachable!();
        };
        let TreeNode::Folder { children, .. } = folder(children, "projects/2026") else {
            unreachable!();
        };

        assert_eq!(note_slugs(children), vec!["deep-note"]);
    }

    #[wasm_bindgen_test]
    fn build_tree_keeps_empty_folders_from_folder_list() {
        let tree = build_tree(vec![], vec!["empty/subfolder".into()]);

        let TreeNode::Folder { children, .. } = folder(&tree, "empty") else {
            unreachable!();
        };
        let TreeNode::Folder { children, .. } = folder(children, "empty/subfolder") else {
            unreachable!();
        };

        assert!(children.is_empty());
    }

    #[wasm_bindgen_test]
    fn build_tree_sorts_notes_case_insensitively_within_a_folder() {
        let tree = build_tree(
            vec![
                note("Zulu", "projects/Zulu.md"),
                note("alpha", "projects/alpha.md"),
                note("Bravo", "projects/Bravo.md"),
            ],
            vec![],
        );

        let TreeNode::Folder { children, .. } = folder(&tree, "projects") else {
            unreachable!();
        };

        assert_eq!(note_slugs(children), vec!["alpha", "Bravo", "Zulu"]);
    }
}
