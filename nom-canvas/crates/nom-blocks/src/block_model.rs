use crate::flavour::Flavour;

/// Opaque 64-bit identifier for a block instance.
pub type BlockId = u64;

/// Fractional index string used for ordering blocks (e.g. "a0", "a1").
pub type FractionalIndex = String;

/// A comment attached to a block.
#[derive(Debug, Clone)]
pub struct BlockComment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub created_at_ms: u64,
}

/// Lifecycle and collaboration metadata carried by every block.
#[derive(Debug, Clone)]
pub struct BlockMeta {
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub comments: Vec<BlockComment>,
}

impl Default for BlockMeta {
    fn default() -> Self {
        Self {
            created_at_ms: 0,
            updated_at_ms: 0,
            created_by: None,
            updated_by: None,
            comments: Vec::new(),
        }
    }
}

/// Generic block model parameterised over its content props.
#[derive(Debug, Clone)]
pub struct BlockModel<Props> {
    pub id: BlockId,
    pub flavour: Flavour,
    pub props: Props,
    pub children: Vec<BlockId>,
    pub meta: BlockMeta,
    pub version: u32,
    pub version_nonce: u64,
}

impl<Props> BlockModel<Props> {
    /// Construct a new block with sensible defaults (version=0, nonce=0, empty meta).
    pub fn new(id: BlockId, flavour: Flavour, props: Props) -> Self {
        Self {
            id,
            flavour,
            props,
            children: Vec::new(),
            meta: BlockMeta::default(),
            version: 0,
            version_nonce: 0,
        }
    }

    /// Increment version and derive a new nonce from the current version.
    pub fn bump_version(&mut self) {
        self.bump_version_with(simple_nonce(self.version as u64));
    }

    /// Increment version and set an explicit nonce (useful in deterministic tests).
    pub fn bump_version_with(&mut self, nonce: u64) {
        self.version = self.version.saturating_add(1);
        self.version_nonce = nonce;
    }

    /// Append a child block id.
    pub fn add_child(&mut self, id: BlockId) {
        self.children.push(id);
    }

    /// Remove first occurrence of a child block id.
    pub fn remove_child(&mut self, id: BlockId) {
        self.children.retain(|&c| c != id);
    }
}

/// Deterministic nonce derivation — cheap bit-mix, not cryptographic.
fn simple_nonce(seed: u64) -> u64 {
    let mut v = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    v = (v ^ (v >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    v = (v ^ (v >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    v ^ (v >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_defaults() {
        let m: BlockModel<i32> = BlockModel::new(1, "nom:test", 42);
        assert_eq!(m.version, 0);
        assert_eq!(m.version_nonce, 0);
        assert!(m.children.is_empty());
        assert_eq!(m.meta.created_at_ms, 0);
    }

    #[test]
    fn bump_version_increments() {
        let mut m: BlockModel<i32> = BlockModel::new(1, "nom:test", 0);
        m.bump_version_with(99);
        assert_eq!(m.version, 1);
        assert_eq!(m.version_nonce, 99);
        m.bump_version_with(7);
        assert_eq!(m.version, 2);
    }

    #[test]
    fn add_child_appends() {
        let mut m: BlockModel<()> = BlockModel::new(1, "nom:test", ());
        m.add_child(10);
        m.add_child(20);
        assert_eq!(m.children, vec![10, 20]);
    }

    #[test]
    fn remove_child_works() {
        let mut m: BlockModel<()> = BlockModel::new(1, "nom:test", ());
        m.add_child(10);
        m.add_child(20);
        m.remove_child(10);
        assert_eq!(m.children, vec![20]);
    }
}
