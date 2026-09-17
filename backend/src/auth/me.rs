use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use crate::{
    state::AppState,
    auth::user::AuthUser,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct MeResponse {
    id: Uuid,
    username: String,
}

pub async fn me(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MeResponse>, (StatusCode, String)> {
    let user = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, username FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't connect to the server.".to_string()))?;

    let (id, username) = match user {
        Some(row) => row,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                "User not found.".to_string(),
            ));
        }
    };

    Ok(Json(MeResponse { id, username }))
}