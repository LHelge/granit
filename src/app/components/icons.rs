use leptos::prelude::*;
pub use leptos_icons::Icon;

/// Provider logo icon. Maps `provider_type` (e.g. `"ollama"`, `"anthropic"`)
/// to the corresponding SVG in `/public/`.
#[component]
pub fn ProviderIcon(
    #[prop(into)] provider_type: Signal<String>,
    #[prop(default = "w-4 h-4 shrink-0")] class: &'static str,
) -> impl IntoView {
    // Returns (path, needs_invert) — monochrome black SVGs need a CSS filter.
    let icon_info = move || match provider_type.get().as_str() {
        "anthropic" => Some(("/public/claude.svg", false)),
        "ollama" => Some(("/public/ollama.svg", true)),
        "mistral" => Some(("/public/mistral.svg", false)),
        "prisma" => Some(("/public/prisma.svg", false)),
        "openai" => Some(("/public/openai.svg", true)),
        _ => None,
    };
    move || {
        icon_info().map(|(src, invert)| {
            let style = if invert {
                "filter: brightness(0) invert(0.85)"
            } else {
                ""
            };
            view! { <img src=src class=class style=style /> }
        })
    }
}
