use crate::app::{components::icons::Icon, ipc, AppCtx};
use leptos::{prelude::*, task::spawn_local};
use wasm_bindgen::{prelude::*, JsCast};

/// Format a JS Date as a month-only string using the browser's locale.
fn month_name_from_js_date(date: &js_sys::Date) -> Option<String> {
    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"month".into(), &"long".into()).ok()?;
    date.to_locale_string("default", &opts).as_string()
}

#[component]
pub fn Calendar() -> impl IntoView {
    let ctx = expect_context::<AppCtx>();
    let calendar_ref = NodeRef::<leptos::html::Div>::new();

    // Current heading: month name only (no year).
    let (heading, set_heading) = signal(String::new());

    // Derive set of daily-note date strings (YYYY-MM-DD) from the notes signal.
    let daily_dates = Memo::new(move |_| {
        let config = ctx.config.get();
        let folder_prefix = format!("{}/", config.daily_note_folder);
        ctx.notes
            .get()
            .iter()
            .filter(|n| n.relative_path.starts_with(&folder_prefix))
            .map(|n| n.slug.clone())
            .collect::<std::collections::HashSet<String>>()
    });

    // After the calendar element mounts, set up getDayParts and event listeners.
    Effect::new(move || {
        let Some(wrapper) = calendar_ref.get() else {
            return;
        };
        // The <calendar-date> is inside the wrapper div (after the Today button).
        let Some(el) = wrapper
            .query_selector("calendar-date")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return;
        };

        // Set initial heading from today's date.
        let now = js_sys::Date::new_0();
        if let Some(name) = month_name_from_js_date(&now) {
            set_heading.set(name);
        }

        // --- getDayParts: mark dates that have a daily note ---
        let dates = daily_dates.get();
        let get_day_parts = Closure::wrap(Box::new(move |date: JsValue| -> JsValue {
            if let Some(d) = date.dyn_ref::<js_sys::Date>() {
                let year = d.get_full_year();
                let month = d.get_month() + 1; // JS months are 0-based
                let day = d.get_date();
                let date_str = format!("{year:04}-{month:02}-{day:02}");
                if dates.contains(&date_str) {
                    return JsValue::from_str("has-note");
                }
            }
            JsValue::from_str("")
        }) as Box<dyn Fn(JsValue) -> JsValue>);

        js_sys::Reflect::set(
            &el,
            &JsValue::from_str("getDayParts"),
            get_day_parts.as_ref(),
        )
        .ok();
        get_day_parts.forget();

        // --- focusday event: update heading when the displayed month changes ---
        let focusday_handler = Closure::wrap(Box::new(move |ev: web_sys::CustomEvent| {
            if let Ok(date) = ev.detail().dyn_into::<js_sys::Date>() {
                if let Some(name) = month_name_from_js_date(&date) {
                    set_heading.set(name);
                }
            }
        }) as Box<dyn Fn(_)>);

        let target: &web_sys::EventTarget = el.as_ref();
        target
            .add_event_listener_with_callback("focusday", focusday_handler.as_ref().unchecked_ref())
            .ok();
        focusday_handler.forget();

        // --- change event: open daily note for selected date ---
        let change_handler = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let el_inner = wrapper
                .query_selector("calendar-date")
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
            let Some(el_inner) = el_inner else { return };

            let value = js_sys::Reflect::get(&el_inner, &JsValue::from_str("value"))
                .ok()
                .and_then(|v| v.as_string());
            let Some(date) = value.filter(|s| !s.is_empty()) else {
                return;
            };

            spawn_local(async move {
                match ipc::open_daily_note_for_date(&date).await {
                    Ok(note) => {
                        ctx.set_active_note_document(note);
                        // Refresh notes and folders so the tree and calendar stay in sync.
                        ctx.refresh_notes().await;
                        ctx.refresh_folders().await;
                    }
                    Err(e) => {
                        ctx.push_error("calendar", e);
                    }
                }
            });
        }) as Box<dyn Fn(_)>);

        target
            .add_event_listener_with_callback("change", change_handler.as_ref().unchecked_ref())
            .ok();
        change_handler.forget();
    });

    view! {
        <div node_ref=calendar_ref class="px-1 pt-1 pb-0.5 relative">
            <button
                class="absolute right-1.5 top-1 btn btn-ghost btn-xs btn-square"
                on:click=move |_| {
                    spawn_local(async move {
                        match ipc::open_daily_note().await {
                            Ok(note) => {
                                ctx.set_active_note_document(note);
                                ctx.refresh_notes().await;
                                ctx.refresh_folders().await;
                            }
                            Err(e) => { ctx.push_error("daily-note", e); }
                        }
                    });
                }
                title="Open daily note"
            >
                <span class="inline-flex w-3.5 h-3.5">
                    <Icon icon=icondata_lu::LuCalendar width="100%" height="100%"/>
                </span>
            </button>
            <calendar-date class="cally granit-calendar w-full" show-week-numbers>
                <svg aria-label="Previous" slot="previous" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="w-4 h-4 fill-none stroke-current stroke-2">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5 8.25 12l7.5-7.5"/>
                </svg>
                <span slot="heading" class="text-sm font-semibold capitalize">{heading}</span>
                <svg aria-label="Next" slot="next" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="w-4 h-4 fill-none stroke-current stroke-2">
                    <path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5"/>
                </svg>
                <calendar-month></calendar-month>
            </calendar-date>
        </div>
    }
}
