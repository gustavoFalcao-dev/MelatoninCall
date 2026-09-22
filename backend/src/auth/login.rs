use bcrypt::verify;
use::serde::{
    Serialize,
    Deserialize
};
use::axum::{
    extract::State,
    http::StatusCode,
    Json
};
use uuid::Uuid;
use crate::state::AppState;


#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}
#[derive(Serialize)]
pub struct LoginResponse {
    access_token: String,
    refresh_token: String,
}


pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)>{
    let user = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, password_hash FROM users WHERE username = $1"
    )
    .bind(&payload.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't connect to the server.".to_string()))?;

    let user = match user {
        Some(row) => row,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid username or password.".to_string()
            ));
        }
    };

    let (id, password_hash) = user;

    let is_valid = verify(&payload.password, &password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Login failed.".to_string()))?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password.".to_string()
        ));
    }

    let access_token = state.jwt
    .create_token(id)
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to create authentication token.".to_string()
    ))?;

    let refresh_token = crate::auth::refresh::generate_refresh_token();

    let token_hash = crate::auth::refresh::hash_refresh_token(&refresh_token);

    let refresh_token_id = Uuid::now_v7();

    let refresh_ttl = i64::try_from(state.jwt.refresh_ttl)
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "JWT_REFRESH_TTL is too large.".to_string()
    ))?;
    
    let expires_at = chrono::Utc::now()
    + chrono::Duration::seconds(refresh_ttl);

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)"
    )
    .bind(refresh_token_id)
    .bind(id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to create refresh token.".to_string()
    ))?;

    Ok((
        StatusCode::OK,
        Json(LoginResponse { 
            access_token,
            refresh_token,
        })
    ))
}