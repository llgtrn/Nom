#![deny(unsafe_code)]

use std::collections::HashMap;

/// A credential entry for a vendor/service.
#[derive(Debug, Clone)]
pub struct Credential {
    pub kind: String,    // e.g., "api_key", "bearer_token", "oauth2"
    pub value: String,   // the secret value
}

/// Kind-keyed credential store (per spec: "Kind-keyed secrets").
pub struct CredentialStore {
    entries: HashMap<String, Credential>,
}

impl CredentialStore {
    pub fn new() -> Self { Self { entries: HashMap::new() } }

    pub fn set(&mut self, vendor: impl Into<String>, cred: Credential) {
        self.entries.insert(vendor.into(), cred);
    }

    pub fn get(&self, vendor: &str) -> Option<&Credential> {
        self.entries.get(vendor)
    }

    pub fn remove(&mut self, vendor: &str) -> bool {
        self.entries.remove(vendor).is_some()
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn vendor_names(&self) -> Vec<&str> { self.entries.keys().map(|s| s.as_str()).collect() }
}

impl Default for CredentialStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_store_set_and_get() {
        let mut s = CredentialStore::new();
        s.set("openai", Credential { kind: "api_key".into(), value: "sk-test".into() });
        assert_eq!(s.get("openai").unwrap().value, "sk-test");
        assert!(s.get("unknown").is_none());
    }
    #[test]
    fn credential_store_remove() {
        let mut s = CredentialStore::new();
        s.set("v", Credential { kind: "api_key".into(), value: "x".into() });
        assert!(s.remove("v"));
        assert!(!s.remove("v"));
    }
    #[test]
    fn credential_store_len() {
        let mut s = CredentialStore::new();
        assert_eq!(s.len(), 0);
        s.set("a", Credential { kind: "k".into(), value: "v".into() });
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn credential_store_store_retrieve() {
        let mut s = CredentialStore::new();
        s.set("openai", Credential { kind: "api_key".into(), value: "sk-xxx".into() });
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
        s.set("svc", Credential { kind: "bearer".into(), value: "first".into() });
        s.set("svc", Credential { kind: "bearer".into(), value: "second".into() });
        assert_eq!(s.get("svc").unwrap().value, "second");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn credential_store_remove_retrieves_none() {
        let mut s = CredentialStore::new();
        s.set("k", Credential { kind: "api_key".into(), value: "val".into() });
        assert!(s.remove("k"));
        assert!(s.get("k").is_none());
    }

    #[test]
    fn credential_store_count() {
        let mut s = CredentialStore::new();
        s.set("a", Credential { kind: "api_key".into(), value: "1".into() });
        s.set("b", Credential { kind: "api_key".into(), value: "2".into() });
        s.set("c", Credential { kind: "api_key".into(), value: "3".into() });
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
        s.set("alpha", Credential { kind: "api_key".into(), value: "x".into() });
        s.set("beta", Credential { kind: "api_key".into(), value: "y".into() });
        let mut names = s.vendor_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }
}
