use sqlx::PgPool;
use crate::auth::JwtService;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt: JwtService,
}