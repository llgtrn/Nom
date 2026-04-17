//! Stable buffer positions that survive edits.
#![deny(unsafe_code)]

/// Which side of an insertion point an anchor is attached to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bias {
    /// Anchor stays to the left of inserted text (offset does not shift).
    Left,
    /// Anchor moves to the right of inserted text (offset shifts up).
    Right,
}

/// A byte-offset into a buffer that adjusts automatically after edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anchor {
    pub offset: usize,
    pub bias: Bias,
}

impl Anchor {
    pub fn new(offset: usize, bias: Bias) -> Self {
        Self { offset, bias }
    }

    /// The canonical "before everything" anchor.
    pub fn min() -> Self {
        Self { offset: 0, bias: Bias::Left }
    }

    /// Adjust this anchor to reflect an insertion of `len` bytes at position `at`.
    ///
    /// - Insert strictly before offset → offset shifts up by `len`.
    /// - Insert at offset with `Bias::Left`  → anchor stays (attaches left of insert).
    /// - Insert at offset with `Bias::Right` → anchor shifts to after the inserted text.
    pub fn after_insert(self, at: usize, len: usize) -> Self {
        let new_offset = if at < self.offset {
            self.offset + len
        } else if at == self.offset {
            match self.bias {
                Bias::Left => self.offset,
                Bias::Right => self.offset + len,
            }
        } else {
            self.offset
        };
        Self { offset: new_offset, bias: self.bias }
    }

    /// Adjust this anchor to reflect a deletion of bytes in range `[start..end)`.
    ///
    /// - Anchor fully before deletion → unchanged.
    /// - Anchor inside or at start of deletion → clamped to `start`.
    /// - Anchor after deletion → shifted down by `(end - start)`.
    pub fn after_delete(self, start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "delete range must be ordered");
        let new_offset = if self.offset <= start {
            self.offset
        } else if self.offset < end {
            start
        } else {
            self.offset - (end - start)
        };
        Self { offset: new_offset, bias: self.bias }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn min_is_zero_left() {
        let a = Anchor::min();
        assert_eq!(a.offset, 0);
        assert_eq!(a.bias, Bias::Left);
    }

    #[test]
    fn insert_before_anchor_shifts_offset() {
        let a = Anchor::new(10, Bias::Left);
        let b = a.after_insert(5, 3);
        assert_eq!(b.offset, 13);
    }

    #[test]
    fn insert_at_left_bias_stays() {
        let a = Anchor::new(5, Bias::Left);
        let b = a.after_insert(5, 4);
        assert_eq!(b.offset, 5);
    }

    #[test]
    fn insert_at_right_bias_shifts() {
        let a = Anchor::new(5, Bias::Right);
        let b = a.after_insert(5, 4);
        assert_eq!(b.offset, 9);
    }

    #[test]
    fn insert_after_anchor_unchanged() {
        let a = Anchor::new(3, Bias::Left);
        let b = a.after_insert(10, 5);
        assert_eq!(b.offset, 3);
    }

    #[test]
    fn delete_before_anchor_shifts_back() {
        let a = Anchor::new(10, Bias::Left);
        let b = a.after_delete(2, 5);
        assert_eq!(b.offset, 7);
    }

    #[test]
    fn delete_around_anchor_clamps_to_start() {
        let a = Anchor::new(6, Bias::Right);
        let b = a.after_delete(4, 9);
        assert_eq!(b.offset, 4);
    }

    #[test]
    fn delete_after_anchor_unchanged() {
        let a = Anchor::new(3, Bias::Left);
        let b = a.after_delete(5, 10);
        assert_eq!(b.offset, 3);
    }

    #[test]
    fn anchor_usable_as_hashmap_key() {
        let mut map: HashMap<Anchor, &str> = HashMap::new();
        map.insert(Anchor::new(0, Bias::Left), "start");
        map.insert(Anchor::new(5, Bias::Right), "mid");
        assert_eq!(map[&Anchor::new(0, Bias::Left)], "start");
        assert_eq!(map[&Anchor::new(5, Bias::Right)], "mid");
    }
}
