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
use crate::{
    auth::user::AuthUser, state::AppState,
};
use uuid::Uuid;
use sqlx::FromRow;


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

#[derive(Serialize, FromRow)]
pub struct ServerResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

const MIN_SERVER_NAME: usize = 4;
const MAX_SERVER_NAME: usize = 30;

impl CreateRequest {
    fn validate_name(&self) -> Result<String, (StatusCode, String)> {
        let name = self.name.split_whitespace().collect::<Vec<&str>>().join(" ");

        if name.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST, 
                "Server name cannot be empty.".into()
            ));
        }

        if name.chars().count() < MIN_SERVER_NAME {
            return Err((
                StatusCode::BAD_REQUEST, 
                format!("Server name should be at least {MIN_SERVER_NAME} characters long.")
            ));
        }
        
        if name.chars().count() > MAX_SERVER_NAME {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Server name should be at most {MAX_SERVER_NAME} characters long.")
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
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Server name cannot be empty.".into()));
            }
            if clean_name.chars().count() < MIN_SERVER_NAME {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Server name must be at least {MIN_SERVER_NAME} characters long.")));
            }
            if clean_name.chars().count() > MAX_SERVER_NAME {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Server name cannot exceed {MAX_SERVER_NAME} characters.")));
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
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(server_id_str): Path<String>,
    Json(payload): Json<UpdateRequest>
) -> Result<StatusCode, (StatusCode, String)> {

    let server_id = Uuid::parse_str(&server_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        "Invalid Server ID format. Must be a valid UUID.".into()
    ))?;

    let validated_name = payload.validate()?;

    if validated_name.is_none() {
        return Ok(StatusCode::OK);
    }

    let result = sqlx::query(
        "UPDATE servers SET name = $1 WHERE id = $2 AND owner_id = $3"
    )
    .bind(validated_name) 
    .bind(server_id)
    .bind(user_id)
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
                tracing::error!("Database error updating server: {:?}", db_err);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to update server.".into()
                ))
            // }
        }
        Err(e) => {
            tracing::error!("Unexpected error updating server: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update server.".into()
            ))
        }
    }
}

pub async fn delete(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(server_id_str): Path<String>
) -> Result<StatusCode, (StatusCode, String)> {

    let server_id = Uuid::parse_str(&server_id_str)
    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Server ID format. Must be a valid UUID.".into()))?;
    
    let result = sqlx::query("DELETE FROM servers WHERE id = $1 AND owner_id = $2")
    .bind(server_id)
    .bind(user_id)
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

pub async fn list(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<ServerResponse>>, (StatusCode, String)> {
    
    let servers = sqlx::query_as::<_, ServerResponse>(
        "SELECT s.id, s.name, s.owner_id, s.created_at FROM servers s JOIN server_members sm ON sm.server_id = s.id WHERE sm.user_id = $1 ORDER BY s.created_at"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to fetch servers.".to_string()
    ))?;

    Ok(Json(servers))

}

pub async fn get(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(server_id_str): Path<String>,
) -> Result<Json<ServerResponse>, (StatusCode, String)> {

    let server_id = Uuid::parse_str(&server_id_str)
    .map_err(|_| (
        StatusCode::BAD_REQUEST,
        "Invalid Server ID format. Must be a valid UUID.".into()
    ))?;

    let server = sqlx::query_as::<_, ServerResponse>(
        "SELECT s.id, s.name, s.owner_id, s.created_at FROM servers s JOIN server_members sm ON sm.server_id = s.id WHERE s.id = $1 AND sm.user_id = $2"
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to fetch server".into()
    ))?;

    match server {
        Some(server) => Ok(Json(server)),
        None => Err((
            StatusCode::NOT_FOUND,
            "Server not found.".into(),
        ))
    }

}