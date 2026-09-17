use axum::{
    extract::FromRequestParts, 
    http::{StatusCode, request::Parts},
};
use uuid::Uuid;
use crate::state::AppState;
// use async_trait::async_trait;

pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection>
    {
        let authorization = parts
        .headers
        .get("authorization")
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing authorization header.".to_string(),
        ))?;

        let authorization = authorization
        .to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid authorization header.".to_string()))?;

        let token = authorization
        .strip_prefix("Bearer ")
        .ok_or((
            StatusCode::UNAUTHORIZED, "Invalid authorization scheme.".to_string()
        ))?;

        let claims = state.jwt
        .validate_token(token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token.".to_string()))?;

        let user_id = claims.sub;

        Ok(AuthUser(user_id))
        
    }
}