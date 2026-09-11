use axum::{
    extract::State,
    http::StatusCode,
    Json
};
use serde::{
    Deserialize,
    Serialize
};
use bcrypt::{
    hash,
    verify,
    DEFAULT_COST
};
use uuid::Uuid;
use crate::state::AppState;
use email_address::EmailAddress;

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
    email: String
}
#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String
}
#[derive(Serialize)]
pub struct RegisterResponse {
    id: Uuid,
    username: String,
}
#[derive(Serialize)]
pub struct LoginResponse {
    id: Uuid,
    username: String
}

const MIN_USERNAME_LEN: usize = 4;
const MIN_PASSWORD_LEN: usize = 8;
const MAX_USERNAME_LEN: usize = 30;
const MAX_PASSWORD_LEN: usize = 30;

impl RegisterRequest {
    fn validate_username(&self) -> Result<&str, (StatusCode, String)> {
        /*
        Length between 4 and 30
        ASCII alphanumeric with _ and -
        Can't start or end with punctuation
        Can't contain a sequence of punctuations
        */
        let username = &self.username;

        if username.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Username field cannot be empty.".into()
            ))
        }
        
        if username.len() < MIN_USERNAME_LEN {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Username should be at least {MIN_USERNAME_LEN} characters long.")
            ))
        }

        if username.len() > MAX_USERNAME_LEN {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Username should be at most {MAX_USERNAME_LEN} characters long")
            ))
        }

        if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err((
                StatusCode::BAD_REQUEST,
                "Username should only contain letters, numbers, underscores and hyphens.".into()
            ))
        }

        if !username.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Username needs to start with a letter or a number.".into()
            ))
        }

        if !username.chars().last().is_some_and(|c| c.is_ascii_alphanumeric()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Username needs to end with a letter or a number.".into()
            ))
        }

        if username.as_bytes().windows(2).any(|w| matches!(w[0], b'_' | b'-') && matches!(w[1], b'_' | b'-')) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Username can't contain consecutive hyphens and/or underscores.".into()
            ))
        }

        Ok(username)
    }

    fn validate_email(&self) -> Result<&str, (StatusCode, String)>{
        /*
        Call the crate for default email handling
        */
        let email = &self.email;

        if email.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Email field cannot be empty.".into()
            ))
        }
        
        if !EmailAddress::is_valid(email) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Please enter a valid email.".into()
            ))
        }
        
        Ok(email)
    }

    fn validate_password(&self) -> Result<&str, (StatusCode, String)> {
        /*
        Length between 8 and 30
        ASCII
        At least 1 uppercase and 1 lowercase
        At least one special char with the exception of whitespaces
        */
        let password = &self.password;

        if password.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password field cannot be empty.".into()
            ))
        }
        
        if password.len() < MIN_PASSWORD_LEN {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Password should be at least {MIN_PASSWORD_LEN} characters long.")
            ))
        }

        if password.len() > MAX_PASSWORD_LEN {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Password should be at most {MAX_PASSWORD_LEN} characters long")
            ))
        }

        if !password.is_ascii() {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password should only contain ASCII characters.".into()
            ))
        }

        if !password.chars().any(|c| c.is_ascii_uppercase()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password should contain at least one uppercase letter.".into()
            ))
        }

        if !password.chars().any(|c| c.is_ascii_lowercase()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password should contain at least one lowercase letter.".into()
            ))
        }

        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password should contain at least one digit.".into()
            ))
        }

        if !password.chars().any(|c| c.is_ascii_punctuation()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password should contain at least one special character.".into()
            ))
        }

        if password.chars().any(|c| c.is_whitespace()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password can't contain whitespaces.".into()
            ))
        }

        Ok(password)
    }
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
    
    let username = payload.validate_username()?;

    let email = payload.validate_email()?;

    let password = payload.validate_password()?;

    let password_hash = hash(password, DEFAULT_COST)
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed.".to_string()))?;
    
    let id = Uuid::now_v7();

    let result = sqlx::query_as::<_, (Uuid, String)>(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING id, username"
    )
    .bind(id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, username)) => Ok((StatusCode::CREATED, Json(RegisterResponse { id, username }))),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            let constraint = db_err.constraint().unwrap_or("");
            let msg = if constraint.contains("email") {
                "Email already registered."
            } else {
                "Username already taken."
            };
            Err((StatusCode::CONFLICT,
                msg.to_string()
            ))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Registration failed.".to_string()
        ))
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)>{
    let user = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, username, password_hash FROM users WHERE username = $1"
    )
    .bind(&payload.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't connect to the server.".to_string()))?;

    let (id, username, password_hash) = match user {
        Some(row) => row,
        None => return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password.".to_string()
        ))
    };

    let is_valid = verify(&payload.password, &password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Login failed.".to_string()))?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password.".to_string()
        ));
    }

    Ok((StatusCode::OK, Json(LoginResponse { id, username })))
}