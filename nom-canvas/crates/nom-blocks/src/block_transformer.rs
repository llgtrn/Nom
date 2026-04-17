use thiserror::Error;

use crate::flavour::Flavour;

/// A portable serialisation envelope for a block's props.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub flavour: Flavour,
    pub version: u32,
    pub data: Vec<u8>,
}

/// Errors that can occur while converting between props and snapshots.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransformError {
    #[error("invalid data in snapshot")]
    InvalidData,
    #[error("version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
    #[error("missing required field '{0}'")]
    MissingField(&'static str),
}

/// Converts a block's props to/from a serialised [`Snapshot`].
pub trait BlockTransformer {
    type Props;

    fn from_snapshot(&self, snap: &Snapshot) -> Result<Self::Props, TransformError>;
    fn to_snapshot(&self, props: &Self::Props) -> Snapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trivial transformer: props are raw bytes, snapshot wraps them as-is.
    struct BytesTransformer;

    impl BlockTransformer for BytesTransformer {
        type Props = Vec<u8>;

        fn from_snapshot(&self, snap: &Snapshot) -> Result<Vec<u8>, TransformError> {
            Ok(snap.data.clone())
        }

        fn to_snapshot(&self, props: &Vec<u8>) -> Snapshot {
            Snapshot {
                flavour: "nom:test",
                version: 1,
                data: props.clone(),
            }
        }
    }

    #[test]
    fn round_trip_bytes_are_identical() {
        let t = BytesTransformer;
        let original: Vec<u8> = vec![1, 2, 3, 4, 5];
        let snap = t.to_snapshot(&original);
        let recovered = t.from_snapshot(&snap).unwrap();
        assert_eq!(original, recovered);
    }
}
