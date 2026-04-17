//! Document identifier — a newtype over String.

use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocId(pub String);

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for DocId {
    fn from(s: &str) -> Self {
        DocId(s.to_owned())
    }
}

impl FromStr for DocId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DocId(s.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_roundtrip() {
        let id = DocId("workspace/doc-1".to_owned());
        assert_eq!(id.to_string(), "workspace/doc-1");
    }

    #[test]
    fn from_str_roundtrip() {
        let id: DocId = "hello-doc".parse().unwrap();
        assert_eq!(id.0, "hello-doc");
    }

    #[test]
    fn from_str_ref_and_equality() {
        let a = DocId::from("abc");
        let b: DocId = "abc".parse().unwrap();
        assert_eq!(a, b);
    }
}
