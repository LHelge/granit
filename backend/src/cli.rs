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

/// `revoke-key`: revoke a key by id. Existing backups are kept.
pub async fn revoke_key(pool: &PgPool, id: Uuid) -> Result<(), ServerError> {
    let result = sqlx::query!(
        "UPDATE api_keys SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
        id,
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ServerError::NotFound);
    }
    println!("Revoked API key {id}");
    Ok(())
}
