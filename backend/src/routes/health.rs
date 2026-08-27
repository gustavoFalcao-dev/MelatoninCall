use axum::extract::State;
use crate::state::AppState;

pub async fn health_check(State(state): State<AppState>) -> &'static str {
    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => "OK - database connected.",
        Err(error) => {
            println!("Database error: {:?}", error);
            "ERROR - database couldn't be reached."
        }
    }
}