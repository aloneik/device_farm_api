use crate::auth::{
    jwt::create_token,
    ldap::{verify_ldap, LdapConfig},
    local::verify_local,
};
use crate::models::{AuthMethod, LoginRequest, LoginResponse};
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use tracing::warn;

/// `POST /auth/login`
///
/// Accepts a username + password and returns a signed JWT on success.
/// The authentication backend (local Argon2 or LDAP bind) is determined by the
/// user's entry in `users.json` — the client cannot override it.
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Look up the user; return 401 (not 404) to avoid leaking valid usernames.
    let user = state
        .users
        .iter()
        .find(|u| u.username == body.username)
        .ok_or_else(|| {
            warn!(username = %body.username, "Login: unknown user");
            StatusCode::UNAUTHORIZED
        })?;

    let authenticated = match user.auth {
        AuthMethod::Local => verify_local(&body.password, user),
        AuthMethod::Ldap => {
            let ldap_uid = user.ldap_uid.as_deref().ok_or_else(|| {
                warn!(username = %body.username, "Login: ldap_uid not configured for LDAP user");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            let cfg = LdapConfig::from_env().ok_or_else(|| {
                warn!("Login: LDAP_URL or LDAP_BIND_DN_TEMPLATE env vars not set");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            verify_ldap(ldap_uid, &body.password, &cfg)
                .await
                .map_err(|e| {
                    warn!(error = %e, "Login: LDAP transport error");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        }
    };

    if !authenticated {
        warn!(username = %body.username, "Login: invalid credentials");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_token(&user.username, user.role.clone(), &state.jwt_secret, 3600);
    Ok(Json(LoginResponse { token }))
}
