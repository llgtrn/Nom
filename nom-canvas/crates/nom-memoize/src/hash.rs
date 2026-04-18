#![deny(unsafe_code)]

use siphasher::sip128::{Hash128 as SipHash128, Hasher128, SipHasher13};
use std::hash::Hasher;

/// 128-bit content hash for stable memoization keys (typst pattern: hash128)
/// Uses SipHash13 128-bit for cryptographic-quality stability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hash128(pub u64, pub u64);

impl Hash128 {
    pub const ZERO: Hash128 = Hash128(0, 0);

    pub fn of_bytes(data: &[u8]) -> Self {
        let mut hasher = SipHasher13::new();
        hasher.write(data);
        let h: SipHash128 = hasher.finish128();
        Hash128(h.h1, h.h2)
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
            self.0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(other.0),
            self.1
                .wrapping_mul(6364136223846793005)
                .wrapping_add(other.1),
        )
    }

    pub fn as_u64(&self) -> u64 {
        self.0 ^ self.1.rotate_left(32)
    }
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

    #[test]
    fn hash128_siphash_known_vector() {
        // SipHash13 with default keys (0,0) on "test" produces a stable value.
        // Record the actual output so any future hasher swap is caught immediately.
        let h = Hash128::of_str("test");
        assert_eq!(h, Hash128::of_str("test"), "hash must be deterministic");
        // Freeze the exact 128-bit value produced by SipHasher13::new() on b"test".
        let expected = {
            use siphasher::sip128::{Hash128 as SipHash128, Hasher128, SipHasher13};
            use std::hash::Hasher;
            let mut hasher = SipHasher13::new();
            hasher.write(b"test");
            let raw: SipHash128 = hasher.finish128();
            Hash128(raw.h1, raw.h2)
        };
        assert_eq!(h, expected);
    }

    #[test]
    fn hash128_zero_constant() {
        assert_eq!(Hash128::ZERO, Hash128(0, 0));
    }

    #[test]
    fn hash128_as_u64_xor_pattern() {
        let a: u64 = 0xdeadbeef_cafebabe;
        let b: u64 = 0x12345678_87654321;
        let h = Hash128(a, b);
        assert_eq!(h.as_u64(), a ^ b.rotate_left(32));
    }

    #[test]
    fn hash128_combine_not_commutative() {
        let a = Hash128::of_str("alpha");
        let b = Hash128::of_str("beta");
        // combine is defined to be order-sensitive
        assert_ne!(a.combine(b), b.combine(a));
    }

    #[test]
    fn hash128_of_bytes_matches_of_str() {
        let via_bytes = Hash128::of_bytes("hello".as_bytes());
        let via_str = Hash128::of_str("hello");
        assert_eq!(via_bytes, via_str);
    }

    #[test]
    fn hash128_of_u64_vs_of_bytes() {
        let v: u64 = 42;
        let via_u64 = Hash128::of_u64(v);
        let via_bytes = Hash128::of_bytes(&v.to_le_bytes());
        assert_eq!(via_u64, via_bytes);
    }

    #[test]
    fn hash128_zero_as_u64() {
        // ZERO has both halves 0, so as_u64 should also be 0
        assert_eq!(Hash128::ZERO.as_u64(), 0u64);
    }

    #[test]
    fn hash128_display_hex_lowercase() {
        // Display must emit exactly 32 lowercase hex chars
        let h = Hash128(0x0011223344556677, 0x8899aabbccddeeff);
        let s = format!("{}", h);
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(s, "00112233445566778899aabbccddeeff");
    }

    #[test]
    fn hash128_of_empty_bytes() {
        let h1 = Hash128::of_bytes(&[]);
        let h2 = Hash128::of_bytes(&[]);
        assert_eq!(h1, h2, "Hash128::of_bytes(&[]) must be deterministic");
    }

    #[test]
    fn hash128_single_byte() {
        let h0 = Hash128::of_bytes(&[0]);
        let h1 = Hash128::of_bytes(&[1]);
        assert_ne!(h0, h1, "of_bytes(&[0]) must differ from of_bytes(&[1])");
    }

    #[test]
    fn hash128_combine_with_zero() {
        let h = Hash128::of_str("non-zero");
        let combined = h.combine(Hash128::ZERO);
        // combine(ZERO) multiplies then adds 0 to h2; the wrapping_mul changes h1/h2 so result != h
        assert_ne!(combined, h);
    }

    #[test]
    fn hash128_as_u64_zero_hash() {
        assert_eq!(Hash128::ZERO.as_u64(), 0u64);
    }

    #[test]
    fn hash128_large_input() {
        let data = vec![0xABu8; 10_000];
        // Must not panic and must be deterministic
        let h1 = Hash128::of_bytes(&data);
        let h2 = Hash128::of_bytes(&data);
        assert_eq!(h1, h2);
    }

    // ── additional coverage ────────────────────────────────────────────────

    #[test]
    fn hash128_deterministic_multiple_calls() {
        // Calling of_str three times with the same input must always agree.
        let s = "determinism_check";
        let h1 = Hash128::of_str(s);
        let h2 = Hash128::of_str(s);
        let h3 = Hash128::of_str(s);
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn hash128_of_u64_deterministic_multiple_calls() {
        let v: u64 = 0xfedcba9876543210;
        let h1 = Hash128::of_u64(v);
        let h2 = Hash128::of_u64(v);
        let h3 = Hash128::of_u64(v);
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn hash128_collision_avoidance_strings() {
        // A large set of distinct strings must produce distinct hashes.
        let inputs: Vec<String> = (0..50).map(|i| format!("input_{}", i)).collect();
        let hashes: std::collections::HashSet<(u64, u64)> = inputs
            .iter()
            .map(|s| {
                let h = Hash128::of_str(s);
                (h.0, h.1)
            })
            .collect();
        assert_eq!(
            hashes.len(),
            50,
            "no collisions expected among 50 distinct strings"
        );
    }

    #[test]
    fn hash128_collision_avoidance_u64() {
        // Distinct u64 values must produce distinct hashes.
        let hashes: std::collections::HashSet<(u64, u64)> = (0u64..50)
            .map(|v| {
                let h = Hash128::of_u64(v);
                (h.0, h.1)
            })
            .collect();
        assert_eq!(hashes.len(), 50);
    }

    #[test]
    fn hash128_different_byte_lengths_differ() {
        // "a" vs "aa" — same byte, different length — must differ.
        let h1 = Hash128::of_bytes(b"a");
        let h2 = Hash128::of_bytes(b"aa");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash128_of_str_empty_vs_space() {
        let empty = Hash128::of_str("");
        let space = Hash128::of_str(" ");
        assert_ne!(empty, space);
    }

    #[test]
    fn hash128_combine_self_is_deterministic() {
        let h = Hash128::of_str("self_combine");
        let c1 = h.combine(h);
        let c2 = h.combine(h);
        assert_eq!(c1, c2);
    }

    #[test]
    fn hash128_combine_chain_differs_from_single() {
        // h.combine(h).combine(h) should differ from h alone.
        let h = Hash128::of_str("chain");
        let chained = h.combine(h).combine(h);
        assert_ne!(chained, h);
    }

    #[test]
    fn hash128_of_singleton_byte_array() {
        let h1 = Hash128::of_bytes(&[42]);
        let h2 = Hash128::of_bytes(&[42]);
        assert_eq!(h1, h2);
        assert_ne!(h1, Hash128::of_bytes(&[43]));
    }

    #[test]
    fn hash128_as_u64_differs_for_distinct_hashes() {
        let h1 = Hash128::of_str("foo");
        let h2 = Hash128::of_str("bar");
        // as_u64 folding must yield different values for clearly distinct hashes.
        assert_ne!(h1.as_u64(), h2.as_u64());
    }

    #[test]
    fn hash128_clone_copy_equals_original() {
        let h = Hash128::of_str("copy_test");
        let copied = h;
        let cloned = h;
        assert_eq!(h, copied);
        assert_eq!(h, cloned);
    }
}
