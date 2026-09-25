use axum::{
    extract::{Path, State,Query},
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
pub struct PrivateMessagePaginatedResponse<T> {
    data: Vec<T>,
    page: u32,
    per_page: u8,
    total_items: i64,
    total_pages: i64,
}

#[derive(Serialize, FromRow)]
pub struct ListResponse {
    id: Uuid,
    content: String,
}

#[derive(Deserialize)]
pub struct SendRequest {
    content: String,
}

#[derive(Serialize)]
pub struct SendResponse {
    content: String,
}

pub struct ValidatedSendRequest{
     content: String,
}

#[derive(Deserialize)]
pub struct UpdateRequest{
    content: Option<String>,
}

const MAX_MESSAGE_CONTENT: usize = 2000;

impl SendRequest {
    fn validate_send(&self) -> Result<ValidatedSendRequest, (StatusCode, String)> {

        /* Validate content */
        let content = self.content.trim().to_string();

        if content.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Content cannot be empty.")));
        }
        if content.len() > MAX_MESSAGE_CONTENT && content.chars().nth(MAX_MESSAGE_CONTENT).is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Content cannot exceed {MAX_MESSAGE_CONTENT} characters.")
            ));
        }

        Ok(ValidatedSendRequest {
            content,
        })
    }

}

impl UpdateRequest {
    fn validate(&self) -> Result<Option<String>, (StatusCode, String)> {
        if let Some(content) = &self.content {
        
        /* Validate content */
        let content = content.trim().to_string();

        if content.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Content cannot be empty.")));
        }
        if content.len() > MAX_MESSAGE_CONTENT && content.chars().nth(MAX_MESSAGE_CONTENT).is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Content cannot exceed {MAX_MESSAGE_CONTENT} characters.")
            ));
        }

            Ok(Some(content))
        } else {
            Ok(None)
        }
    }
}

pub async fn list(
    AuthUser(author_id): AuthUser,
    State(state): State<AppState>,
    Path(receiver_id_str): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PrivateMessagePaginatedResponse<ListResponse>>, (StatusCode, String)> {

    let receiver_id = Uuid::parse_str(&receiver_id_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, 
        format!("Invalid Receiver ID format. Must be a valid UUID.")))?;

    let receiver_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM private_messages WHERE receiver_id = $1 AND author_id = $2)"
    )
    .bind(receiver_id)
    .bind(author_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking receiver existence: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR,
         format!("Failed to verify receiver existence."))
    })?;

    if !receiver_exists {
        return Err((StatusCode::NOT_FOUND,
            format!("Private message not found.")));
    }

    let page = pagination.page.unwrap_or(1).max(1);
    let per_page = pagination.per_page.unwrap_or(10).clamp(1, 50);
    let offset = ((page - 1) as i64) * (per_page as i64);

    let total_items: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM private_messages WHERE receiver_id = $1 AND author_id = $2"
    )
    .bind(receiver_id)
    .bind(author_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error counting private messages: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, 
        format!("Failed to fetch private messages count."))
    })?;

    let result = sqlx::query_as::<_, ListResponse>(
        "SELECT id, author_id, receiver_id, content FROM private_messages 
            WHERE (author_id = $1 AND receiver_id = $2) OR (author_id = $2 AND receiver_id = $1) 
            ORDER BY id DESC LIMIT $3 OFFSET $4"
    )
    .bind(author_id)
    .bind(receiver_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Database error listing private messages: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, 
        format!("Failed to fetch private messages."))
    })?;

    let total_pages = (total_items as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PrivateMessagePaginatedResponse {
            data: result,
            page,
            per_page,
            total_items,
            total_pages,
        }))
}

pub async fn send(
    AuthUser(author_id): AuthUser,
    State(state): State<AppState>,
    Path(receiver_id_str): Path<String>,
    Json(payload): Json<SendRequest>
) -> Result<(StatusCode, Json<SendResponse>), (StatusCode, String)> {

    let receiver_id = Uuid::parse_str(&receiver_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        format!("Invalid Receiver ID format. Must be a valid UUID.")
    ))?;

    let req = payload.validate_send()?;
    let message_id = Uuid::now_v7();

    let result = sqlx::query_scalar::<_, String>(
        "INSERT INTO private_messages (id, author_id, receiver_id, content) VALUES ($1, $2, $3, $4) RETURNING content"
    )
    .bind(message_id)
    .bind(author_id)
    .bind(receiver_id)
    .bind(req.content)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok( content) => Ok((StatusCode::CREATED, Json(SendResponse { content }))),
        Err(sqlx::Error::Database(db_err)) =>{ 
        if db_err.is_foreign_key_violation() {
            Err((
                StatusCode::NOT_FOUND,
                format!("The specified receiver does not exist.")
            ))
        } else {
            tracing::error!("Database error creating message: {:?}", db_err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to send message.")
            ))
        }
        }
        Err(_e) => {
            tracing::error!("Unexpected error sending message: {:?}", _e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to send message.")
            ))
        }
    }
    
}

pub async fn update(
    AuthUser(author_id): AuthUser,
    State(state): State<AppState>,
    Path(private_message_id_str): Path<String>,
    Json(payload): Json<UpdateRequest>
) -> Result<StatusCode, (StatusCode, String)> {

    let private_message_id = Uuid::parse_str(&private_message_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        format!("Invalid Channel ID format. Must be a valid UUID.")
    ))?;

    let req = payload.validate()?;

    if req.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }

    let result = sqlx::query(
        "UPDATE private_messages SET content = $1, updated_at = NOW() WHERE id = $2 AND author_id = $3"
    )
    .bind(req) 
    .bind(private_message_id)
    .bind(author_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("Message not found.")
                ))
            } else {
                Ok(StatusCode::NO_CONTENT)
            }
        }
        Err(sqlx::Error::Database(db_err)) => {
                tracing::error!("Database error updating private message: {:?}", db_err);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update message.")
                ))
        }
        Err(e) => {
            tracing::error!("Unexpected error updating private message: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update message.")
            ))
        }
    }
}

pub async fn delete(
    AuthUser(author_id): AuthUser,
    State(state): State<AppState>,
    Path(private_message_id_str): Path<String>
) -> Result<StatusCode, (StatusCode, String)> {

    let private_message_id = Uuid::parse_str(&private_message_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        format!("Invalid Channel ID format. Must be a valid UUID.")
    ))?;
    
    let result = sqlx::query(
    "DELETE from private_messages WHERE id = $1 AND author_id = $2")
    .bind(private_message_id)
    .bind(author_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("Message not found")
                ))
            } else {
                Ok(StatusCode::NO_CONTENT)
            }
        }
        Err(e) => {
            tracing::error!("Unexpected error deleting channel: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete Message.")
            ))
        }
    }
}