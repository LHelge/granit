use crate::app::{components::modal::Modal, ipc, AppCtx};
use granit_types::ReleaseNotes;
use leptos::prelude::*;

/// Shown once on the first launch after an automatic update: the release
/// notes for the version now running. Dismissing acknowledges the notes on
/// the backend so they are not shown again.
#[component]
pub fn ReleaseNotesModal(
    notes: ReleaseNotes,
    set_notes: WriteSignal<Option<ReleaseNotes>>,
) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();

    let close = move || {
        leptos::task::spawn_local(async move {
            if let Err(err) = ipc::acknowledge_release_notes().await {
                ctx.push_error("updater", format!("Failed to dismiss release notes: {err}"));
            }
        });
        set_notes.set(None);
    };

    view! {
        <Modal
            title=format!("What's new in v{}", notes.version)
            subtitle="Granit was updated automatically"
            panel_class="w-[560px] max-w-[90vw]"
            on_close=Callback::new(move |()| close())
        >
            <div class="p-4 overflow-y-auto max-h-[60vh]">
                <div
                    class="prose prose-sm max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
                    inner_html=notes.notes_html.clone()
                />
            </div>
        </Modal>
    }
}
