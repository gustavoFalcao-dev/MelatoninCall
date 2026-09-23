use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json
};
use serde::{
    Deserialize,
    Serialize
};

use sqlx::prelude::FromRow;
use uuid::Uuid;
use crate::{
    auth::user::AuthUser, 
    state::AppState,
};

#[derive(Deserialize)]
pub struct PaginationParams{
    page: Option<u32>,
    per_page: Option<u8>
}

#[derive(Serialize)]
pub struct ChannelPaginatedResponse<T> {
    data: Vec<T>,
    page: u32,
    per_page: u8,
    total_items: i64,
    total_pages: i64,
}

#[derive(Serialize, FromRow)]
pub struct ChannelResponse {
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
pub struct CreateRequest {
    name: String,
    server_id: String
}

#[derive(Serialize)]
pub struct CreateResponse {
    name: String,
}

pub struct ValidatedCreateRequest{
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
    fn validate_name(&self) -> Result<ValidatedCreateRequest, (StatusCode, String)> {
        let name = self.name.split_whitespace().collect::<Vec<&str>>().join(" ");

        if name.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Channel name cannot be empty.".into()
            ));
        }
        if name.chars().count() < MIN_CHANNEL_NAME {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Channel name must be at least {MIN_CHANNEL_NAME} characters long.")
            ));
        }
        if name.chars().count() > MAX_CHANNEL_NAME {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Channel name must be at most {MAX_CHANNEL_NAME} characters long.")
            ));
        }

        /* Validate server_id */
        if self.server_id.trim().is_empty(){
            return Err((
                StatusCode::BAD_REQUEST,
                "Server ID is required.".into()
            ));
        }

        let server_id = Uuid::parse_str(&self.server_id)
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            "Invalid Server ID format. Must be a valid UUID.".into()
        ))?;

        Ok(ValidatedCreateRequest {
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
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Channel name cannot be empty.".into()
                ));
            }
            if clean_name.chars().count() < MIN_CHANNEL_NAME {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Channel name must be at least {MIN_CHANNEL_NAME} characters long.")
                ));
            }
            if clean_name.chars().count() > MAX_CHANNEL_NAME {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Channel name must be at most {MAX_CHANNEL_NAME} characters long.")
                ));
            }

            Ok(Some(clean_name))
        } else {
            Ok(None)
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    Path(server_id_str): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ChannelPaginatedResponse<ChannelResponse>>, (StatusCode, String)> {

    let server_id = Uuid::parse_str(&server_id_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Server ID format. Must be a valid UUID.".into()))?;

    let server_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM servers WHERE id = $1)"
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking server existence: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR,
         "Failed to verify server existence.".into())
    })?;

    if !server_exists {
        return Err((StatusCode::NOT_FOUND,
             "Server not found.".into()));
    }

    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(10).clamp(1, 50);
    let offset = ((page - 1) as i64) * (per_page as i64);

    let total_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM channels WHERE server_id = $1"
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error counting channels: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, 
        "Failed to fetch channels count.".into())
    })?;

    let result = sqlx::query_as::<_, ChannelResponse>(
        "SELECT id, name, server_id FROM channels WHERE server_id = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
    )
    .bind(server_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error listing channels: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, 
        "Failed to fetch channels.".into())
    })?;

    let total_pages = (total_items as f64 / per_page as f64).ceil() as i64;

    Ok(Json(ChannelPaginatedResponse {
            data: result,
            page,
            per_page,
            total_items,
            total_pages,
        }))
}

pub async fn create(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateRequest>
) -> Result<(StatusCode, Json<CreateResponse>), (StatusCode, String)> {

    let req = payload.validate_name()?;

    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS( SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2)"
    )
    .bind(req.server_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to verify server membership.".into()
    ))?;

    if !is_member {
        return Err((
            StatusCode::NOT_FOUND,
            "Server not found.".into()
        ))
    }

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
            Err((
                StatusCode::CONFLICT,
                "A channel with this name already exists in this server.".into()
            ))
        } else if db_err.is_foreign_key_violation() {
            Err((
                StatusCode::NOT_FOUND,
                "No server found with the provided ID.".into()
            ))
        } else {
            tracing::error!("Database error creating channel: {:?}", db_err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create channel.".into()
            ))
        }
        }
        Err(_e) => {
            tracing::error!("Unexpected error creating channel: {:?}", _e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create channel.".into()
            ))
        }
    }
    
}

pub async fn update(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(channel_id_str): Path<String>,
    Json(payload): Json<UpdateRequest>
) -> Result<StatusCode, (StatusCode, String)> {

    let channel_id = Uuid::parse_str(&channel_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        "Invalid Channel ID format. Must be a valid UUID.".into()
    ))?;

    let validated_name = payload.validate()?;

    if validated_name.is_none() {
        return Ok(StatusCode::OK);
    }

    let result = sqlx::query(
        "UPDATE channels SET name = $1 WHERE id = $2 AND server_id IN ( SELECT server_id FROM server_members WHERE user_id = $3 )"
    )
    .bind(validated_name) 
    .bind(channel_id)
    .bind(user_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((
                    StatusCode::NOT_FOUND,
                    "Channel not found.".into()
                ))
            } else {
                Ok(StatusCode::OK)
            }
        }
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.is_unique_violation() {
                Err((
                    StatusCode::CONFLICT,
                    "A channel with this name already exists in this server.".into()
                ))
            } else {
                tracing::error!("Database error updating channel: {:?}", db_err);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update channel.".into()
                ))
            }
        }
        Err(e) => {
            tracing::error!("Unexpected error updating channel: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update channel.".into()
            ))
        }
    }
}

pub async fn delete(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(channel_id_str): Path<String>
) -> Result<StatusCode, (StatusCode, String)> {

    let channel_id = Uuid::parse_str(&channel_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        "Invalid Channel ID format. Must be a valid UUID.".into()
    ))?;
    
    let result = sqlx::query("DELETE from channels WHERE id = $1 AND server_id IN ( SELECT server_id FROM server_members WHERE user_id = $2 )")
    .bind(channel_id)
    .bind(user_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((
                    StatusCode::NOT_FOUND,
                    "Channel not found".into()
                ))
            } else {
                Ok(StatusCode::NO_CONTENT)
            }
        }
        Err(e) => {
            tracing::error!("Unexpected error deleting channel: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete channel.".into()
            ))
        }
    }
}