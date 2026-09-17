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
    token: String,
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

    let token = state.jwt
    .create_token(id)
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create authentication token.".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(LoginResponse { 
            token,
        })
    ))
}