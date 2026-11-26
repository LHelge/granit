//! Spike gate G6: replay a canned agent-style markdown response into a GPUI
//! Kit `TextView` in small chunks (~30 per second), the way a streamed agent
//! reply arrives.
//!
//! Run with `cargo run -p granit-gpui --bin stream_smoke`.
//!
//! Environment switches:
//! - `GRANIT_SMOKE_NO_FADE=1` turns off the Kit's streamed fade-in, to judge
//!   raw append rendering.
//! - `GRANIT_SMOKE_EXIT=1` quits once the replay has finished (headless
//!   "does it run to the end without panicking" check).

#[path = "../theme.rs"]
mod theme;

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use gpui::{
    div, px, size, App, AppContext, Bounds, Context, Entity, FollowMode, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Task, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_kit::component::{
    text::{TextView, TextViewState},
    ActiveTheme,
};

gpui_kit::actions!(stream_smoke, [Quit]);

/// Characters handed to the view per tick.
const CHUNK_CHARS: usize = 6;
/// Tick interval: ~30 chunks per second.
const CHUNK_INTERVAL: Duration = Duration::from_millis(33);
/// URL scheme the wiki-link rewriter targets, so the click handler can tell
/// wiki-links from web links.
const WIKI_SCHEME: &str = "wiki:";

const RESPONSE: &str = r#"# Trip planning summary

Here is what I found in your notes about the **Gothenburg trip**, plus a few
suggestions. See [[Travel log]] for the earlier entries and
[[Packing list|the packing list]] for what is still missing.

## Itinerary

1. Friday
   - Train from Stockholm at `07:12`
   - Check in at the hotel
     - Ask for a late checkout
     - Confirm breakfast hours
2. Saturday
   - Archipelago ferry, see the [timetable](https://www.vasttrafik.se/)
   - Dinner near Haga
3. Sunday
   - Return train

## Open tasks

- [x] Book train tickets
- [x] Reserve hotel
- [ ] Buy ferry day pass
- [ ] Ask Anna about the [[Volvo]] museum visit

## Budget

| Item          | Cost (SEK) | Paid |
| ------------- | ---------: | :--: |
| Train         |      1 240 | yes  |
| Hotel, 2 nts  |      2 980 | yes  |
| Ferry pass    |        120 |  no  |
| Food (est.)   |      1 500 |  no  |

## Script to total the budget

```rust
fn total(items: &[(&str, u32)]) -> u32 {
    items.iter().map(|(_, cost)| cost).sum()
}

fn main() {
    let items = [("Train", 1240), ("Hotel", 2980), ("Ferry", 120)];
    println!("total: {} SEK", total(&items));
}
```

Inline code like `cargo run --bin stream_smoke` should keep its background,
and a long paragraph should wrap cleanly while it streams in, without the
lines above it jumping around. This sentence is deliberately long so that the
reflow of a growing paragraph is easy to watch: it keeps going for a while,
touching on ferries, trams, rain, and the fact that Gothenburg weather is
famously unpredictable in every season of the year.

> A blockquote, because agents like to quote notes back at you.
> It spans two lines.

### Raw HTML probe

This line has <b>inline bold HTML</b> and <span style="color:red">a styled span</span>.

<div>
A raw HTML block. Granit sanitizes HTML today; check what the Kit does with it.
</div>

<script>alert("this must not run")</script>

### Wiki-link edge cases

- Plain: [[Travel log]]
- With label: [[Packing list|packing]]
- Heading anchor target: [[Volvo]]
- Unclosed at end of line: [[not a link
- In inline code (rewriter is naive, expect a false positive): `[[code]]`

---

That is everything. Let me know if I should add the ferry pass to your todo
list.
"#;

/// Rewrites `[[target]]` / `[[target|label]]` into normal Markdown links
/// pointing at `wiki:target`, on a stream of chunks.
///
/// An opening `[[` without its closing `]]` yet is held back until the rest
/// arrives, so the text handed to the view only ever grows: the view keeps
/// appending instead of seeing a changed prefix (which would force a full
/// replace and re-parse).
///
/// Deliberately naive: it does not know about code spans or fences.
#[derive(Default)]
struct WikiLinkRewriter {
    pending: String,
}

impl WikiLinkRewriter {
    /// Feed a chunk, returning the text that is safe to append now.
    fn feed(&mut self, chunk: &str) -> String {
        self.pending.push_str(chunk);
        let mut out = String::new();
        loop {
            let Some(open) = self.pending.find("[[") else {
                // Hold back a lone trailing `[`: it may become `[[`.
                let keep = usize::from(self.pending.ends_with('['));
                let emit = self.pending.len() - keep;
                out.push_str(&self.pending[..emit]);
                self.pending.drain(..emit);
                return out;
            };
            out.push_str(&self.pending[..open]);
            self.pending.drain(..open);
            // `pending` now starts with `[[`.
            let rest = &self.pending[2..];
            let close = rest.find("]]");
            let newline = rest.find('\n');
            match (close, newline) {
                (Some(close), newline) if newline.is_none_or(|nl| close < nl) && close > 0 => {
                    let inner = &rest[..close];
                    let (target, label) = inner.split_once('|').unwrap_or((inner, inner));
                    out.push_str(&format!("[{label}](<{WIKI_SCHEME}{target}>)"));
                    self.pending.drain(..2 + close + 2);
                }
                (None, None) => return out, // wait for more input
                _ => {
                    // Empty, or broken by a line end: not a wiki-link.
                    out.push_str("[[");
                    self.pending.drain(..2);
                }
            }
        }
    }

    /// Flush whatever is still held back, literally.
    fn finish(&mut self) -> String {
        std::mem::take(&mut self.pending)
    }
}

/// Split `text` into chunks of `CHUNK_CHARS` characters (char boundaries).
fn chunks(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(CHUNK_CHARS)
        .map(|c| c.iter().collect())
        .collect()
}

struct StreamSmoke {
    state: Entity<TextViewState>,
    stream: Option<Task<()>>,
    streaming: bool,
    sent_chunks: usize,
    total_chunks: usize,
    last_link: Arc<Mutex<Option<String>>>,
    fade: bool,
    _appearance: Subscription,
}

impl StreamSmoke {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = cx.observe_window_appearance(window, |_, window, cx| {
            theme::sync(window, cx);
        });
        let state = cx.new(|cx| TextViewState::markdown("", cx));
        // Snap to the end on each layout while the user is at the bottom;
        // scrolling up pauses it, scrolling back down resumes it.
        state
            .read(cx)
            .list_state()
            .set_follow_mode(FollowMode::Tail);
        let mut this = Self {
            state,
            stream: None,
            streaming: false,
            sent_chunks: 0,
            total_chunks: 0,
            last_link: Arc::default(),
            fade: std::env::var_os("GRANIT_SMOKE_NO_FADE").is_none(),
            _appearance: appearance,
        };
        this.start(cx);
        this
    }

    /// (Re)start the replay from an empty view.
    fn start(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.set_text("", cx));
        self.state
            .read(cx)
            .list_state()
            .set_follow_mode(FollowMode::Tail);
        let chunks = chunks(RESPONSE);
        self.total_chunks = chunks.len();
        self.sent_chunks = 0;
        self.streaming = true;
        let state = self.state.clone();
        self.stream = Some(cx.spawn(async move |this, cx| {
            let started = Instant::now();
            let mut rewriter = WikiLinkRewriter::default();
            let mut late_ticks = 0usize;
            let mut deadline = Instant::now();
            for chunk in chunks {
                deadline += CHUNK_INTERVAL;
                cx.background_executor()
                    .timer(deadline.saturating_duration_since(Instant::now()))
                    .await;
                if Instant::now() > deadline + CHUNK_INTERVAL {
                    late_ticks += 1;
                }
                let text = rewriter.feed(&chunk);
                state.update(cx, |state, cx| state.push_str(&text, cx));
                let alive = this.update(cx, |this, cx| {
                    this.sent_chunks += 1;
                    cx.notify();
                });
                if alive.is_err() {
                    return;
                }
            }
            let tail = rewriter.finish();
            state.update(cx, |state, cx| state.push_str(&tail, cx));
            eprintln!(
                "stream_smoke: replayed {} chars in {:.2?} ({} ticks more than one interval late)",
                RESPONSE.len(),
                started.elapsed(),
                late_ticks,
            );
            _ = this.update(cx, |this, cx| {
                this.streaming = false;
                cx.notify();
            });
            if std::env::var_os("GRANIT_SMOKE_EXIT").is_some() {
                cx.update(|cx| cx.quit());
            }
        }));
        cx.notify();
    }
}

impl Render for StreamSmoke {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let following = self.state.read(cx).list_state().is_following_tail();
        let last_link = self
            .last_link
            .lock()
            .ok()
            .and_then(|link| link.clone())
            .unwrap_or_else(|| "none".into());
        let status: SharedString = format!(
            "{} {}/{} chunks | follow tail: {} | last link click: {}",
            if self.streaming { "streaming" } else { "done" },
            self.sent_chunks,
            self.total_chunks,
            if following { "on" } else { "paused" },
            last_link,
        )
        .into();

        let last_link = self.last_link.clone();
        let view = TextView::new(&self.state)
            .scrollable(true)
            .selectable(true)
            .stream_fade(self.fade)
            .on_link_click(move |url, _, window, _| {
                let kind = if url.starts_with(WIKI_SCHEME) {
                    "wiki"
                } else {
                    "web"
                };
                eprintln!("stream_smoke: link click ({kind}): {url}");
                if let Ok(mut last) = last_link.lock() {
                    *last = Some(format!("{kind} {url}"));
                }
                window.refresh();
            });

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .text_sm()
                    .child(
                        div()
                            .id("replay")
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(theme.secondary)
                            .hover(|s| s.bg(theme.secondary_hover))
                            .cursor_pointer()
                            .child("Replay")
                            .on_click(cx.listener(|this, _, _, cx| this.start(cx))),
                    )
                    .child(div().text_color(theme.muted_foreground).child(status)),
            )
            // `scrollable` needs a parent with a definite height.
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .py_2()
                    .child(view.size_full()),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);

        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(820.), px(720.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Granit stream smoke test".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            theme::install(window, cx);
            cx.new(|cx| StreamSmoke::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite_in_chunks(text: &str, size: usize) -> String {
        let mut rewriter = WikiLinkRewriter::default();
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        for chunk in chars.chunks(size) {
            out.push_str(&rewriter.feed(&chunk.iter().collect::<String>()));
        }
        out.push_str(&rewriter.finish());
        out
    }

    #[test]
    fn rewrites_wiki_links_split_across_chunks() {
        let text = "See [[Travel log]] and [[Packing list|the list]].";
        let expected = "See [Travel log](<wiki:Travel log>) and [the list](<wiki:Packing list>).";
        for size in 1..=text.len() {
            assert_eq!(rewrite_in_chunks(text, size), expected, "chunk size {size}");
        }
    }

    #[test]
    fn leaves_unclosed_and_normal_links_alone() {
        let text = "[[open\nnext [a](b) [[]] end [";
        for size in 1..=text.len() {
            assert_eq!(rewrite_in_chunks(text, size), text, "chunk size {size}");
        }
    }
}
