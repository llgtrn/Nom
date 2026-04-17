//! Stub token decoder — placeholder for real JWT validation.
//!
//! Expected token format: `"<user_id>:<workspace_id>:ro:<expires_ms>"` (read-only)
//!                     or `"<user_id>:<workspace_id>:rw:<expires_ms>"` (read-write).
//! Modelled after huly's `decodeToken` + `isReadOnlyOrGuest` pattern.

use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct AuthClaims {
    pub user_id: String,
    pub workspace_id: String,
    pub readonly: bool,
    pub expires_ms: u64,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("malformed token")]
    MalformedToken,
    #[error("token expired")]
    Expired,
}

/// Decode a stub token.  Pass `now_ms = 0` to skip expiry checks in tests.
pub fn decode_stub_token(token: &str) -> Result<AuthClaims, AuthError> {
    decode_stub_token_at(token, 0)
}

/// Decode a stub token and check expiry against `now_ms`.
pub fn decode_stub_token_at(token: &str, now_ms: u64) -> Result<AuthClaims, AuthError> {
    let parts: Vec<&str> = token.splitn(4, ':').collect();
    if parts.len() != 4 {
        return Err(AuthError::MalformedToken);
    }
    let user_id = parts[0];
    let workspace_id = parts[1];
    let readonly = match parts[2] {
        "ro" => true,
        "rw" => false,
        _ => return Err(AuthError::MalformedToken),
    };
    let expires_ms: u64 = parts[3].parse().map_err(|_| AuthError::MalformedToken)?;
    if now_ms > 0 && now_ms > expires_ms {
        return Err(AuthError::Expired);
    }
    Ok(AuthClaims {
        user_id: user_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        readonly,
        expires_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_readonly_token() {
        let claims = decode_stub_token("alice:ws-1:ro:9999999").unwrap();
        assert_eq!(claims.user_id, "alice");
        assert_eq!(claims.workspace_id, "ws-1");
        assert!(claims.readonly);
        assert_eq!(claims.expires_ms, 9999999);
    }

    #[test]
    fn decode_readwrite_token() {
        let claims = decode_stub_token("bob:ws-2:rw:1234567").unwrap();
        assert!(!claims.readonly);
        assert_eq!(claims.user_id, "bob");
    }

    #[test]
    fn malformed_token_missing_field() {
        let result = decode_stub_token("user:ws:ro"); // only 3 parts
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn malformed_token_bad_mode() {
        let result = decode_stub_token("user:ws:admin:999");
        assert!(matches!(result, Err(AuthError::MalformedToken)));
    }

    #[test]
    fn expired_token_rejected() {
        let result = decode_stub_token_at("user:ws:rw:1000", 2000);
        assert!(matches!(result, Err(AuthError::Expired)));
    }
}
