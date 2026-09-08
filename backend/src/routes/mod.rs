mod backups;

use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use sqlx::PgPool;

use crate::storage::Storage;

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppCtx {
    pub db: PgPool,
    pub storage: Arc<Storage>,
}

pub fn router(ctx: AppCtx) -> Router {
    Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/backups", post(backups::create).get(backups::list))
                .route("/backups/{id}", delete(backups::delete))
                .route("/backups/{id}/complete", post(backups::complete))
                .route("/backups/{id}/download", get(backups::download)),
        )
        .route("/healthz", get(|| async { "ok" }))
        .with_state(ctx)
}
