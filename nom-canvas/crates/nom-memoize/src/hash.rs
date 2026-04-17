#![deny(unsafe_code)]

/// 128-bit content hash for stable memoization keys (typst pattern: hash128)
/// TODO: Replace with SipHash13 via siphasher::sip128::SipHasher13
/// Current: FNV-1a dual-chain (structurally equivalent, different constants)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hash128(pub u64, pub u64);

impl Hash128 {
    pub const ZERO: Hash128 = Hash128(0, 0);

    pub fn of_bytes(data: &[u8]) -> Self {
        // FNV-1a 128-bit: two independent 64-bit FNV chains
        const FNV_PRIME_64: u64 = 1099511628211;
        const FNV_OFFSET_A: u64 = 14695981039346656037;
        const FNV_OFFSET_B: u64 = 0xcbf29ce484222325u64.wrapping_add(0x517cc1b727220a95);

        let mut h0 = FNV_OFFSET_A;
        let mut h1 = FNV_OFFSET_B;
        for &byte in data {
            h0 ^= byte as u64;
            h0 = h0.wrapping_mul(FNV_PRIME_64);
            h1 ^= (byte as u64).wrapping_add(1);
            h1 = h1.wrapping_mul(FNV_PRIME_64).wrapping_add(h1.rotate_left(17));
        }
        Hash128(h0, h1)
    }

    pub fn of_str(s: &str) -> Self {
        Self::of_bytes(s.as_bytes())
    }

    pub fn of_u64(v: u64) -> Self {
        Self::of_bytes(&v.to_le_bytes())
    }

    /// Combine two hashes (for multi-input memoization)
    pub fn combine(self, other: Hash128) -> Hash128 {
        Hash128(
            self.0.wrapping_mul(6364136223846793005).wrapping_add(other.0),
            self.1.wrapping_mul(6364136223846793005).wrapping_add(other.1),
        )
    }

    pub fn as_u64(&self) -> u64 { self.0 ^ self.1.rotate_left(32) }
}

impl std::fmt::Display for Hash128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}{:016x}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash128_deterministic() {
        let h1 = Hash128::of_str("hello world");
        let h2 = Hash128::of_str("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash128_different_inputs() {
        let h1 = Hash128::of_str("hello");
        let h2 = Hash128::of_str("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash128_combine_is_order_sensitive() {
        let a = Hash128::of_str("a");
        let b = Hash128::of_str("b");
        assert_ne!(a.combine(b), b.combine(a));
    }

    #[test]
    fn hash128_display() {
        let h = Hash128(0xdeadbeef, 0xcafebabe);
        let s = format!("{}", h);
        assert_eq!(s.len(), 32);
    }

    #[test]
    fn hash128_of_u64_deterministic() {
        let h1 = Hash128::of_u64(42);
        let h2 = Hash128::of_u64(42);
        assert_eq!(h1, h2);
        assert_ne!(h1, Hash128::of_u64(43));
    }
}
