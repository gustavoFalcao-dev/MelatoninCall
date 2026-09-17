mod health;
mod users;
mod servers;
mod channels;
mod messages;

use axum::{
    Router, routing::{
        delete, get, patch, post
    }
};
use tower_http::cors::{
    CorsLayer,
    Any
};
use crate::{
    state::AppState,
    auth,
};

pub fn build_router() -> Router<AppState> {
    let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

    Router::new()
        .route("/", get(|| async{"Hello, World"}))
        .route("/health", get(health::health_check))
        .route("/users/register", post(users::register))
        .route("/servers/create", post(servers::create))
        .route("/channel/create", post(channels::create))
        .route("/channel/update/{id}", patch(channels::update))
        .route("/channel/delete/{id}", delete(channels::delete))
        .route("/messages/send", post(messages::send))
        .route("/auth/login", post(auth::login::login))
        .route("/auth/me", get(auth::me::me))
        .layer(cors)
    }