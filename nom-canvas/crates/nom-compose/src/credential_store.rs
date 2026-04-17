//! Plaintext credential store (AES encryption tracked separately).
//!
//! Keys use `(kind, id)` pairs so that e.g. `("api-key", "openai")` and
//! `("api-key", "anthropic")` are independent slots.

use std::collections::HashMap;

use crate::backend_trait::ComposeSpec;

pub struct CredentialStore {
    entries: HashMap<(String, String), Vec<u8>>,
}

impl CredentialStore {
    pub fn new() -> Self {
        CredentialStore {
            entries: HashMap::new(),
        }
    }

    /// Store `plaintext` under `(kind, id)`.
    pub fn put(&mut self, kind: impl Into<String>, id: impl Into<String>, plaintext: Vec<u8>) {
        self.entries.insert((kind.into(), id.into()), plaintext);
    }

    /// Retrieve the bytes for `(kind, id)`.
    pub fn get(&self, kind: &str, id: &str) -> Option<Vec<u8>> {
        self.entries
            .get(&(kind.to_owned(), id.to_owned()))
            .cloned()
    }

    /// Remove and return whether the entry existed.
    pub fn remove(&mut self, kind: &str, id: &str) -> bool {
        self.entries
            .remove(&(kind.to_owned(), id.to_owned()))
            .is_some()
    }

    /// Replace the value of any param whose key starts with `"credential:"` with `"<redacted>"`.
    /// All other params are left unchanged.
    pub fn redact_in_spec(&self, spec: &mut ComposeSpec) {
        for (key, value) in spec.params.iter_mut() {
            if key.starts_with("credential:") {
                *value = "<redacted>".to_owned();
            }
        }
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend_trait::ComposeSpec, kind::NomKind};

    #[test]
    fn put_and_get_round_trip() {
        let mut store = CredentialStore::new();
        store.put("api-key", "vendor-a", b"secret".to_vec());
        assert_eq!(store.get("api-key", "vendor-a").unwrap(), b"secret");
    }

    #[test]
    fn get_missing_returns_none() {
        let store = CredentialStore::new();
        assert!(store.get("api-key", "nobody").is_none());
    }

    #[test]
    fn remove_returns_true_on_hit() {
        let mut store = CredentialStore::new();
        store.put("token", "svc", b"tok".to_vec());
        assert!(store.remove("token", "svc"));
        assert!(store.get("token", "svc").is_none());
    }

    #[test]
    fn remove_returns_false_on_miss() {
        let mut store = CredentialStore::new();
        assert!(!store.remove("ghost", "nothing"));
    }

    #[test]
    fn redact_replaces_credential_params() {
        let store = CredentialStore::new();
        let mut spec = ComposeSpec {
            kind: NomKind::DataQuery,
            params: vec![
                ("credential:api-key".into(), "supersecret".into()),
                ("width".into(), "512".into()),
            ],
        };
        store.redact_in_spec(&mut spec);
        assert_eq!(spec.params[0].1, "<redacted>");
        assert_eq!(spec.params[1].1, "512"); // non-credential untouched
    }

    #[test]
    fn redact_preserves_non_credential_params() {
        let store = CredentialStore::new();
        let mut spec = ComposeSpec {
            kind: NomKind::MediaImage,
            params: vec![
                ("format".into(), "png".into()),
                ("quality".into(), "95".into()),
            ],
        };
        store.redact_in_spec(&mut spec);
        assert_eq!(spec.params[0].1, "png");
        assert_eq!(spec.params[1].1, "95");
    }
}
