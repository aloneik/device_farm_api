use crate::models::domain::Role;
use crate::state::AppState;
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Claims embedded in every issued JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Username (`sub`ject).
    pub sub: String,
    pub role: Role,
    /// Expiry as a Unix timestamp (seconds).
    pub exp: u64,
}

/// Issue a signed HS256 JWT with the given subject, role, and TTL.
pub fn create_token(sub: &str, role: Role, secret: &str, ttl_secs: u64) -> String {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + ttl_secs;
    let claims = Claims {
        sub: sub.to_owned(),
        role,
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("JWT encoding failed")
}

/// Validate a token and return its claims, or an error.
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

/// Axum extractor.  Adding `claims: Claims` (or `_claims: Claims`) to any handler
/// parameter list makes that route require a valid `Authorization: Bearer <jwt>` header.
/// Returns `401 Unauthorized` if the header is missing or the token is invalid/expired.
impl FromRequestParts<AppState> for Claims {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, StatusCode> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        verify_token(token, &state.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)
    }
}
