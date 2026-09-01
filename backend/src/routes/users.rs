use axum::{
    extract::State,
    http::StatusCode,
    Json
};
use serde::{
    Deserialize,
    Serialize
};
use bcrypt::{
    hash,
    verify,
    DEFAULT_COST
};
use uuid::Uuid;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
    email: String
}
#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String
}
#[derive(Serialize)]
pub struct RegisterResponse {
    id: Uuid,
    username: String,
}
#[derive(Serialize)]
pub struct LoginResponse {
    id: Uuid,
    username: String
}

fn is_valid_username(username: &str) -> bool {
    let len_ok = username.len() >=4 && username.len() <= 30;

    let char_ok = username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');

    let start_end_ok = 
    username.chars().next().is_some_and(|c| c.is_alphanumeric()) &&
    username.chars().last().is_some_and(|c| c.is_alphanumeric());

    let is_consecutive_punctuation =  !username.as_bytes().windows(2).any(|w| matches!(w[0], b'_' | b'-') && matches!(w[1], b'_' | b'-'));

    len_ok && char_ok && start_end_ok && is_consecutive_punctuation
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
    
    if !is_valid_username(&payload.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username can only contain letters, numbers, hyphens and underscores.".to_string()
        ));
    }

    let id = Uuid::now_v7();

    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed.".to_string()))?;

    let result = sqlx::query_as::<_, (Uuid, String)>(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING id, username"
    )
    .bind(&id)
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(&password_hash)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, username)) => Ok((StatusCode::CREATED, Json(RegisterResponse { id, username }))),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            let constraint = db_err.constraint().unwrap_or("");
            let msg = if constraint.contains("email") {
                "Email already registered."
            } else {
                "Username already taken."
            };
            Err((StatusCode::CONFLICT, msg.to_string()))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Registration failed.".to_string()))
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)>{
    let user = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, username, password_hash FROM users WHERE username = $1"
    )
    .bind(&payload.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't connect to the server.".to_string()))?;

    let (id, username, password_hash) = match user {
        Some(row) => row,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid username or password.".to_string()))
    };

    let is_valid = verify(&payload.password, &password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Login failed.".to_string()))?;

    if !is_valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid username or password.".to_string()))?;
    }

    Ok((StatusCode::OK, Json(LoginResponse { id, username })))
}