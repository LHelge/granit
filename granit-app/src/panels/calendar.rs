//! The calendar of daily notes, above the sidebar's tabs (as in the Tauri
//! app): one month of ISO weeks, Monday first, with week numbers. Days that
//! have a daily note are marked, today is outlined and the open note's day
//! is filled. A click on a day opens that day's note, creating it from the
//! daily-note template when it does not exist yet
//! ([`crate::cave_ops::open_daily_note`]).
//!
//! The month grid is the core's ([`granit_core::calendar::Month`]); the
//! marked days are the core's [`granit_core::cave::Cave::daily_note_slugs`],
//! a lookup in the note index cheap enough to redo on every cave change.

use std::collections::HashSet;

use gpui::{
    div, prelude::FluentBuilder as _, px, Context, EventEmitter, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, IconName, Sizable,
};
use granit_core::calendar::{today_slug, Month, WEEKDAYS};
use granit_core::cave::CaveError;
use log::warn;

use super::{error_line, PanelEvent};
use crate::cave_ops;
use crate::events::{CaveEvent, CaveEvents};

/// The height of a day cell.
const CELL_HEIGHT: gpui::Pixels = px(22.);

pub struct CalendarPanel {
    month: Month,
    /// The daily notes that exist.
    notes: HashSet<String>,
    /// The open note, filled when it is a day of the month.
    active: Option<String>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for CalendarPanel {}

impl CalendarPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
            if *event == CaveEvent::Opened {
                this.month = Month::current();
                this.error = None;
            }
            this.refresh(cx);
        })];
        let mut this = Self {
            month: Month::current(),
            notes: HashSet::new(),
            active: None,
            error: None,
            _subscriptions: subscriptions,
        };
        this.refresh(cx);
        this
    }

    /// Re-read which days have a note.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.notes = match cave_ops::daily_note_slugs(cx) {
            Ok(notes) => {
                self.error = None;
                notes
            }
            Err(CaveError::NoCaveOpen) => HashSet::new(),
            Err(err) => {
                warn!("Could not list the daily notes: {err}");
                self.error = Some(format!("Could not list the daily notes: {err}").into());
                HashSet::new()
            }
        };
        cx.notify();
    }

    /// The note open in the main area changed.
    pub fn set_active(&mut self, slug: Option<String>, cx: &mut Context<Self>) {
        self.active = slug;
        cx.notify();
    }

    /// Show `month`.
    pub fn show_month(&mut self, month: Month, cx: &mut Context<Self>) {
        self.month = month;
        cx.notify();
    }

    /// The month shown.
    #[cfg(test)]
    pub fn month(&self) -> Month {
        self.month
    }

    /// The days of the shown month that have a note, for tests.
    #[cfg(test)]
    pub fn marked_days(&self) -> Vec<String> {
        let mut days: Vec<String> = self
            .month
            .weeks()
            .into_iter()
            .flat_map(|week| week.days)
            .filter(|day| day.in_month && self.notes.contains(&day.slug))
            .map(|day| day.slug)
            .collect();
        days.sort();
        days
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// Open (or create) the daily note of `slug` (`YYYY-MM-DD`).
    pub fn open_day(&mut self, slug: &str, cx: &mut Context<Self>) {
        match cave_ops::open_daily_note(Some(slug), cx) {
            Ok(meta) => {
                self.error = None;
                cx.emit(PanelEvent::OpenNote {
                    slug: meta.slug,
                    reveal: None,
                });
            }
            Err(err) => {
                warn!("Could not open the daily note of {slug}: {err}");
                self.error = Some(format!("Could not open the daily note: {err}").into());
            }
        }
        cx.notify();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .child(
                Button::new("calendar-prev")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronLeft)
                    .tooltip("Previous month")
                    .on_click(cx.listener(|this, _, _, cx| {
                        let month = this.month.prev();
                        this.show_month(month, cx)
                    })),
            )
            .child(
                div().flex_1().flex().justify_center().child(
                    Button::new("calendar-month")
                        .ghost()
                        .xsmall()
                        .label(self.month.title())
                        .tooltip("Go to this month")
                        .on_click(
                            cx.listener(|this, _, _, cx| this.show_month(Month::current(), cx)),
                        ),
                ),
            )
            .child(
                Button::new("calendar-next")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronRight)
                    .tooltip("Next month")
                    .on_click(cx.listener(|this, _, _, cx| {
                        let month = this.month.next();
                        this.show_month(month, cx)
                    })),
            )
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let today = today_slug();
        let cell = || {
            div()
                .flex_1()
                .h(CELL_HEIGHT)
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
        };
        let heading = h_flex()
            .text_color(theme.muted_foreground)
            .child(cell().child("Wk"))
            .children(WEEKDAYS.iter().map(|day| cell().child(*day)));
        let weeks = self.month.weeks().into_iter().map(|week| {
            h_flex()
                .child(
                    cell()
                        .text_color(theme.muted_foreground.opacity(0.6))
                        .child(week.number.to_string()),
                )
                .children(week.days.into_iter().map(|day| {
                    let has_note = self.notes.contains(&day.slug);
                    let is_active = self.active.as_deref() == Some(day.slug.as_str());
                    let slug = day.slug.clone();
                    cell().child(
                        div()
                            .id(SharedString::from(format!("day-{}", day.slug)))
                            .size(CELL_HEIGHT)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .cursor_pointer()
                            .when(!day.in_month, |this| {
                                this.text_color(theme.muted_foreground.opacity(0.5))
                            })
                            .when(has_note, |this| {
                                this.font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.primary)
                            })
                            .when(day.slug == today, |this| {
                                this.border_1().border_color(theme.primary)
                            })
                            .when(is_active, |this| {
                                this.bg(theme.primary).text_color(theme.primary_foreground)
                            })
                            .when(!is_active, |this| this.hover(|this| this.bg(theme.muted)))
                            .on_click(cx.listener(move |this, _, _, cx| this.open_day(&slug, cx)))
                            .child(day.day.to_string()),
                    )
                }))
        });
        v_flex().child(heading).children(weeks)
    }
}

impl Render for CalendarPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_none()
            .px_2()
            .pb_1()
            .child(self.render_header(cx))
            .child(self.render_grid(cx))
            .children(error_line(self.error.clone(), cx))
    }
}
