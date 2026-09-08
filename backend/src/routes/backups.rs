use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use granit_api::{
    BackupInfo, BackupState, CreateBackupRequest, CreateBackupResponse, DownloadBackupResponse,
    ListBackupsResponse,
};
use uuid::Uuid;

use crate::auth::AuthedKey;
use crate::error::ServerError;
use crate::routes::AppCtx;

struct BackupRow {
    id: Uuid,
    cave_name: String,
    size_bytes: i64,
    sha256_hex: String,
    state: String,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<BackupRow> for BackupInfo {
    fn from(row: BackupRow) -> Self {
        BackupInfo {
            id: row.id,
            cave_name: row.cave_name,
            size_bytes: row.size_bytes.max(0) as u64,
            sha256_hex: row.sha256_hex,
            // States are only ever written by this server; anything else is
            // an unfinished upload.
            state: match row.state.as_str() {
                "complete" => BackupState::Complete,
                _ => BackupState::Pending,
            },
            created_at: row.created_at,
            completed_at: row.completed_at,
        }
    }
}

/// `POST /api/v1/backups` — create a pending record and presign its upload.
pub async fn create(
    State(ctx): State<AppCtx>,
    auth: AuthedKey,
    Json(req): Json<CreateBackupRequest>,
) -> Result<Json<CreateBackupResponse>, ServerError> {
    let size_bytes = i64::try_from(req.size_bytes)
        .map_err(|_| ServerError::BadRequest("size_bytes out of range".to_string()))?;
    if req.cave_name.is_empty() {
        return Err(ServerError::BadRequest("cave_name is empty".to_string()));
    }

    let id = Uuid::new_v4();
    let object_key = format!("backups/{}/{}.grnt", auth.api_key_id, id);

    let row = sqlx::query_as!(
        BackupRow,
        r#"INSERT INTO backups (id, api_key_id, cave_name, object_key, size_bytes, sha256_hex)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, cave_name, size_bytes, sha256_hex, state, created_at, completed_at"#,
        id,
        auth.api_key_id,
        req.cave_name,
        object_key,
        size_bytes,
        req.sha256_hex,
    )
    .fetch_one(&ctx.db)
    .await?;

    let (upload_url, upload_expires_at) = ctx.storage.presign_put(&object_key).await?;

    Ok(Json(CreateBackupResponse {
        backup: row.into(),
        upload_url,
        upload_expires_at,
    }))
}

/// `POST /api/v1/backups/{id}/complete` — verify the upload and mark the
/// record complete. Idempotent for already-complete backups.
pub async fn complete(
    State(ctx): State<AppCtx>,
    auth: AuthedKey,
    Path(id): Path<Uuid>,
) -> Result<Json<BackupInfo>, ServerError> {
    let row = sqlx::query!(
        "SELECT object_key, size_bytes, state FROM backups WHERE id = $1 AND api_key_id = $2",
        id,
        auth.api_key_id,
    )
    .fetch_optional(&ctx.db)
    .await?
    .ok_or(ServerError::NotFound)?;

    if row.state != "complete" {
        let stored = ctx.storage.head_size(&row.object_key).await?;
        if stored != Some(row.size_bytes.max(0) as u64) {
            return Err(ServerError::Conflict(
                "uploaded object is missing or its size does not match".to_string(),
            ));
        }
        sqlx::query!(
            "UPDATE backups SET state = 'complete', completed_at = now() WHERE id = $1",
            id,
        )
        .execute(&ctx.db)
        .await?;
    }

    let row = sqlx::query_as!(
        BackupRow,
        r#"SELECT id, cave_name, size_bytes, sha256_hex, state, created_at, completed_at
           FROM backups WHERE id = $1"#,
        id,
    )
    .fetch_one(&ctx.db)
    .await?;
    Ok(Json(row.into()))
}

/// `GET /api/v1/backups/{id}/download` — presign a GET for a complete snapshot.
pub async fn download(
    State(ctx): State<AppCtx>,
    auth: AuthedKey,
    Path(id): Path<Uuid>,
) -> Result<Json<DownloadBackupResponse>, ServerError> {
    let row = sqlx::query!(
        "SELECT object_key, state FROM backups WHERE id = $1 AND api_key_id = $2",
        id,
        auth.api_key_id,
    )
    .fetch_optional(&ctx.db)
    .await?
    .ok_or(ServerError::NotFound)?;

    if row.state != "complete" {
        return Err(ServerError::Conflict(
            "backup upload has not been completed".to_string(),
        ));
    }

    let (download_url, download_expires_at) = ctx.storage.presign_get(&row.object_key).await?;

    Ok(Json(DownloadBackupResponse {
        download_url,
        download_expires_at,
    }))
}

/// `DELETE /api/v1/backups/{id}` — remove the stored object and the record.
/// Pending records may be deleted too (cleanup of abandoned uploads).
pub async fn delete(
    State(ctx): State<AppCtx>,
    auth: AuthedKey,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ServerError> {
    let row = sqlx::query!(
        "SELECT object_key FROM backups WHERE id = $1 AND api_key_id = $2",
        id,
        auth.api_key_id,
    )
    .fetch_optional(&ctx.db)
    .await?
    .ok_or(ServerError::NotFound)?;

    // Object first, row second: a failure in between leaves a row whose
    // object is gone, which a retried delete still removes.
    ctx.storage.delete_object(&row.object_key).await?;
    sqlx::query!("DELETE FROM backups WHERE id = $1", id)
        .execute(&ctx.db)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// `GET /api/v1/backups` — all snapshots for this key, newest first.
pub async fn list(
    State(ctx): State<AppCtx>,
    auth: AuthedKey,
) -> Result<Json<ListBackupsResponse>, ServerError> {
    let rows = sqlx::query_as!(
        BackupRow,
        r#"SELECT id, cave_name, size_bytes, sha256_hex, state, created_at, completed_at
           FROM backups WHERE api_key_id = $1 ORDER BY created_at DESC, id"#,
        auth.api_key_id,
    )
    .fetch_all(&ctx.db)
    .await?;

    Ok(Json(ListBackupsResponse {
        backups: rows.into_iter().map(Into::into).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sqlx::PgPool;
    use tower::ServiceExt;

    use super::*;
    use crate::auth::generate_api_key;
    use crate::routes::router;
    use crate::storage::Storage;

    fn test_ctx(pool: PgPool) -> AppCtx {
        AppCtx {
            db: pool,
            storage: Arc::new(Storage::stub()),
        }
    }

    async fn insert_key(pool: &PgPool, name: &str) -> (String, Uuid) {
        let (token, hash) = generate_api_key();
        let row = sqlx::query!(
            "INSERT INTO api_keys (name, key_hash) VALUES ($1, $2) RETURNING id",
            name,
            &hash[..],
        )
        .fetch_one(pool)
        .await
        .unwrap();
        (token, row.id)
    }

    fn create_request(token: &str, cave_name: &str) -> Request<Body> {
        let body = serde_json::to_string(&CreateBackupRequest {
            cave_name: cave_name.to_string(),
            size_bytes: 1234,
            sha256_hex: "ab".repeat(32),
        })
        .unwrap();
        Request::builder()
            .method("POST")
            .uri("/api/v1/backups")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    async fn json_body<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[sqlx::test]
    async fn rejects_missing_invalid_and_revoked_keys(pool: PgPool) {
        let ctx = test_ctx(pool.clone());

        let no_auth = Request::builder()
            .method("GET")
            .uri("/api/v1/backups")
            .body(Body::empty())
            .unwrap();
        let response = router(ctx.clone()).oneshot(no_auth).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let bad = Request::builder()
            .method("GET")
            .uri("/api/v1/backups")
            .header(AUTHORIZATION, "Bearer grnt_not-a-real-token")
            .body(Body::empty())
            .unwrap();
        let response = router(ctx.clone()).oneshot(bad).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let (token, id) = insert_key(&pool, "revoked").await;
        sqlx::query!("UPDATE api_keys SET revoked_at = now() WHERE id = $1", id)
            .execute(&pool)
            .await
            .unwrap();
        let revoked = Request::builder()
            .method("GET")
            .uri("/api/v1/backups")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = router(ctx).oneshot(revoked).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn create_returns_pending_record_with_upload_url(pool: PgPool) {
        let ctx = test_ctx(pool.clone());
        let (token, key_id) = insert_key(&pool, "laptop").await;

        let response = router(ctx)
            .oneshot(create_request(&token, "my-cave"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let created: CreateBackupResponse = json_body(response).await;

        assert_eq!(created.backup.state, BackupState::Pending);
        assert_eq!(created.backup.cave_name, "my-cave");
        assert_eq!(created.backup.size_bytes, 1234);
        let expected_key = format!("backups/{key_id}/{}.grnt", created.backup.id);
        assert!(created.upload_url.contains(&expected_key));
        assert!(created.upload_expires_at > Utc::now());
    }

    #[sqlx::test]
    async fn complete_verifies_size_and_is_idempotent(pool: PgPool) {
        let ctx = test_ctx(pool.clone());
        let (token, _) = insert_key(&pool, "laptop").await;

        let response = router(ctx.clone())
            .oneshot(create_request(&token, "my-cave"))
            .await
            .unwrap();
        let created: CreateBackupResponse = json_body(response).await;
        let complete_uri = format!("/api/v1/backups/{}/complete", created.backup.id);
        let complete_request = || {
            Request::builder()
                .method("POST")
                .uri(&complete_uri)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };

        // Object missing → conflict, record stays pending.
        let response = router(ctx.clone())
            .oneshot(complete_request())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // Wrong size → conflict.
        ctx.storage.stub_set_head_size(Some(999));
        let response = router(ctx.clone())
            .oneshot(complete_request())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // Matching size → complete.
        ctx.storage.stub_set_head_size(Some(1234));
        let response = router(ctx.clone())
            .oneshot(complete_request())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let info: BackupInfo = json_body(response).await;
        assert_eq!(info.state, BackupState::Complete);
        assert!(info.completed_at.is_some());

        // Second complete is idempotent even if the object vanished.
        ctx.storage.stub_set_head_size(None);
        let response = router(ctx.clone())
            .oneshot(complete_request())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let again: BackupInfo = json_body(response).await;
        assert_eq!(again.completed_at, info.completed_at);
    }

    #[sqlx::test]
    async fn download_presigns_complete_backups_only(pool: PgPool) {
        let ctx = test_ctx(pool.clone());
        let (token, key_id) = insert_key(&pool, "laptop").await;

        let response = router(ctx.clone())
            .oneshot(create_request(&token, "my-cave"))
            .await
            .unwrap();
        let created: CreateBackupResponse = json_body(response).await;
        let download_request = |token: &str| {
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/backups/{}/download", created.backup.id))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };

        // Still pending → conflict.
        let response = router(ctx.clone())
            .oneshot(download_request(&token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        ctx.storage.stub_set_head_size(Some(1234));
        let complete = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/backups/{}/complete", created.backup.id))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = router(ctx.clone()).oneshot(complete).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Complete → presigned URL for the snapshot's object.
        let response = router(ctx.clone())
            .oneshot(download_request(&token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let downloaded: DownloadBackupResponse = json_body(response).await;
        let expected_key = format!("backups/{key_id}/{}.grnt", created.backup.id);
        assert!(downloaded.download_url.contains(&expected_key));
        assert!(downloaded.download_expires_at > Utc::now());

        // Another key's backup → not found.
        let (foreign_token, _) = insert_key(&pool, "desktop").await;
        let response = router(ctx.clone())
            .oneshot(download_request(&foreign_token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Unknown id → not found.
        let unknown = Request::builder()
            .method("GET")
            .uri(format!("/api/v1/backups/{}/download", Uuid::new_v4()))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = router(ctx).oneshot(unknown).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn delete_removes_pending_and_complete_rows_but_not_foreign_ones(pool: PgPool) {
        let ctx = test_ctx(pool.clone());
        let (token, _) = insert_key(&pool, "laptop").await;

        let response = router(ctx.clone())
            .oneshot(create_request(&token, "my-cave"))
            .await
            .unwrap();
        let created: CreateBackupResponse = json_body(response).await;
        let delete_request = |token: &str, id: Uuid| {
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/backups/{id}"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };

        // Another key cannot delete it.
        let (foreign_token, _) = insert_key(&pool, "desktop").await;
        let response = router(ctx.clone())
            .oneshot(delete_request(&foreign_token, created.backup.id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Deleting the still-pending record is allowed (abandoned upload
        // cleanup) even though its object was never stored.
        let response = router(ctx.clone())
            .oneshot(delete_request(&token, created.backup.id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let remaining = sqlx::query!("SELECT count(*) AS n FROM backups")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining.n, Some(0));

        // Deleting it again → 404 (row already gone).
        let response = router(ctx)
            .oneshot(delete_request(&token, created.backup.id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn list_is_scoped_to_the_authenticated_key(pool: PgPool) {
        let ctx = test_ctx(pool.clone());
        let (token_a, _) = insert_key(&pool, "laptop").await;
        let (token_b, _) = insert_key(&pool, "desktop").await;

        for (token, cave) in [(&token_a, "cave-a"), (&token_b, "cave-b")] {
            let response = router(ctx.clone())
                .oneshot(create_request(token, cave))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let list_request = Request::builder()
            .method("GET")
            .uri("/api/v1/backups")
            .header(AUTHORIZATION, format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap();
        let response = router(ctx).oneshot(list_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let listed: ListBackupsResponse = json_body(response).await;
        assert_eq!(listed.backups.len(), 1);
        assert_eq!(listed.backups[0].cave_name, "cave-a");
    }
}
