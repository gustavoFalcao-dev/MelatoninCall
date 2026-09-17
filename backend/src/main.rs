mod routes;
mod state;
mod auth;
use state::AppState;
use auth::JwtService;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let jwt = JwtService::from_env()
    .expect("Failed to initialize JWT service");

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is not set on .env.");

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    let state = AppState {
        db: pool,
        jwt     
    };


    let app= routes::build_router().with_state(state);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}