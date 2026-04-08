use super::SettingsForm;
use crate::app::{components::icons::Icon, AppCtx};
use leptos::prelude::*;

/// A DaisyUI theme entry: (data-theme value, display label, is_dark).
struct ThemeEntry {
    id: &'static str,
    label: &'static str,
    is_dark: bool,
}

const DAISY_THEMES: &[ThemeEntry] = &[
    ThemeEntry {
        id: "light",
        label: "Light",
        is_dark: false,
    },
    ThemeEntry {
        id: "dark",
        label: "Dark",
        is_dark: true,
    },
    ThemeEntry {
        id: "cupcake",
        label: "Cupcake",
        is_dark: false,
    },
    ThemeEntry {
        id: "bumblebee",
        label: "Bumblebee",
        is_dark: false,
    },
    ThemeEntry {
        id: "emerald",
        label: "Emerald",
        is_dark: false,
    },
    ThemeEntry {
        id: "corporate",
        label: "Corporate",
        is_dark: false,
    },
    ThemeEntry {
        id: "synthwave",
        label: "Synthwave",
        is_dark: true,
    },
    ThemeEntry {
        id: "retro",
        label: "Retro",
        is_dark: false,
    },
    ThemeEntry {
        id: "cyberpunk",
        label: "Cyberpunk",
        is_dark: false,
    },
    ThemeEntry {
        id: "valentine",
        label: "Valentine",
        is_dark: false,
    },
    ThemeEntry {
        id: "halloween",
        label: "Halloween",
        is_dark: true,
    },
    ThemeEntry {
        id: "garden",
        label: "Garden",
        is_dark: false,
    },
    ThemeEntry {
        id: "forest",
        label: "Forest",
        is_dark: true,
    },
    ThemeEntry {
        id: "aqua",
        label: "Aqua",
        is_dark: false,
    },
    ThemeEntry {
        id: "lofi",
        label: "Lo-fi",
        is_dark: false,
    },
    ThemeEntry {
        id: "pastel",
        label: "Pastel",
        is_dark: false,
    },
    ThemeEntry {
        id: "fantasy",
        label: "Fantasy",
        is_dark: false,
    },
    ThemeEntry {
        id: "wireframe",
        label: "Wireframe",
        is_dark: false,
    },
    ThemeEntry {
        id: "black",
        label: "Black",
        is_dark: true,
    },
    ThemeEntry {
        id: "luxury",
        label: "Luxury",
        is_dark: true,
    },
    ThemeEntry {
        id: "dracula",
        label: "Dracula",
        is_dark: true,
    },
    ThemeEntry {
        id: "cmyk",
        label: "CMYK",
        is_dark: false,
    },
    ThemeEntry {
        id: "autumn",
        label: "Autumn",
        is_dark: false,
    },
    ThemeEntry {
        id: "business",
        label: "Business",
        is_dark: true,
    },
    ThemeEntry {
        id: "acid",
        label: "Acid",
        is_dark: false,
    },
    ThemeEntry {
        id: "lemonade",
        label: "Lemonade",
        is_dark: false,
    },
    ThemeEntry {
        id: "night",
        label: "Night",
        is_dark: true,
    },
    ThemeEntry {
        id: "coffee",
        label: "Coffee",
        is_dark: true,
    },
    ThemeEntry {
        id: "winter",
        label: "Winter",
        is_dark: false,
    },
    ThemeEntry {
        id: "dim",
        label: "Dim",
        is_dark: true,
    },
    ThemeEntry {
        id: "nord",
        label: "Nord",
        is_dark: false,
    },
    ThemeEntry {
        id: "sunset",
        label: "Sunset",
        is_dark: true,
    },
    ThemeEntry {
        id: "caramellatte",
        label: "Caramel Latte",
        is_dark: false,
    },
    ThemeEntry {
        id: "abyss",
        label: "Abyss",
        is_dark: true,
    },
    ThemeEntry {
        id: "silk",
        label: "Silk",
        is_dark: false,
    },
];

