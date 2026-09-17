use jsonwebtoken::{
    DecodingKey,
    EncodingKey
};
use serde::{
    Deserialize,
    Serialize
};
use std::time::{
    SystemTime,
    UNIX_EPOCH
};
use uuid::Uuid;

#[derive(Clone)]
pub struct JwtService {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub access_ttl: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims{
    pub sub: Uuid,
    pub exp: usize
}

#[derive(Debug)]
pub enum JwtError {
    MissingEnvVar(String),
    ReadPrivateKey(std::io::Error),
    ReadPublicKey(std::io::Error),
    InvalidPrivateKey(jsonwebtoken::errors::Error),
    InvalidPublicKey(jsonwebtoken::errors::Error),
    InvalidExpirationTime(String),
    CreateToken(jsonwebtoken::errors::Error),
    ValidateToken(jsonwebtoken::errors::Error),
}

impl JwtService {
    pub fn from_env() -> Result<Self, JwtError> {
        let private_key_path = std::env::var("JWT_PRIVATE_PATH")
            .map_err(|_| JwtError::MissingEnvVar(
                "JWT_PRIVATE_PATH".to_string()
            ))?;

        let public_key_path = std::env::var("JWT_PUBLIC_PATH")
            .map_err(|_| JwtError::MissingEnvVar(
                "JWT_PUBLIC_PATH".to_string()
            ))?;

        let private_key = std::fs::read(private_key_path)
            .map_err(JwtError::ReadPrivateKey)?;

        let public_key = std::fs::read(public_key_path)
            .map_err(JwtError::ReadPublicKey)?;

        let encoding_key = EncodingKey::from_rsa_pem(&private_key)
            .map_err(JwtError::InvalidPrivateKey)?;

        let decoding_key = DecodingKey::from_rsa_pem(&public_key)
            .map_err(JwtError::InvalidPublicKey)?;

        let jwt_access_ttl = std::env::var("JWT_ACCESS_TTL")
            .map_err(|_| JwtError::MissingEnvVar(
                "JWT_ACCESS_TTL".to_string()
            ))?
                .parse::<u64>()
                .map_err(|_| JwtError::InvalidExpirationTime(
                    "JWT_ACCESS_TTL must be a valid number.".to_string()
            ))?;

        Ok(Self{
            encoding_key,
            decoding_key,
            access_ttl: jwt_access_ttl,
        })
    }

    pub fn create_token(&self, user_id: Uuid) -> Result<String, JwtError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| JwtError::InvalidExpirationTime(
                "System time is before UNIX epoch.".to_string()
            ))?
            .as_secs();

        let expiration_time = current_time + self.access_ttl;
        
        let exp = usize::try_from(expiration_time)
            .map_err(|_| JwtError::InvalidExpirationTime(
                "Expiration timestamp is too large.".to_string()
            ))?;
        
        let claims = Claims {
            sub: user_id,
            exp,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &self.encoding_key,
        )
        .map_err(JwtError::CreateToken)?;

        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &self.decoding_key,
            &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256),   
        )
        .map_err(JwtError::ValidateToken)?;
        
        Ok(token_data.claims)
    }
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::MissingEnvVar(var) =>
                write!(f, "{var} is not set"),

            JwtError::ReadPrivateKey(err) =>
                write!(f, "Failed to read JWT private key: {err}"),

            JwtError::ReadPublicKey(err) =>
                write!(f, "Failed to read JWT public key: {err}"),

            JwtError::InvalidPrivateKey(err) =>
                write!(f, "Invalid RSA private key: {err}"),
            
            JwtError::InvalidPublicKey(err) =>
                write!(f, "Invalid RSA public key: {err}"),
            
            JwtError::InvalidExpirationTime(err) =>
                write!(f, "Invalid token expiration time: {err}"),

            JwtError::CreateToken(err) =>
                write!(f, "Failed to create a JWT: {err}"),

            JwtError::ValidateToken(err) =>
                write!(f, "Failed to validate JWT: {err}"),
        }
    }
}