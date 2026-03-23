use ldap3::LdapConnAsync;

pub struct LdapConfig {
    /// e.g. `ldap://corp.example.com:389` or `ldaps://corp.example.com:636`
    pub url: String,
    /// DN template where `{}` is replaced by the sanitised uid.
    /// e.g. `uid={},ou=users,dc=corp,dc=example,dc=com`
    pub bind_dn_template: String,
}

impl LdapConfig {
    /// Reads configuration from environment variables.
    /// Returns `None` if any required variable is absent.
    pub fn from_env() -> Option<Self> {
        Some(Self {
            url: std::env::var("LDAP_URL").ok()?,
            bind_dn_template: std::env::var("LDAP_BIND_DN_TEMPLATE").ok()?,
        })
    }
}

/// Tries to authenticate `ldap_uid` / `password` via a simple LDAP bind.
///
/// Returns `Ok(true)` when the bind succeeds, `Ok(false)` when the server
/// rejects the credentials, and `Err` only on transport / protocol errors.
pub async fn verify_ldap(
    ldap_uid: &str,
    password: &str,
    cfg: &LdapConfig,
) -> Result<bool, ldap3::LdapError> {
    // Reject empty passwords — some LDAP servers allow unauthenticated binds
    // for an empty password (RFC 4513 §5.1.2).
    if password.is_empty() {
        return Ok(false);
    }

    // Whitelist the UID to prevent DN injection: only allow characters safe
    // to embed in a Distinguished Name attribute value without escaping.
    if !ldap_uid
        .chars()
        .all(|c| c.is_alphanumeric() || "._-".contains(c))
    {
        return Ok(false);
    }

    let bind_dn = cfg.bind_dn_template.replace("{}", ldap_uid);

    let (conn, mut ldap) = LdapConnAsync::new(&cfg.url).await?;
    ldap3::drive!(conn);

    let result = ldap.simple_bind(&bind_dn, password).await?;
    let _ = ldap.unbind().await;

    // `success()` returns Err for any non-zero LDAP result code (e.g. 49 = InvalidCredentials).
    Ok(result.success().is_ok())
}
