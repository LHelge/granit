use crate::app::{components::modal::Modal, AppCtx};
use leptos::prelude::*;

/// A group of keybindings shown as one section in the modal.
struct BindGroup {
    title: &'static str,
    binds: Vec<(String, &'static str)>,
}

/// Build the keybinding reference. `combo` renders a platform-appropriate
/// modifier combo ("⌘B" / "Ctrl+B"); plain keys are passed through as-is.
fn bind_groups(is_mac: bool) -> Vec<BindGroup> {
    let combo = |key: &str| {
        if is_mac {
            format!("⌘{key}")
        } else {
            format!("Ctrl+{key}")
        }
    };
    vec![
        BindGroup {
            title: "General",
            binds: vec![
                (combo("E"), "Edit the active note"),
                (combo("S"), "Save and close the editor"),
                (combo("Click"), "Follow a link while editing"),
            ],
        },
        BindGroup {
            title: "Formatting",
            binds: vec![
                (combo("B"), "Bold"),
                (combo("I"), "Italic"),
                (
                    combo("K"),
                    "Wiki-link the selection (URLs become markdown links)",
                ),
                (combo("L"), "Toggle task checkbox"),
            ],
        },
        BindGroup {
            title: "Lists and structure",
            binds: vec![
                ("Enter".to_string(), "Continue list or blockquote"),
                ("Backspace".to_string(), "Remove empty list marker"),
                ("Tab".to_string(), "Indent"),
                ("Shift+Tab".to_string(), "Outdent"),
            ],
        },
        BindGroup {
            title: "Search",
            binds: vec![
                (combo("F"), "Find and replace in note"),
                (combo("G"), "Find next"),
                (
                    if is_mac {
                        "⇧⌘G".to_string()
                    } else {
                        "Shift+Ctrl+G".to_string()
                    },
                    "Find previous",
                ),
                (combo("D"), "Select next occurrence"),
            ],
        },
        BindGroup {
            title: "Completion",
            binds: vec![
                ("[[".to_string(), "Wiki-link completion"),
                ("> [!".to_string(), "Alert completion"),
                ("Esc".to_string(), "Close the completion popup"),
            ],
        },
    ]
}

#[component]
pub fn KeybindsModal(set_open: WriteSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let close = move || set_open.set(false);
    let groups = bind_groups(ctx.is_mac);

    view! {
        <Modal
            title="Keyboard shortcuts"
            subtitle="Bindings available in the editor"
            panel_class="w-[420px] max-w-[90vw] max-h-[80vh]"
            on_close=Callback::new(move |()| close())
        >
            <div class="p-4 space-y-4 overflow-y-auto">
                {groups
                    .into_iter()
                    .map(|group| {
                        view! {
                            <div class="space-y-1">
                                <h3 class="text-xs font-semibold uppercase tracking-wide text-base-content/45 mb-1.5">
                                    {group.title}
                                </h3>
                                <div class="rounded-box border border-base-content/15 overflow-hidden">
                                    {group
                                        .binds
                                        .into_iter()
                                        .map(|(keys, action)| {
                                            view! {
                                                <div class="flex items-center justify-between gap-4 px-4 py-2 border-b border-base-content/10 last:border-b-0">
                                                    <span class="text-sm text-base-content/70">{action}</span>
                                                    <kbd class="kbd kbd-sm">{keys}</kbd>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </Modal>
    }
}
