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
pub struct SendRequest {
    content: String,
    author_id: String,
    channel_id: String,
}

#[derive(Serialize)]
pub struct SendResponse {
    content: String,
}

pub struct ValidatedSendRequest{
     content: String,
     author_id: Uuid,
     channel_id: Uuid,
}

const MAX_MESSAGE_CONTENT: usize = 2000;

impl SendRequest {
    fn send_validate(&self) -> Result<ValidatedSendRequest, (StatusCode, String)> {

        /* Validate content */
        let content = self.content.trim().to_string();

        if content.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Content cannot be empty.".into()));
        }
        if content.len() > MAX_MESSAGE_CONTENT && content.chars().nth(MAX_MESSAGE_CONTENT).is_some() {
            return Err((StatusCode::BAD_REQUEST, format!("Content cannot exceed {MAX_MESSAGE_CONTENT} characters.")));
        }

        /* Validate author_id */
        if self.author_id.trim().is_empty(){
            return Err((StatusCode::BAD_REQUEST, "Author ID is required.".into()));
        }

        let author_id = Uuid::parse_str(&self.author_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Author ID format. Must be a valid UUID.".into()))?;

        
        /* Validate channel_id */
        if self.channel_id.trim().is_empty(){
            return Err((StatusCode::BAD_REQUEST, "Channel ID is required.".into()));
        }

        let channel_id = Uuid::parse_str(&self.channel_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Channel ID format. Must be a valid UUID.".into()))?;

        Ok(ValidatedSendRequest {
            content, 
            author_id, 
            channel_id
        })
    }

}

pub async fn send(
    State(state): State<AppState>,
    Json(payload): Json<SendRequest>
) -> Result<(StatusCode, Json<SendResponse>), (StatusCode, String)> {

    let req = payload.send_validate()?;
    let message_id = Uuid::now_v7();

    let result = sqlx::query_scalar::<_, String>(
        "INSERT INTO messages (id, channel_id, author_id, content) VALUES ($1, $2, $3, $4) RETURNING content"
    )
    .bind(message_id)
    .bind(req.channel_id)
    .bind(req.author_id)
    .bind(req.content)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok( content) => Ok((StatusCode::CREATED, Json(SendResponse { content }))),
        Err(sqlx::Error::Database(db_err)) =>{ 
        if db_err.is_foreign_key_violation() {
            Err((StatusCode::NOT_FOUND, "The specified channel or author does not exist.".into()))
        } else {
            tracing::error!("Database error creating message: {:?}", db_err);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to send message.".into()))
        }
        }
        Err(_e) => {
            tracing::error!("Unexpected error sending message: {:?}", _e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to send message.".into()))
        }
    }
    
}