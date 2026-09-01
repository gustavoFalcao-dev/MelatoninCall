mod health;
mod users;

use axum::{
    routing::{
        get, post
    },
    Router
};
use crate::state::AppState;

pub fn build_router() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async{"Hello, World"}))
        .route("/health", get(health::health_check))
        .route("/users/register", post(users::register))
        .route("/users/login", post(users::login))
}