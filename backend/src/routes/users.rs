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
use bcrypt::{
    hash,
    DEFAULT_COST
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
    email: String
}
#[derive(Serialize)]
pub struct RegisterResponse {
    id: Uuid,
    username: String,
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


    if !payload.email.contains('@') {
        return Err ((
            StatusCode::BAD_REQUEST,
            "The e-mail inserted is invalid.".to_string()
        ))
    }
    if payload.email.trim().is_empty() {
        return Err ((
            StatusCode::BAD_REQUEST,
            "E-mail required.".to_string()
        ))
    }

    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed.".to_string()))?;

    let result = sqlx::query_as::<_, (Uuid, String)>(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username"
    )
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(&password_hash)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, username)) => Ok((StatusCode::CREATED, Json(RegisterResponse { id, username}))),
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