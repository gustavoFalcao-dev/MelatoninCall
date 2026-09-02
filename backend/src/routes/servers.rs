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

const MIN_SERVERS_NAME: usize = 4;
const MAX_SERVERS_NAME: usize = 30;

impl CreateRequest {
    fn create_validated_name(&self) -> Result<String, (StatusCode, String)> {
        let name = self.name.split_whitespace().collect::<Vec<&str>>().join(" ");

        if name.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "Server name cannot be empty.".to_string()));
        }
        if name.chars().count() < MIN_SERVERS_NAME {
            return Err((StatusCode::BAD_REQUEST, "Server name must be at least 4 characters long.".to_string()));
        }
        if name.len() > MAX_SERVERS_NAME && name.chars().nth(MAX_SERVERS_NAME).is_some() {
            return Err((StatusCode::BAD_REQUEST, "Server name cannot exceed 30 characters.".to_string()));
        }

        Ok(name)
    }
}

#[derive(Deserialize)]
pub struct CreateRequest {
    name: String,
    owner_id: Uuid
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

    let server_id = Uuid::now_v7();

    let result = sqlx::query_scalar::<_, String>(
        "INSERT INTO servers (id, name, owner_id) VALUES ($1, $2, $3) RETURNING name"
    )
    .bind(server_id)
    .bind(name)
    .bind(&payload.owner_id)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok( name) => Ok((StatusCode::CREATED, Json(CreateResponse { name }))),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            let constraint = db_err.constraint().unwrap_or("");
            Err((StatusCode::CONFLICT, constraint.to_string()))
        }
        Err(_) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create server.".to_string()))
        }
    }
}