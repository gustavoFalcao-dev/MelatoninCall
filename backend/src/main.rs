// Imports
use axum::{ // Axum HTTP Request handling
    routing::get,
    Router
};
use sqlx::PgPool; // SQLX Postgre connection pool
// use std::sync::Arc;


// Making conection pool usable for multiple handlers simultaneously
#[derive(Clone)]
struct Appstate {
    db: PgPool
}

// Creating asynchronous main function with tokio, since on Rust the main can't be async
#[tokio::main]
async fn main() {
    // Load variables from .env into the environment
    dotenvy::dotenv().ok();
    // Using tracing for logs
    tracing_subscriber::fmt::init();

// Database
    // Set database_url
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env.");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");
    let state = Appstate {
        db: pool
    };

    // Creating a router to especify which route should handle each request
    let app = Router::new()
    .route("/", get(|| async { "Hello, World!" }))
    // Route for testing DB state
    .route("/health", get(health_check))
    .with_state(state);

    // Creating a listener for TCP requests on localhost port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    // Diplays on CLI the address that is being listenned
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    // Makes the bridge so listener can work with app
    axum::serve(listener, app).await.unwrap();
}

// Function to check DBs connected properly
async fn health_check(
    // Extracts the state and makes "state" it's owner
    axum::extract::State(state): axum::extract::State<Appstate>,
    // The answer should be a static sliced string
) -> &'static str {
    // Sends a basic query
    match sqlx::query("SELECT 1")
    // Gets one row of DBs state
        .fetch_one(&state.db)
        .await
        {
            // Matches with one of the 2 possible answers
            Ok(_) => "OK - database connected.",
            // If it fails display the error
            Err(error) => {
                println!("Database error: {:?}", error);
                "ERROR - database unreachable."
            }
        }
}