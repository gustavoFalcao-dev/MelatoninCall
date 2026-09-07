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

const MIN_CHANNEL_NAME: usize = 4;
const MAX_CHANNEL_NAME: usize = 30;

impl CreateRequest {
    fn create_validated_name(&self) -> Result<String, (StatusCode, String)> {
        let name = self.name.split_whitespace().collect::<Vec<&str>>().join(" ");

        if name.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Channel name cannot be empty.".into()));
        }
        if name.chars().count() < MIN_CHANNEL_NAME {
            return Err((StatusCode::BAD_REQUEST, "Channel name must be at least 4 characters long.".into()));
        }
        if name.len() > MAX_CHANNEL_NAME && name.chars().nth(MAX_CHANNEL_NAME).is_some() {
            return Err((StatusCode::BAD_REQUEST, "Channel name cannot exceed 2000 characters.".into()));
        }

        Ok(name)
    }
}

#[derive(Deserialize)]
pub struct CreateRequest {
    name: String,
    server_id: Uuid
}
#[derive(Serialize)]
pub struct CreateResponse {
    name: String,
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateRequest>
) -> Result<(StatusCode, Json<CreateResponse>), (StatusCode, String)> {

    let name = payload.create_validated_name()?;
    let channel_id = Uuid::now_v7();

    let result = sqlx::query_scalar::<_, String>(
        "INSERT INTO channels (id, name, server_id) VALUES ($1, $2, $3) RETURNING name"
    )
    .bind(channel_id)
    .bind(name)
    .bind(&payload.server_id)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok( name) => Ok((StatusCode::CREATED, Json(CreateResponse { name }))),
        Err(sqlx::Error::Database(db_err)) =>{ 
        if db_err.is_unique_violation() {
            Err((StatusCode::CONFLICT, "A channel with this name already exists in this server.".into()))
        } else if db_err.is_foreign_key_violation() {
            Err((StatusCode::NOT_FOUND, "No server found with the provided ID.".into()))
        } else {
            tracing::error!("Database error creating channel: {:?}", db_err);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create channel.".into()))
        }
        }
        Err(_e) => {
            tracing::error!("Unexpected error creating channel: {:?}", _e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create channel.".into()))
        }
    }
    
}