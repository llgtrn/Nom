//! DataView block: filtered, sorted views over a source kind.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

/// A filter predicate for a [`DataViewBlock`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataViewFilter {
    /// Value must contain the given substring.
    Contains(String),
    /// Value must exactly equal the given string.
    Equals(String),
    /// Numeric value must be strictly greater than the threshold.
    GreaterThan(f64),
    /// Numeric value must be strictly less than the threshold.
    LessThan(f64),
}

impl DataViewFilter {
    /// Returns true if `value` satisfies this filter.
    /// For numeric filters (GreaterThan, LessThan) the value is parsed as f64.
    pub fn matches(&self, value: &str) -> bool {
        match self {
            DataViewFilter::Contains(substr) => value.contains(substr.as_str()),
            DataViewFilter::Equals(expected) => value == expected.as_str(),
            DataViewFilter::GreaterThan(threshold) => {
                value.parse::<f64>().is_ok_and(|n| n > *threshold)
            }
            DataViewFilter::LessThan(threshold) => {
                value.parse::<f64>().is_ok_and(|n| n < *threshold)
            }
        }
    }
}

/// A block representing a filtered, sorted view over entries of a given grammar kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataViewBlock {
    /// DB entity reference (NON-OPTIONAL).
    pub entity: NomtuRef,
    /// Grammar kind of the data source.
    pub source_kind: String,
    /// Active filter predicates (AND-combined).
    pub filters: Vec<DataViewFilter>,
    /// Column name to sort by (or `None` for default order).
    pub sort_column: Option<String>,
    /// `true` for ascending, `false` for descending.
    pub sort_ascending: bool,
    /// Maximum number of rows per page.
    pub page_size: usize,
}

impl DataViewBlock {
    /// Construct a new [`DataViewBlock`] with no filters and default sort settings.
    pub fn new(entity: NomtuRef, source_kind: impl Into<String>) -> Self {
        Self {
            entity,
            source_kind: source_kind.into(),
            filters: Vec::new(),
            sort_column: None,
            sort_ascending: true,
            page_size: 20,
        }
    }

    /// Append a filter predicate.
    pub fn add_filter(&mut self, filter: DataViewFilter) {
        self.filters.push(filter);
    }

    /// Remove all active filters.
    pub fn clear_filters(&mut self) {
        self.filters.clear();
    }

