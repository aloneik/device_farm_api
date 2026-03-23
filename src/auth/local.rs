use crate::models::domain::UserEntry;
use argon2::{Argon2, PasswordHash, PasswordVerifier};

/// Verifies `password` against the Argon2id hash stored in `user.password_hash`.
/// Returns `false` if the user has no hash configured or if the password doesn't match.
pub fn verify_local(password: &str, user: &UserEntry) -> bool {
    let Some(ref hash_str) = user.password_hash else {
        return false;
    };
    let Ok(parsed_hash) = PasswordHash::new(hash_str) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
