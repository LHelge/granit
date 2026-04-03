use icondata_core::IconData;
use icondata_lu;

/// An entry in the curated note icon catalog.
#[allow(dead_code)]
pub struct NoteIconEntry {
    /// Normalized icon id stored in frontmatter (PascalCase, e.g. `"Star"`).
    pub id: &'static str,
    /// Human-readable label shown in the picker.
    pub label: &'static str,
    /// Extra search terms (space-separated, lowercase) for fuzzy matching.
    pub tags: &'static str,
    /// The icon data used to render via `leptos_icons`.
    pub icon: &'static IconData,
}

/// Curated subset of Lucide icons suitable for note labelling.
///
/// Kept intentionally small (~25 entries) so the picker grid stays readable.
/// Add entries here when the picker gains a new useful category; do not expose
/// the full `icondata_lu` set — there are thousands. Icons from other
/// `icondata_*` crates (e.g. `icondata_re` for Remix Icons) can be added
/// here too — just extend the array and pick a unique id.
#[rustfmt::skip]
pub static NOTE_ICONS: &[NoteIconEntry] = &[
    NoteIconEntry { id: "FileText",   label: "Note",       tags: "default file document",           icon: icondata_lu::LuFileText   },
    NoteIconEntry { id: "Book",       label: "Book",       tags: "read reference manual guide",     icon: icondata_lu::LuBook       },
    NoteIconEntry { id: "BookOpen",   label: "Open book",  tags: "reading study",                   icon: icondata_lu::LuBookOpen   },
    NoteIconEntry { id: "Bookmark",   label: "Bookmark",   tags: "saved reference important",       icon: icondata_lu::LuBookmark   },
    NoteIconEntry { id: "Pencil",     label: "Pencil",     tags: "write draft edit",                icon: icondata_lu::LuPencil     },
    NoteIconEntry { id: "ListChecks", label: "Checklist",  tags: "todo task list action",           icon: icondata_lu::LuListChecks },
    NoteIconEntry { id: "Calendar",   label: "Calendar",   tags: "date meeting schedule event log", icon: icondata_lu::LuCalendar   },
    NoteIconEntry { id: "Star",       label: "Star",       tags: "favourite important highlight",   icon: icondata_lu::LuStar       },
    NoteIconEntry { id: "Heart",      label: "Heart",      tags: "personal love favourite",         icon: icondata_lu::LuHeart      },
    NoteIconEntry { id: "Flame",      label: "Flame",      tags: "hot urgent priority fire",        icon: icondata_lu::LuFlame      },
    NoteIconEntry { id: "Zap",        label: "Zap",        tags: "quick idea flash energy",         icon: icondata_lu::LuZap        },
    NoteIconEntry { id: "Lightbulb",  label: "Lightbulb",  tags: "idea brainstorm concept",         icon: icondata_lu::LuLightbulb  },
    NoteIconEntry { id: "Brain",      label: "Brain",      tags: "knowledge ideas memory thought",  icon: icondata_lu::LuBrain      },
    NoteIconEntry { id: "Code",       label: "Code",       tags: "programming dev technical",       icon: icondata_lu::LuCode       },
    NoteIconEntry { id: "Bug",        label: "Bug",        tags: "issue error debugging",           icon: icondata_lu::LuBug        },
    NoteIconEntry { id: "Rocket",     label: "Rocket",     tags: "project launch startup",          icon: icondata_lu::LuRocket     },
    NoteIconEntry { id: "Target",     label: "Target",     tags: "goal objective milestone",        icon: icondata_lu::LuTarget     },
    NoteIconEntry { id: "Flag",       label: "Flag",       tags: "milestone marker important",      icon: icondata_lu::LuFlag       },
    NoteIconEntry { id: "Trophy",     label: "Trophy",     tags: "achievement win success award",   icon: icondata_lu::LuTrophy     },
    NoteIconEntry { id: "Lock",       label: "Lock",       tags: "private secret secure",           icon: icondata_lu::LuLock       },
    NoteIconEntry { id: "Globe",      label: "Globe",      tags: "public web world internet",       icon: icondata_lu::LuGlobe      },
    NoteIconEntry { id: "MapPin",     label: "Map pin",    tags: "place location travel",           icon: icondata_lu::LuMapPin     },
    NoteIconEntry { id: "Camera",     label: "Camera",     tags: "photo image picture visual",      icon: icondata_lu::LuCamera     },
    NoteIconEntry { id: "Music",      label: "Music",      tags: "audio song playlist",             icon: icondata_lu::LuMusic      },
    NoteIconEntry { id: "Coffee",     label: "Coffee",     tags: "break journal casual daily",      icon: icondata_lu::LuCoffee     },
];

/// Map a frontmatter icon id (e.g. `"Star"`) to its `IconData`.
///
/// Looks up the id in [`NOTE_ICONS`]. Falls back to `LuFileText` for unknown
/// or empty ids so unknown icons never break rendering.
pub fn resolve_note_icon(id: &str) -> &'static IconData {
    NOTE_ICONS
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.icon)
        .unwrap_or(icondata_lu::LuFileText)
}
