mod routes;
mod state;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is not set on .env.");
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");
    let state = AppState {
        db: pool
    };


    let app= routes::build_router().with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}