const CATPPUCCIN_THEMES: &[ThemeEntry] = &[
    ThemeEntry {
        id: "catppuccin-latte",
        label: "Latte",
        is_dark: false,
    },
    ThemeEntry {
        id: "catppuccin-frappe",
        label: "Frappé",
        is_dark: true,
    },
    ThemeEntry {
        id: "catppuccin-macchiato",
        label: "Macchiato",
        is_dark: true,
    },
    ThemeEntry {
        id: "catppuccin-mocha",
        label: "Mocha",
        is_dark: true,
    },
];

const COMMUNITY_THEMES: &[ThemeEntry] = &[
    ThemeEntry {
        id: "gruvbox-light",
        label: "Gruvbox Light",
        is_dark: false,
    },
    ThemeEntry {
        id: "gruvbox-dark",
        label: "Gruvbox Dark",
        is_dark: true,
    },
    ThemeEntry {
        id: "tokyo-night",
        label: "Tokyo Night",
        is_dark: true,
    },
    ThemeEntry {
        id: "rose-pine-dawn",
        label: "Rosé Pine Dawn",
        is_dark: false,
    },
    ThemeEntry {
        id: "rose-pine-moon",
        label: "Rosé Pine Moon",
        is_dark: true,
    },
    ThemeEntry {
        id: "one-dark",
        label: "One Dark",
        is_dark: true,
    },
];

#[component]
pub fn ThemeSettings(form: RwSignal<SettingsForm>) -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let active_id = move || form.get().theme;

    let show_light = RwSignal::new(true);
    let show_dark = RwSignal::new(true);

    let apply = move |id: &'static str| {
        // Visual preview only — persisted when the user clicks Save.
        ctx.set_theme(id);
        form.update(|f| f.theme = id.to_string());
    };

    view! {
        <div class="space-y-4">

            // ── Light / dark filter ───────────────────────────────
            <div class="flex gap-1.5">
                <button
                    type="button"
                    class=move || if show_light.get() {
                        "btn btn-sm gap-1.5"
                    } else {
                        "btn btn-sm btn-ghost gap-1.5 opacity-40"
                    }
                    on:click=move |_| {
                        if !show_light.get_untracked() || show_dark.get_untracked() {
                            show_light.update(|v| *v = !*v);
                        }
                    }
                >
                    <span class="inline-flex w-3.5 h-3.5">
                        <Icon icon=icondata_lu::LuSun width="100%" height="100%"/>
                    </span>
                    "Light"
                </button>
                <button
                    type="button"
                    class=move || if show_dark.get() {
                        "btn btn-sm gap-1.5"
                    } else {
                        "btn btn-sm btn-ghost gap-1.5 opacity-40"
                    }
                    on:click=move |_| {
                        if !show_dark.get_untracked() || show_light.get_untracked() {
                            show_dark.update(|v| *v = !*v);
                        }
                    }
                >
                    <span class="inline-flex w-3.5 h-3.5">
                        <Icon icon=icondata_lu::LuMoon width="100%" height="100%"/>
                    </span>
                    "Dark"
                </button>
            </div>

            // ── Community themes ──────────────────────────────────
            <Show when=move || COMMUNITY_THEMES.iter().any(|t| (t.is_dark && show_dark.get()) || (!t.is_dark && show_light.get()))>
                <fieldset>
                    <legend class="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-2">
                        "Community"
                    </legend>
                    <div class="grid grid-cols-2 gap-1">
                        {move || COMMUNITY_THEMES.iter()
                            .filter(|t| (t.is_dark && show_dark.get()) || (!t.is_dark && show_light.get()))
                            .map(|t| {
                                let id = t.id;
                                let label = t.label;
                                let is_dark = t.is_dark;
                                view! {
                                    <ThemeSwatch
                                        id=id
                                        label=label
                                        is_dark=is_dark
                                        active_id=active_id
                                        on_select=move || apply(id)
                                    />
                                }
                            }).collect_view()}
                    </div>
                </fieldset>
            </Show>

            // ── Catppuccin themes ─────────────────────────────────
            <Show when=move || CATPPUCCIN_THEMES.iter().any(|t| (t.is_dark && show_dark.get()) || (!t.is_dark && show_light.get()))>
                <fieldset>
                    <legend class="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-2">
                        "Catppuccin"
                    </legend>
                    <div class="grid grid-cols-2 gap-1">
                        {move || CATPPUCCIN_THEMES.iter()
                            .filter(|t| (t.is_dark && show_dark.get()) || (!t.is_dark && show_light.get()))
                            .map(|t| {
                                let id = t.id;
                                let label = t.label;
                                let is_dark = t.is_dark;
                                view! {
                                    <ThemeSwatch
                                        id=id
                                        label=label
                                        is_dark=is_dark
                                        active_id=active_id
                                        on_select=move || apply(id)
                                    />
                                }
                            }).collect_view()}
                    </div>
                </fieldset>
            </Show>

            // ── DaisyUI built-in themes ───────────────────────────
            <Show when=move || DAISY_THEMES.iter().any(|t| (t.is_dark && show_dark.get()) || (!t.is_dark && show_light.get()))>
                <fieldset>
                    <legend class="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-2">
                        "DaisyUI"
                    </legend>
                    <div class="grid grid-cols-2 gap-1">
                        {move || DAISY_THEMES.iter()
                            .filter(|t| (t.is_dark && show_dark.get()) || (!t.is_dark && show_light.get()))
                            .map(|t| {
                                let id = t.id;
                                let label = t.label;
                                let is_dark = t.is_dark;
                                view! {
                                    <ThemeSwatch
                                        id=id
                                        label=label
                                        is_dark=is_dark
                                        active_id=active_id
                                        on_select=move || apply(id)
                                    />
                                }
                            }).collect_view()}
                    </div>
                </fieldset>
            </Show>

        </div>
    }
}

