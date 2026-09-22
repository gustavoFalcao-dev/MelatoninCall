use axum::{
    extract::{
        Path, 
        State,
    },
    http::StatusCode,
    Json
};
use serde::{
    Deserialize,
    Serialize
};
use uuid::Uuid;
use crate::{
    state::AppState,
    auth::user::AuthUser,
};

#[derive(Deserialize)]
pub struct CreateRequest {
    name: String,
}

#[derive(Serialize)]
pub struct CreateResponse {
    name: String,
}

#[derive(Deserialize)]
pub struct UpdateRequest{
    name: Option<String>,
}

const MIN_SERVERS_NAME: usize = 4;
const MAX_SERVERS_NAME: usize = 30;

impl CreateRequest {
    fn validate_name(&self) -> Result<String, (StatusCode, String)> {
        let name = self.name.split_whitespace().collect::<Vec<&str>>().join(" ");

        if name.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST, 
                "Server name cannot be empty.".into()
            ));
        }

        if name.chars().count() < MIN_SERVERS_NAME {
            return Err((
                StatusCode::BAD_REQUEST, 
                format!("Server name should be at least {MIN_SERVERS_NAME} characters long.")
            ));
        }
        
        if name.chars().count() > MAX_SERVERS_NAME {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Server name should be at most {MAX_SERVERS_NAME} characters long.")
            ));
        }

        Ok(name)
    }
}

impl UpdateRequest {
    fn validate(&self) -> Result<Option<String>, (StatusCode, String)> {
        if let Some(name) = &self.name {
            let clean_name = name.trim().to_string();

            if clean_name.is_empty() {
                return Err((StatusCode::BAD_REQUEST, "Server name cannot be empty.".into()));
            }
            if clean_name.chars().count() < MIN_SERVERS_NAME {
                return Err((StatusCode::BAD_REQUEST, format!("Server name must be at least {MIN_SERVERS_NAME} characters long.")));
            }
            if clean_name.len() > MAX_SERVERS_NAME && clean_name.chars().nth(MAX_SERVERS_NAME).is_some() {
                return Err((StatusCode::BAD_REQUEST, format!("Server name cannot exceed {MAX_SERVERS_NAME} characters.")));
            }

            Ok(Some(clean_name))
        } else {
            Ok(None)
        }
    }
}
    

pub async fn create(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateRequest>
) -> Result<(StatusCode, Json<CreateResponse>), (StatusCode, String)> {

    let name = payload.validate_name()?;

    let server_id = Uuid::now_v7();

    let mut tx = state.db
    .begin()
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to start database transaction.".to_string()))?;

    sqlx::query(
        "INSERT INTO servers (id, name, owner_id) VALUES ($1, $2, $3)"
    )
    .bind(server_id)
    .bind(&name)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create server.".to_string()))?;

    sqlx::query(
        "INSERT INTO server_members (server_id, user_id) VALUES ($1, $2)"
    )
    .bind(server_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to add owner to server.".to_string()))?;

    tx.commit()
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to commit server creation.".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateResponse { name }),
    ))
}



pub async fn update(
    State(state): State<AppState>,
    Path(server_id_str): Path<String>,
    Json(payload): Json<UpdateRequest>
) -> Result<StatusCode, (StatusCode, String)> {

    let server_id = Uuid::parse_str(&server_id_str)
    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Server ID format. Must be a valid UUID.".into()))?;

    let validated_name = payload.validate()?;

    if validated_name.is_none() {
        return Ok(StatusCode::OK);
    }

    let result = sqlx::query(
        "UPDATE servers SET name = COALESCE($1, name) WHERE id = $2"
    )
    .bind(validated_name) 
    .bind(server_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((StatusCode::NOT_FOUND, "Server not found.".into()))
            } else {
                Ok(StatusCode::OK)
            }
        }
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.is_unique_violation() {
                Err((StatusCode::CONFLICT, "A Server with this name already exists in this server.".into()))
            } else {
                tracing::error!("Database error updating server: {:?}", db_err);
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update server.".into()))
            }
        }
        Err(e) => {
            tracing::error!("Unexpected error updating server: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update server.".into()))
        }
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(server_id_str): Path<String>
) -> Result<StatusCode, (StatusCode, String)> {

    let server_id = Uuid::parse_str(&server_id_str)
    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Server ID format. Must be a valid UUID.".into()))?;
    
    let result = sqlx::query("DELETE from servers WHERE id = $1")
    .bind(server_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(db_result) => {
            if db_result.rows_affected() == 0 {
                Err((StatusCode::NOT_FOUND, "Server not found".into()))
            } else {
                Ok(StatusCode::NO_CONTENT)
            }
        }
        Err(_e) => {
            tracing::error!("Unexpected error deleting Server: {:?}", _e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete server.".into()))
        }
    }

}