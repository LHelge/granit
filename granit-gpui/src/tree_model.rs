//! Folder/note tree built from the cave's flat note and folder lists.
//!
//! The shape and ordering match the web explorer
//! (`src/app/explorer/tree_view/tree_model.rs`): within each level, folders
//! come first in case-insensitive path order, followed by notes in
//! case-insensitive relative-path order.

use gpui_kit::component::tree::TreeItem;
use granit_lib::cave::DocumentMeta;

const FOLDER_ID_PREFIX: &str = "folder:";
const NOTE_ID_PREFIX: &str = "note:";

/// A node in the display tree.
#[derive(Clone, Debug)]
pub enum TreeNode {
    Note(DocumentMeta),
    Folder {
        name: String,
        /// Relative path from the cave root, e.g. `"projects/2026"`.
        path: String,
        children: Vec<TreeNode>,
    },
}

/// Build a display tree from the flat note list and folder paths.
///
/// Each `relative_path` like `"a/b/note.md"` is split on `/` to produce the
/// hierarchy. `folders` makes empty directories appear in the tree too.
pub fn build_tree(notes: Vec<DocumentMeta>, folders: Vec<String>) -> Vec<TreeNode> {
    let mut roots: Vec<TreeNode> = Vec::new();

    let mut sorted_folders = folders;
    sorted_folders.sort_by_key(|a| a.to_lowercase());
    for folder_path in sorted_folders {
        let parts: Vec<&str> = folder_path.split('/').collect();
        ensure_folder(&mut roots, &parts, 0);
    }

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
    let children = folder_children(nodes, parts, depth);
    ensure_folder(children, parts, depth + 1);
}

fn insert_node(nodes: &mut Vec<TreeNode>, parts: &[&str], depth: usize, meta: DocumentMeta) {
    if depth == parts.len().saturating_sub(1) {
        nodes.push(TreeNode::Note(meta));
        return;
    }
    let children = folder_children(nodes, parts, depth);
    insert_node(children, parts, depth + 1, meta);
}

/// Return the children of the folder `parts[depth]` at this level, appending
/// a new empty folder node when it does not exist yet.
fn folder_children<'a>(
    nodes: &'a mut Vec<TreeNode>,
    parts: &[&str],
    depth: usize,
) -> &'a mut Vec<TreeNode> {
    let folder_name = parts[depth];
    let ix = match nodes
        .iter()
        .position(|n| matches!(n, TreeNode::Folder { name, .. } if name == folder_name))
    {
        Some(ix) => ix,
        None => {
            nodes.push(TreeNode::Folder {
                name: folder_name.to_string(),
                path: parts[0..=depth].join("/"),
                children: Vec::new(),
            });
            nodes.len() - 1
        }
    };
    match &mut nodes[ix] {
        TreeNode::Folder { children, .. } => children,
        TreeNode::Note(_) => unreachable!("position matched a folder"),
    }
}

/// What a tree item id refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind<'a> {
    Folder(&'a str),
    Note(&'a str),
}

/// Decode a tree item id produced by [`to_tree_items`].
///
/// Folder and note ids carry a prefix because the Kit tree treats only items
/// with children as folders, so an empty folder is otherwise
/// indistinguishable from a note.
pub fn item_kind(id: &str) -> Option<ItemKind<'_>> {
    if let Some(path) = id.strip_prefix(FOLDER_ID_PREFIX) {
        Some(ItemKind::Folder(path))
    } else {
        id.strip_prefix(NOTE_ID_PREFIX).map(ItemKind::Note)
    }
}

/// Convert the display tree into Kit tree items, all folders collapsed.
pub fn to_tree_items(nodes: &[TreeNode]) -> Vec<TreeItem> {
    nodes.iter().map(to_tree_item).collect()
}