    /// Returns true if `value` satisfies ALL active filters (AND semantics).
    /// With no filters, always returns true.
    pub fn matches_filter(&self, value: &str) -> bool {
        if self.filters.is_empty() {
            return true;
        }
        self.filters.iter().all(|f| f.matches(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dv(entity_id: &str, source_kind: &str) -> DataViewBlock {
        let entity = NomtuRef::new(entity_id, "view", "concept");
        DataViewBlock::new(entity, source_kind)
    }

    #[test]
    fn dataview_new_has_empty_filters() {
        let dv = make_dv("dv-01", "verb");
        assert!(dv.filters.is_empty());
    }

    #[test]
    fn dataview_add_filter_increments_count() {
        let mut dv = make_dv("dv-02", "verb");
        assert_eq!(dv.filters.len(), 0);
        dv.add_filter(DataViewFilter::Contains("foo".into()));
        assert_eq!(dv.filters.len(), 1);
        dv.add_filter(DataViewFilter::Equals("bar".into()));
        assert_eq!(dv.filters.len(), 2);
    }

    #[test]
    fn dataview_filter_contains_matches_substring() {
        let f = DataViewFilter::Contains("hello".into());
        assert!(f.matches("say hello world"));
        assert!(f.matches("hello"));
    }

    #[test]
    fn dataview_filter_contains_no_match() {
        let f = DataViewFilter::Contains("xyz".into());
        assert!(!f.matches("hello world"));
        assert!(!f.matches(""));
    }

    #[test]
    fn dataview_filter_equals_exact_match() {
        let f = DataViewFilter::Equals("exact".into());
        assert!(f.matches("exact"));
    }

    #[test]
    fn dataview_filter_equals_no_match() {
        let f = DataViewFilter::Equals("exact".into());
        assert!(!f.matches("not exact"));
        assert!(!f.matches("EXACT"));
    }

    #[test]
    fn dataview_filter_greater_than_numeric() {
        let f = DataViewFilter::GreaterThan(5.0);
        assert!(f.matches("6"));
        assert!(f.matches("100.5"));
        assert!(!f.matches("5"));
        assert!(!f.matches("4.99"));
    }

    #[test]
    fn dataview_filter_less_than_numeric() {
        let f = DataViewFilter::LessThan(10.0);
        assert!(f.matches("9"));
        assert!(f.matches("0"));
        assert!(!f.matches("10"));
        assert!(!f.matches("11"));
    }

    #[test]
    fn dataview_entity_is_nomturef_not_option() {
        let entity = NomtuRef::new("dv-eid", "query", "verb");
        let dv = DataViewBlock::new(entity, "concept");
        // entity is NomtuRef (not Option) — all fields accessible directly
        assert_eq!(dv.entity.id, "dv-eid");
        assert_eq!(dv.entity.word, "query");
        assert_eq!(dv.entity.kind, "verb");
    }

    #[test]
    fn dataview_source_kind_nonempty() {
        let dv = make_dv("dv-10", "concept");
        assert!(!dv.source_kind.is_empty());
        assert_eq!(dv.source_kind, "concept");
    }

    #[test]
    fn dataview_sort_column_default_none() {
        let dv = make_dv("dv-11", "verb");
        assert!(dv.sort_column.is_none());
    }

    #[test]
    fn dataview_sort_ascending_default() {
        let dv = make_dv("dv-12", "verb");
        assert!(dv.sort_ascending);
    }

    #[test]
    fn dataview_page_size_positive() {
        let dv = make_dv("dv-13", "verb");
        assert!(dv.page_size > 0);
        assert_eq!(dv.page_size, 20);
    }

    #[test]
    fn dataview_multiple_filters_all_must_match() {
        let mut dv = make_dv("dv-14", "verb");
        dv.add_filter(DataViewFilter::Contains("nom".into()));
        dv.add_filter(DataViewFilter::Contains("lang".into()));
        // "nom-lang" satisfies both
        assert!(dv.matches_filter("nom-lang"));
        // "nom-only" satisfies first but not second
        assert!(!dv.matches_filter("nom-only"));
        // "lang-only" satisfies second but not first
        assert!(!dv.matches_filter("lang-only"));
    }

    #[test]
    fn dataview_clear_filters() {
        let mut dv = make_dv("dv-15", "verb");
        dv.add_filter(DataViewFilter::Contains("foo".into()));
        dv.add_filter(DataViewFilter::Equals("bar".into()));
        assert_eq!(dv.filters.len(), 2);
        dv.clear_filters();
        assert!(dv.filters.is_empty());
    }

    #[test]
    fn dataview_clone_equal() {
        let mut dv = make_dv("dv-16", "concept");
        dv.add_filter(DataViewFilter::Contains("test".into()));
        dv.sort_column = Some("name".to_string());
        let cloned = dv.clone();
        assert_eq!(cloned.entity.id, dv.entity.id);
        assert_eq!(cloned.source_kind, dv.source_kind);
        assert_eq!(cloned.filters.len(), dv.filters.len());
        assert_eq!(cloned.sort_column, dv.sort_column);
        assert_eq!(cloned.sort_ascending, dv.sort_ascending);
    }

    #[test]
    fn dataview_matches_filter_empty_filters_always_true() {
        let dv = make_dv("dv-17", "verb");
        // No filters — every value should pass
        assert!(dv.matches_filter("anything"));
        assert!(dv.matches_filter(""));
        assert!(dv.matches_filter("12345"));
    }

    // ── wave AI: new dataview tests ──────────────────────────────────────────────

    #[test]
    fn dataview_filter_contains_case_sensitive() {
        let f = DataViewFilter::Contains("Nom".into());
        // "Nom" (capital N) must match
        assert!(f.matches("NomLang"));
        // lowercase "nom" must NOT match "Nom"
        assert!(!f.matches("nomlang"));
    }

    #[test]
    fn dataview_no_filters_matches_all() {
        let dv = make_dv("dv-nf", "concept");
        for val in &["", "hello", "12345", "abc def"] {
            assert!(
                dv.matches_filter(val),
                "no-filter dataview must match '{val}'"
            );
        }
    }

    #[test]
    fn dataview_two_filters_and_semantics() {
        let mut dv = make_dv("dv-and", "verb");
        dv.add_filter(DataViewFilter::Contains("block".into()));
        dv.add_filter(DataViewFilter::Contains("model".into()));
        assert!(dv.matches_filter("block_model"));
        assert!(!dv.matches_filter("block_only"));
        assert!(!dv.matches_filter("model_only"));
        assert!(!dv.matches_filter("neither"));
    }

    #[test]
    fn dataview_entity_nomturef_nonempty() {
        let entity = NomtuRef::new("dv-eid-new", "query", "verb");
        let dv = DataViewBlock::new(entity, "concept");
        assert!(!dv.entity.id.is_empty());
        assert!(!dv.entity.word.is_empty());
        assert!(!dv.entity.kind.is_empty());
    }

    #[test]
    fn dataview_source_kind_preserved() {
        let dv = make_dv("dv-sk", "workflow");
        assert_eq!(dv.source_kind, "workflow");
    }

    #[test]
    fn dataview_page_size_default_positive() {
        let dv = make_dv("dv-ps", "verb");
        assert!(dv.page_size > 0);
    }

    #[test]
    fn dataview_sort_ascending_default_true() {
        let dv = make_dv("dv-sa", "concept");
        assert!(dv.sort_ascending, "sort_ascending must default to true");
    }

    #[test]
    fn dataview_add_then_clear_filters() {
        let mut dv = make_dv("dv-ac", "verb");
        dv.add_filter(DataViewFilter::Contains("x".into()));
        dv.add_filter(DataViewFilter::Equals("y".into()));
        assert_eq!(dv.filters.len(), 2);
        dv.clear_filters();
        assert!(dv.filters.is_empty());
        // After clear, matches_filter must always return true
        assert!(dv.matches_filter("anything"));
    }
}
