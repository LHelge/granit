use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ServerError;
use crate::routes::AppCtx;

/// Prefix on every issued API key, so keys are recognizable in configs.
const KEY_PREFIX: &str = "grnt_";

/// The authenticated API key, extracted from the `Authorization` header.
pub struct AuthedKey {
    pub api_key_id: Uuid,
}

impl FromRequestParts<AppCtx> for AuthedKey {
    type Rejection = ServerError;

    async fn from_request_parts(parts: &mut Parts, ctx: &AppCtx) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(ServerError::Unauthorized)?;

        let hash = hash_token(token);
        let row = sqlx::query!(
            "SELECT id FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL",
            &hash[..],
        )
        .fetch_optional(&ctx.db)
        .await?;

        match row {
            Some(row) => Ok(AuthedKey { api_key_id: row.id }),
            None => Err(ServerError::Unauthorized),
        }
    }
}

/// Generate a fresh API key. Returns the token to hand to the user (shown
/// exactly once) and the hash to persist. The token is 256 bits of CSPRNG
/// output, which is why plain SHA-256 (rather than a password KDF) is enough
/// for storage: brute force is infeasible and the deterministic hash allows an
/// indexed lookup per request.
pub fn generate_api_key() -> (String, [u8; 32]) {
    let mut secret = [0u8; 32];
    rand::rng().fill_bytes(&mut secret);
    let token = format!(
        "{KEY_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret)
    );
    let hash = hash_token(&token);
    (token, hash)
}

pub fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_unique_and_hash_consistently() {
        let (token_a, hash_a) = generate_api_key();
        let (token_b, hash_b) = generate_api_key();
        assert_ne!(token_a, token_b);
        assert_ne!(hash_a, hash_b);
        assert!(token_a.starts_with(KEY_PREFIX));
        assert_eq!(hash_token(&token_a), hash_a);
    }
}
