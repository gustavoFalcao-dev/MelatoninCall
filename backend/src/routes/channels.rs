use axum::{
    extract::{Path, State},
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
pub struct CreateRequest {
    name: String,
    server_id: String
}

#[derive(Serialize)]
pub struct CreateResponse {
    name: String,
}

pub struct ValidatedSendRequest{
    name: String,
    server_id: Uuid
}

#[derive(Deserialize)]
pub struct UpdateRequest{
    name: Option<String>,
}

const MIN_CHANNEL_NAME: usize = 4;
const MAX_CHANNEL_NAME: usize = 30;

impl CreateRequest {
    fn create_validated_name(&self) -> Result<ValidatedSendRequest, (StatusCode, String)> {
        let name = self.name.split_whitespace().collect::<Vec<&str>>().join(" ");

        if name.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Channel name cannot be empty.".into()));
        }
        if name.chars().count() < MIN_CHANNEL_NAME {
            return Err((StatusCode::BAD_REQUEST, format!("Content cannot exceed {MIN_CHANNEL_NAME} characters.")));
        }
        if name.len() > MAX_CHANNEL_NAME && name.chars().nth(MAX_CHANNEL_NAME).is_some() {
            return Err((StatusCode::BAD_REQUEST, format!("Content cannot exceed {MAX_CHANNEL_NAME} characters.")));
        }

        /* Validate server_id */
        if self.server_id.trim().is_empty(){
            return Err((StatusCode::BAD_REQUEST, "Server ID is required.".into()));
        }

        let server_id = Uuid::parse_str(&self.server_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Server ID format. Must be a valid UUID.".into()))?;

        Ok(ValidatedSendRequest {
            name,
            server_id
        })
    }
}

impl UpdateRequest {
    fn validate(&self) -> Result<Option<String>, (StatusCode, String)> {
        if let Some(name) = &self.name {
            let clean_name = name.trim().to_string();

            if clean_name.is_empty() {
                return Err((StatusCode::BAD_REQUEST, "Channel name cannot be empty.".into()));
            }
            if clean_name.chars().count() < MIN_CHANNEL_NAME {
                return Err((StatusCode::BAD_REQUEST, format!("Channel name must be at least {MIN_CHANNEL_NAME} characters long.")));
            }
            if clean_name.len() > MAX_CHANNEL_NAME && clean_name.chars().nth(MAX_CHANNEL_NAME).is_some() {
                return Err((StatusCode::BAD_REQUEST, format!("Channel name cannot exceed {MAX_CHANNEL_NAME} characters.")));
            }

            Ok(Some(clean_name))
        } else {
            Ok(None)
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateRequest>
) -> Result<(StatusCode, Json<CreateResponse>), (StatusCode, String)> {

    let req = payload.create_validated_name()?;
    let channel_id = Uuid::now_v7();

    let result = sqlx::query_scalar::<_, String>(
        "INSERT INTO channels (id, name, server_id) VALUES ($1, $2, $3) RETURNING name"
    )
    .bind(channel_id)
    .bind(req.name)
    .bind(req.server_id)
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

pub async fn update(
    State(state): State<AppState>,
    Path(channel_id_str): Path<String>,
    Json(payload): Json<UpdateRequest>
) -> Result<StatusCode, (StatusCode, String)> {

    let channel_id = Uuid::parse_str(&channel_id_str)
    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Channel ID format. Must be a valid UUID.".into()))?;

    let validated_name = payload.validate()?;

    if validated_name.is_none() {
        return Ok(StatusCode::OK);
    }

    let result = sqlx::query(
        "UPDATE channels SET name = COALESCE($1, name) WHERE id = $2"
    )
    .bind(validated_name) 
    .bind(channel_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((StatusCode::NOT_FOUND, "Channel not found.".into()))
            } else {
                Ok(StatusCode::OK)
            }
        }
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.is_unique_violation() {
                Err((StatusCode::CONFLICT, "A channel with this name already exists in this server.".into()))
            } else {
                tracing::error!("Database error updating channel: {:?}", db_err);
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update channel.".into()))
            }
        }
        Err(e) => {
            tracing::error!("Unexpected error updating channel: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update channel.".into()))
        }
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(channel_id_str): Path<String>
) -> Result<StatusCode, (StatusCode, String)> {

    let channel_id = Uuid::parse_str(&channel_id_str)
    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Channel ID format. Must be a valid UUID.".into()))?;
    
    let result = sqlx::query("DELETE from channels WHERE id = $1")
    .bind(channel_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((StatusCode::NOT_FOUND, "Channel not found".into()))
            } else {
                Ok(StatusCode::NO_CONTENT)
            }
        }
        Err(_e) => {
            tracing::error!("Unexpected error deleting channel: {:?}", _e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete channel.".into()))
        }
    }

}