fn to_tree_item(node: &TreeNode) -> TreeItem {
    match node {
        TreeNode::Note(meta) => {
            TreeItem::new(format!("{NOTE_ID_PREFIX}{}", meta.slug), meta.slug.clone())
        }
        TreeNode::Folder {
            name,
            path,
            children,
        } => TreeItem::new(format!("{FOLDER_ID_PREFIX}{path}"), name.clone())
            .children(children.iter().map(to_tree_item)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(slug: &str, relative_path: &str) -> DocumentMeta {
        DocumentMeta {
            slug: slug.into(),
            relative_path: relative_path.into(),
            icon: None,
            favorite: None,
            description: None,
        }
    }

    fn folder<'a>(nodes: &'a [TreeNode], path: &str) -> &'a [TreeNode] {
        nodes
            .iter()
            .find_map(|node| match node {
                TreeNode::Folder {
                    path: node_path,
                    children,
                    ..
                } if node_path == path => Some(children.as_slice()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing folder: {path}"))
    }

    fn note_slugs(nodes: &[TreeNode]) -> Vec<&str> {
        nodes
            .iter()
            .filter_map(|node| match node {
                TreeNode::Note(meta) => Some(meta.slug.as_str()),
                TreeNode::Folder { .. } => None,
            })
            .collect()
    }

    fn labels(nodes: &[TreeNode]) -> Vec<&str> {
        nodes
            .iter()
            .map(|node| match node {
                TreeNode::Note(meta) => meta.slug.as_str(),
                TreeNode::Folder { name, .. } => name.as_str(),
            })
            .collect()
    }

    #[test]
    fn build_tree_creates_missing_folder_hierarchy_from_notes() {
        let tree = build_tree(
            vec![note("deep-note", "projects/2026/deep-note.md")],
            vec![],
        );
        let children = folder(folder(&tree, "projects"), "projects/2026");
        assert_eq!(note_slugs(children), vec!["deep-note"]);
    }

    #[test]
    fn build_tree_keeps_empty_folders_from_folder_list() {
        let tree = build_tree(vec![], vec!["empty/subfolder".into()]);
        let children = folder(folder(&tree, "empty"), "empty/subfolder");
        assert!(children.is_empty());
    }

    #[test]
    fn build_tree_places_folders_before_notes() {
        let tree = build_tree(
            vec![note("alpha", "alpha.md"), note("child", "zeta/child.md")],
            vec!["zeta".into()],
        );
        assert_eq!(labels(&tree), vec!["zeta", "alpha"]);
    }

    #[test]
    fn build_tree_sorts_folders_case_insensitively() {
        let tree = build_tree(vec![], vec!["beta".into(), "Alpha".into(), "gamma".into()]);
        assert_eq!(labels(&tree), vec!["Alpha", "beta", "gamma"]);
    }

    #[test]
    fn build_tree_sorts_notes_case_insensitively_within_a_folder() {
        let tree = build_tree(
            vec![
                note("Zulu", "projects/Zulu.md"),
                note("alpha", "projects/alpha.md"),
                note("Bravo", "projects/Bravo.md"),
            ],
            vec![],
        );
        assert_eq!(
            note_slugs(folder(&tree, "projects")),
            vec!["alpha", "Bravo", "Zulu"]
        );
    }

    #[test]
    fn build_tree_of_empty_cave_is_empty() {
        assert!(build_tree(vec![], vec![]).is_empty());
    }

    #[test]
    fn tree_items_carry_kind_in_their_ids() {
        let tree = build_tree(
            vec![note("child", "projects/child.md")],
            vec!["projects".into(), "empty".into()],
        );
        let items = to_tree_items(&tree);

        let ids: Vec<&str> = items.iter().map(|item| item.id.as_ref()).collect();
        assert_eq!(ids, vec!["folder:empty", "folder:projects"]);
        assert_eq!(
            item_kind(&items[0].id),
            Some(ItemKind::Folder("empty")),
            "an empty folder stays a folder"
        );
        assert!(!items[0].is_expanded());

        let child = &items[1].children[0];
        assert_eq!(child.label.as_ref(), "child");
        assert_eq!(item_kind(&child.id), Some(ItemKind::Note("child")));
    }

    #[test]
    fn item_kind_rejects_unknown_ids() {
        assert_eq!(item_kind("other"), None);
    }
}
