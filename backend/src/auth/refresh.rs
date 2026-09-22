use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{
    Serialize,
    Deserialize,
};
use sha2::{
    Digest,
    Sha256,
};
use uuid::Uuid;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: String,
}
#[derive(Serialize)]
pub struct RefreshResponse {
    access_token: String,
    refresh_token: String,
}


pub fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];

    rand::fill(&mut bytes);

    hex::encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());

    hex::encode(hash)
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>
) -> Result<(StatusCode, Json<RefreshResponse>), (StatusCode, String)> {
    
    let token_hash = hash_refresh_token(&payload.refresh_token);

    let mut tx = state.db
    .begin()
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to start database transaction.".to_string()
    ))?;

    let refresh_token = sqlx::query_as::<_, (Uuid, Uuid, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, user_id, expires_at FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE"
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to query refresh token.".to_string(),
    ))?;

    let refresh_token = match refresh_token {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid refresh token.".to_string(),
            ));
        }
    };

    let (refresh_token_id, user_id, expires_at) = refresh_token;

    if expires_at <= chrono::Utc::now() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Refresh token has expired.".to_string(),
        ));
    }
  
    let new_refresh_token = generate_refresh_token();

    let new_token_hash = hash_refresh_token(&new_refresh_token);

    let refresh_ttl = i64::try_from(state.jwt.refresh_ttl)
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "JWT_REFRESH_TTL is too large.".to_string()
    ))?;

    let new_expires_at = chrono::Utc::now() + chrono::Duration::seconds(refresh_ttl);

    let result = sqlx::query(
        "UPDATE refresh_tokens SET token_hash = $1, expires_at = $2 WHERE id = $3"
    )
    .bind(&new_token_hash)
    .bind(new_expires_at)
    .bind(refresh_token_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to rotate refresh token.".to_string()
    ))?;

    if result.rows_affected() != 1 {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Refresh token is no longer valid.".to_string()
        ));
    }

    let access_token = state.jwt.create_token(user_id)
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to create access token.".to_string()
    ))?;

    tx.commit()
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to commit refresh token rotation.".to_string()
    ))?;

    Ok((
        StatusCode::OK,
        Json(RefreshResponse {
            access_token, 
            refresh_token: new_refresh_token 
        }),
    ))
    
}