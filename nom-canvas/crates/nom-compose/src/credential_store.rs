#![deny(unsafe_code)]

use std::collections::HashMap;

/// A credential entry for a vendor/service.
#[derive(Debug, Clone)]
pub struct Credential {
    pub kind: String,  // e.g., "api_key", "bearer_token", "oauth2"
    pub value: String, // the secret value
}

/// Kind-keyed credential store (per spec: "Kind-keyed secrets").
pub struct CredentialStore {
    entries: HashMap<String, Credential>,
}

impl CredentialStore {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn set(&mut self, vendor: impl Into<String>, cred: Credential) {
        self.entries.insert(vendor.into(), cred);
    }

    pub fn get(&self, vendor: &str) -> Option<&Credential> {
        self.entries.get(vendor)
    }

    pub fn remove(&mut self, vendor: &str) -> bool {
        self.entries.remove(vendor).is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn vendor_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_store_set_and_get() {
        let mut s = CredentialStore::new();
        s.set(
            "openai",
            Credential {
                kind: "api_key".into(),
                value: "sk-test".into(),
            },
        );
        assert_eq!(s.get("openai").unwrap().value, "sk-test");
        assert!(s.get("unknown").is_none());
    }
    #[test]
    fn credential_store_remove() {
        let mut s = CredentialStore::new();
        s.set(
            "v",
            Credential {
                kind: "api_key".into(),
                value: "x".into(),
            },
        );
        assert!(s.remove("v"));
        assert!(!s.remove("v"));
    }
    #[test]
    fn credential_store_len() {
        let mut s = CredentialStore::new();
        assert_eq!(s.len(), 0);
        s.set(
            "a",
            Credential {
                kind: "k".into(),
                value: "v".into(),
            },
        );
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn credential_store_store_retrieve() {
        let mut s = CredentialStore::new();
        s.set(
            "openai",
            Credential {
                kind: "api_key".into(),
                value: "sk-xxx".into(),
            },
        );
        let cred = s.get("openai").unwrap();
        assert_eq!(cred.value, "sk-xxx");
        assert_eq!(cred.kind, "api_key");
    }

    #[test]
    fn credential_store_miss_returns_none() {
        let s = CredentialStore::new();
        assert!(s.get("nonexistent").is_none());
    }

    #[test]
    fn credential_store_overwrite() {
        let mut s = CredentialStore::new();
        s.set(
            "svc",
            Credential {
                kind: "bearer".into(),
                value: "first".into(),
            },
        );
        s.set(
            "svc",
            Credential {
                kind: "bearer".into(),
                value: "second".into(),
            },
        );
        assert_eq!(s.get("svc").unwrap().value, "second");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn credential_store_remove_retrieves_none() {
        let mut s = CredentialStore::new();
        s.set(
            "k",
            Credential {
                kind: "api_key".into(),
                value: "val".into(),
            },
        );
        assert!(s.remove("k"));
        assert!(s.get("k").is_none());
    }

    #[test]
    fn credential_store_count() {
        let mut s = CredentialStore::new();
        s.set(
            "a",
            Credential {
                kind: "api_key".into(),
                value: "1".into(),
            },
        );
        s.set(
            "b",
            Credential {
                kind: "api_key".into(),
                value: "2".into(),
            },
        );
        s.set(
            "c",
            Credential {
                kind: "api_key".into(),
                value: "3".into(),
            },
        );
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn credential_store_is_empty_on_new() {
        let s = CredentialStore::new();
        assert!(s.is_empty());
    }

    #[test]
    fn credential_store_vendor_names() {
        let mut s = CredentialStore::new();
        s.set(
            "alpha",
            Credential {
                kind: "api_key".into(),
                value: "x".into(),
            },
        );
        s.set(
            "beta",
            Credential {
                kind: "api_key".into(),
                value: "y".into(),
            },
        );
        let mut names = s.vendor_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn credential_store_default_is_empty() {
        let s = CredentialStore::default();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn credential_store_remove_nonexistent_returns_false() {
        let mut s = CredentialStore::new();
        assert!(
            !s.remove("ghost"),
            "removing non-existent vendor must return false"
        );
    }

    #[test]
    fn credential_kind_field_preserved() {
        let mut s = CredentialStore::new();
        s.set(
            "svc",
            Credential {
                kind: "bearer_token".into(),
                value: "tok123".into(),
            },
        );
        let cred = s.get("svc").unwrap();
        assert_eq!(cred.kind, "bearer_token");
        assert_eq!(cred.value, "tok123");
    }

    #[test]
    fn credential_store_many_vendors() {
        let mut s = CredentialStore::new();
        let vendors = ["openai", "anthropic", "cohere", "gemini", "mistral"];
        for v in &vendors {
            s.set(
                *v,
                Credential {
                    kind: "api_key".into(),
                    value: format!("key-{v}"),
                },
            );
        }
        assert_eq!(s.len(), vendors.len());
        for v in &vendors {
            assert!(s.get(v).is_some(), "vendor {v} must be present");
        }
    }

    #[test]
    fn credential_per_kind_isolation() {
        let mut s = CredentialStore::new();
        s.set(
            "svc_api",
            Credential {
                kind: "api_key".into(),
                value: "key-a".into(),
            },
        );
        s.set(
            "svc_bearer",
            Credential {
                kind: "bearer_token".into(),
                value: "tok-b".into(),
            },
        );
        assert_eq!(s.get("svc_api").unwrap().kind, "api_key");
        assert_eq!(s.get("svc_bearer").unwrap().kind, "bearer_token");
        assert_ne!(
            s.get("svc_api").unwrap().value,
            s.get("svc_bearer").unwrap().value
        );
    }

    #[test]
    fn credential_store_overwrite_changes_kind_and_value() {
        let mut s = CredentialStore::new();
        s.set(
            "svc",
            Credential {
                kind: "api_key".into(),
                value: "old_key".into(),
            },
        );
        s.set(
            "svc",
            Credential {
                kind: "oauth2".into(),
                value: "new_tok".into(),
            },
        );
        let c = s.get("svc").unwrap();
        assert_eq!(c.kind, "oauth2");
        assert_eq!(c.value, "new_tok");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn credential_store_missing_key_is_none() {
        let s = CredentialStore::new();
        assert!(
            s.get("missing_vendor").is_none(),
            "missing key must return None"
        );
    }

    #[test]
    fn credential_store_vendor_names_after_remove() {
        let mut s = CredentialStore::new();
        s.set(
            "x",
            Credential {
                kind: "api_key".into(),
                value: "v".into(),
            },
        );
        s.set(
            "y",
            Credential {
                kind: "api_key".into(),
                value: "w".into(),
            },
        );
        s.remove("x");
        let names = s.vendor_names();
        assert_eq!(names, vec!["y"]);
    }

    #[test]
    fn credential_store_set_empty_value() {
        let mut s = CredentialStore::new();
        s.set(
            "svc",
            Credential {
                kind: "api_key".into(),
                value: "".into(),
            },
        );
        assert_eq!(s.get("svc").unwrap().value, "");
    }

    #[test]
    fn credential_store_len_after_remove_decreases() {
        let mut s = CredentialStore::new();
        s.set("a", Credential { kind: "k".into(), value: "v".into() });
        s.set("b", Credential { kind: "k".into(), value: "v".into() });
        assert_eq!(s.len(), 2);
        s.remove("a");
        assert_eq!(s.len(), 1);
        s.remove("b");
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }
}
