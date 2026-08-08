use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn new(user_id: Uuid, email: &str, role: &str, expiration: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as usize;

        Self {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            exp: now + expiration as usize,
            iat: now,
        }
    }
}

pub fn generate_token(
    claims: &Claims,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    jsonwebtoken::encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

pub fn extract_token(auth_header: Option<&str>) -> Option<String> {
    auth_header
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(|token| token.trim().to_string())
}
