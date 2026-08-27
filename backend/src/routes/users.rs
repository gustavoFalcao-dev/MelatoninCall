use axum::{
    extract::State,
    http::StatusCode,
    Json
};
use serde::{
    Deserialize,
    Serialize
};
use uuid::Uuid;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String
}
#[derive(Serialize)]
pub struct RegisterResponse {
    id: Uuid,
    username: String
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
    if payload.username.trim().is_empty() {
        return Err ((
            StatusCode::BAD_REQUEST,
            "Username required.".to_string()
        ))
    }
    if payload.password.len() < 8 {
        return Err ((
            StatusCode::BAD_REQUEST,
            "Password should be at least 8 characters.".to_string()
        ))
    }

    let result = sqlx::query_as::<_, (Uuid, String)>(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id, username"
    )
    .bind(&payload.username)
    .bind(&payload.password)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, username)) => Ok((StatusCode::CREATED, Json(RegisterResponse { id, username}))),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err((StatusCode::CONFLICT, "Username already taken.".to_string()))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Registration failed.".to_string()))
    }
}