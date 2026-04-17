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
}