#[component]
fn ThemeSwatch(
    id: &'static str,
    label: &'static str,
    is_dark: bool,
    active_id: impl Fn() -> String + Copy + Send + Sync + 'static,
    on_select: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let is_active = move || active_id() == id;

    view! {
        <button
            type="button"
            class=move || {
                if is_active() {
                    "flex items-center justify-between px-2.5 py-1.5 rounded text-sm \
                     bg-base-content/15 border border-primary text-base-content transition-colors"
                } else {
                    "flex items-center justify-between px-2.5 py-1.5 rounded text-sm \
                     border border-transparent hover:bg-base-content/10 \
                     text-base-content/70 hover:text-base-content transition-colors"
                }
            }
            on:click=move |_| on_select()
        >
            // Color preview — scoped to this theme via data-theme
            <div data-theme=id class="flex shrink-0 rounded overflow-hidden border border-black/10">
                <div class="w-3 h-4 bg-base-200"></div>
                <div class="w-3 h-4 bg-primary"></div>
                <div class="w-3 h-4 bg-secondary"></div>
                <div class="w-3 h-4 bg-accent"></div>
            </div>
            <span class="flex-1 truncate ml-2">{label}</span>
            <span class=move || {
                if is_active() {
                    "ml-1.5 shrink-0 inline-flex items-center p-0.5 rounded-sm bg-primary/20 text-primary"
                } else if is_dark {
                    "ml-1.5 shrink-0 inline-flex items-center p-0.5 rounded-sm bg-stone-800 text-stone-200"
                } else {
                    "ml-1.5 shrink-0 inline-flex items-center p-0.5 rounded-sm bg-stone-100 text-stone-600"
                }
            }>
                <span class="inline-flex w-3 h-3">
                    <Icon icon=if is_dark { icondata_lu::LuMoon } else { icondata_lu::LuSun } width="100%" height="100%"/>
                </span>
            </span>
        </button>
    }
}
