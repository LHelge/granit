use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::generate_api_key;
use crate::error::ServerError;

/// `create-key`: mint a new API key and print the token exactly once.
pub async fn create_key(pool: &PgPool, name: &str) -> Result<(), ServerError> {
    let (token, hash) = generate_api_key();
    let row = sqlx::query!(
        "INSERT INTO api_keys (name, key_hash) VALUES ($1, $2) RETURNING id",
        name,
        &hash[..],
    )
    .fetch_one(pool)
    .await?;

    println!("Created API key '{name}' ({})", row.id);
    println!();
    println!("  {token}");
    println!();
    println!("Store it now — only its hash is kept and it cannot be shown again.");
    Ok(())
}

/// `list-keys`: print all keys with their status.
pub async fn list_keys(pool: &PgPool) -> Result<(), ServerError> {
    let rows =
        sqlx::query!("SELECT id, name, created_at, revoked_at FROM api_keys ORDER BY created_at")
            .fetch_all(pool)
            .await?;

    if rows.is_empty() {
        println!("No API keys. Create one with: granit-server create-key --name <name>");
        return Ok(());
    }
    for row in rows {
        let status = match row.revoked_at {
            Some(at) => format!("revoked {}", at.format("%Y-%m-%d %H:%M")),
            None => "active".to_string(),
        };
        println!(
            "{}  {:<20} created {}  {status}",
            row.id,
            row.name,
            row.created_at.format("%Y-%m-%d %H:%M"),
        );
    }
    Ok(())
}

/// Replace the bearer secret atomically while retaining the ownership id.
/// Requests authenticated before rotation may finish; new requests using the
/// old token fail as soon as this update commits.
async fn rotate_key(pool: &PgPool, id: Uuid) -> Result<(String, String), ServerError> {
    let (token, hash) = generate_api_key();
    let name = sqlx::query_scalar!(
        "UPDATE api_keys SET key_hash = $2 WHERE id = $1 AND revoked_at IS NULL RETURNING name",
        id,
        &hash[..],
    )
    .fetch_optional(pool)
    .await?
    .ok_or(ServerError::NotFound)?;
    Ok((name, token))
}

/// `revoke-key`: invalidate the old token and print its replacement once.
/// Keeping the ownership id also preserves pending uploads and object paths.
pub async fn revoke_key(pool: &PgPool, id: Uuid) -> Result<(), ServerError> {
    let (name, token) = rotate_key(pool, id).await?;
    println!("Revoked old token and created replacement for API key '{name}' ({id})");
    println!();
    println!("  {token}");
    println!();
    println!("Store it now — only its hash is kept and it cannot be shown again.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::hash_token;
    use crate::routes::{router, AppCtx};
    use crate::storage::Storage;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use granit_api::ListBackupsResponse;
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[sqlx::test]
    async fn rotation_rejects_old_token_and_keeps_backup_ownership(pool: PgPool) {
        let (old_token, hash) = generate_api_key();
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO api_keys (name, key_hash) VALUES ('laptop', $1) RETURNING id",
        )
        .bind(&hash[..])
        .fetch_one(&pool)
        .await
        .unwrap();
        let snapshot = Uuid::new_v4();
        sqlx::query("INSERT INTO backups (id, api_key_id, cave_name, object_key, size_bytes, sha256_hex, state) VALUES ($1, $2, 'cave', 'existing/path.grnt', 1234, $3, 'complete')")
            .bind(snapshot).bind(id).bind("ab".repeat(32)).execute(&pool).await.unwrap();

        let (name, replacement) = rotate_key(&pool, id).await.unwrap();
        assert_eq!(name, "laptop");
        assert_ne!(old_token, replacement);
        let stored: Vec<u8> = sqlx::query_scalar("SELECT key_hash FROM api_keys WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, hash_token(&replacement));
        let ctx = AppCtx {
            db: pool.clone(),
            storage: Arc::new(Storage::stub()),
        };
        let request = |token: &str, path: &str| {
            Request::builder()
                .uri(path)
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };
        let old = router(ctx.clone())
            .oneshot(request(&old_token, "/api/v1/backups"))
            .await
            .unwrap();
        assert_eq!(old.status(), StatusCode::UNAUTHORIZED);
        let response = router(ctx.clone())
            .oneshot(request(&replacement, "/api/v1/backups"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let listed: ListBackupsResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(listed.backups.len(), 1);
        assert_eq!(listed.backups[0].id, snapshot);
        let download = router(ctx)
            .oneshot(request(
                &replacement,
                &format!("/api/v1/backups/{snapshot}/download"),
            ))
            .await
            .unwrap();
        assert_eq!(download.status(), StatusCode::OK);
    }

    #[sqlx::test]
    async fn rotation_does_not_resurrect_revoked_or_unknown_keys(pool: PgPool) {
        let (_, hash) = generate_api_key();
        let id: Uuid = sqlx::query_scalar("INSERT INTO api_keys (name, key_hash, revoked_at) VALUES ('revoked', $1, now()) RETURNING id")
            .bind(&hash[..]).fetch_one(&pool).await.unwrap();
        for id in [id, Uuid::new_v4()] {
            assert!(matches!(
                rotate_key(&pool, id).await,
                Err(ServerError::NotFound)
            ));
        }
        let stored: Vec<u8> = sqlx::query_scalar("SELECT key_hash FROM api_keys WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, hash);
    }
}
