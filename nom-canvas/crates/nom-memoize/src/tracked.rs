use crate::constraint::Constraint;

/// A thin wrapper over a shared reference that optionally carries a
/// [`Constraint`] recorder so that field accesses can be tracked.
pub struct Tracked<'a, T: ?Sized> {
    pub(crate) inner: &'a T,
    // Consumed by the proc-macro runtime in `nom-memoize-macros`; not yet read
    // inside this library crate.
    #[allow(dead_code)]
    pub(crate) constraint: Option<&'a Constraint<T>>,
}

impl<'a, T: ?Sized> std::ops::Deref for Tracked<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner
    }
}

/// Wrap `value` without any constraint recorder.
pub fn track<'a, T: ?Sized>(value: &'a T) -> Tracked<'a, T> {
    Tracked { inner: value, constraint: None }
}

/// Wrap `value` and attach a constraint recorder so that accesses can be
/// observed and replayed for cache validation.
pub fn track_with<'a, T: ?Sized>(value: &'a T, constraint: &'a Constraint<T>) -> Tracked<'a, T> {
    Tracked { inner: value, constraint: Some(constraint) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::Constraint;

    #[test]
    fn deref_works() {
        let value = String::from("hello");
        let tracked = track(value.as_str());
        assert_eq!(&*tracked, "hello");
    }

    #[test]
    fn address_preserved() {
        let value = 42u32;
        let tracked = track(&value);
        assert_eq!(tracked.inner as *const u32, &value as *const u32);
    }

    #[test]
    fn constraint_none_by_default() {
        let value = 0u8;
        let tracked = track(&value);
        assert!(tracked.constraint.is_none());
    }

    #[test]
    fn track_with_sets_constraint() {
        let value = 0u8;
        let c: Constraint<u8> = Constraint::new();
        let tracked = track_with(&value, &c);
        assert!(tracked.constraint.is_some());
    }
}
