//! The tokio runtime the core's async work runs on, and its bridge to GPUI.
//!
//! GPUI has its own executors (`cx.spawn` on the foreground thread,
//! `cx.background_spawn` on its thread pool), but the core needs tokio: the
//! RAG index spawns its debounced updates on tokio, and the agent streams
//! through `reqwest`. One multi-threaded runtime lives for the whole process.
//!
//! To await tokio work from GPUI, spawn it here and await the returned
//! [`tokio::task::JoinHandle`] inside `cx.spawn`; a join handle is an
//! ordinary future that any executor can poll:
//!
//! ```ignore
//! cx.spawn(async move |this, cx| {
//!     let result = runtime::spawn(async move { do_tokio_work().await }).await;
//!     this.update(cx, |this, cx| this.finish(result, cx)).ok();
//! })
//! .detach();
//! ```

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Builder, Handle, Runtime};
use tokio::task::JoinHandle;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// The process-wide tokio runtime, started on first use.
pub fn handle() -> Handle {
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .enable_all()
                .thread_name("granit-tokio")
                .build()
                .expect("failed to start the tokio runtime")
        })
        .handle()
        .clone()
}

/// Run `future` on the tokio runtime. Await the handle from a GPUI task to
/// get its output back on the foreground.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    handle().spawn(future)
}

/// [`spawn`], but dropping the returned future aborts the tokio task. Use
/// it for work a GPUI task awaits and that must stop when that task is
/// dropped (a cancelled agent stream closes its connection).
pub fn spawn_abortable<F>(future: F) -> AbortOnDrop<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    AbortOnDrop(spawn(future))
}

/// A tokio join handle that aborts its task when dropped; see
/// [`spawn_abortable`].
pub struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Future for AbortOnDrop<T> {
    type Output = Result<T, tokio::task::JoinError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.0).poll(cx)
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    async fn tokio_work_is_awaited_from_gpui(cx: &mut gpui::TestAppContext) {
        let task = cx.spawn(async move |_| {
            spawn(async { Handle::try_current().is_ok() })
                .await
                .unwrap()
        });
        // The tokio thread finishes on its own time, outside GPUI's
        // deterministic test executor.
        cx.executor().allow_parking();
        assert!(task.await, "the future ran inside the tokio runtime");
    }
}
