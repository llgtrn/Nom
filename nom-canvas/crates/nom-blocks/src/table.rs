//! Table block: columnar data with typed rows.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use crate::slot::SlotValue;
use serde::{Deserialize, Serialize};

/// A column definition for a [`TableBlock`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableColumn {
    /// Unique column identifier.
    pub id: String,
    /// Display name for the column.
    pub name: String,
    /// Column data type (e.g. `"text"`, `"number"`, `"boolean"`).
    pub col_type: String,
    /// Optional fractional width (0.0–1.0 or absolute value).
    pub width: Option<f32>,
    /// Pixel width of the column.
    pub width_px: u32,
}

/// A single typed row in a [`TableBlock`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TableRow {
    /// Unique row identifier.
    pub id: String,
    /// Cell values, one per column.
    pub cells: Vec<String>,
}

/// A block holding tabular data with named columns and typed rows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableBlock {
    /// DB entity reference (NON-OPTIONAL).
    pub entity: NomtuRef,
    /// Column definitions.
    pub columns: Vec<TableColumn>,
    /// Raw slot-value rows (legacy path).
    pub rows: Vec<Vec<SlotValue>>,
    /// Typed string-cell rows.
    pub typed_rows: Vec<TableRow>,
}

impl TableBlock {
    /// Construct a new, empty [`TableBlock`] with the given entity and columns.
    pub fn new(entity: NomtuRef, columns: Vec<TableColumn>) -> Self {
        Self {
            entity,
            columns,
            rows: Vec::new(),
            typed_rows: Vec::new(),
        }
    }
    /// Append a raw row of [`SlotValue`]s. Returns `Err` if the cell count doesn't match columns.
    pub fn add_row(&mut self, row: Vec<SlotValue>) -> Result<(), String> {
        if row.len() != self.columns.len() {
            return Err(format!(
                "Row has {} cells, expected {}",
                row.len(),
                self.columns.len()
            ));
        }
        self.rows.push(row);
        Ok(())
    }
    /// Append a column definition.
    pub fn add_column(&mut self, col: TableColumn) {
        self.columns.push(col);
    }
    /// Append a typed row.
    pub fn add_typed_row(&mut self, row: TableRow) {
        self.typed_rows.push(row);
    }
    /// Number of typed rows.
    pub fn row_count(&self) -> usize {
        self.typed_rows.len()
    }
    /// Number of columns.
    pub fn col_count(&self) -> usize {
        self.columns.len()
    }
    /// Return the cell string at `(row_idx, col_idx)`, or `None` if out of range.
    pub fn cell(&self, row_idx: usize, col_idx: usize) -> Option<&str> {
        self.typed_rows
            .get(row_idx)
            .and_then(|r| r.cells.get(col_idx))
            .map(|s| s.as_str())
    }
    /// Remove all typed rows.
    pub fn clear_typed_rows(&mut self) {
        self.typed_rows.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(entity_id: &str) -> TableBlock {
        let entity = NomtuRef::new(entity_id, "tabulate", "verb");
        TableBlock::new(entity, vec![])
    }

    fn make_col(name: &str, col_type: &str, width_px: u32) -> TableColumn {
        TableColumn {
            id: format!("col-{name}"),
            name: name.into(),
            col_type: col_type.into(),
            width: None,
            width_px,
        }
    }

    fn make_row(id: &str, cells: &[&str]) -> TableRow {
        TableRow {
            id: id.into(),
            cells: cells.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ── legacy tests (preserved) ──────────────────────────────────────────────

    fn make_two_col_table() -> TableBlock {
        let entity = NomtuRef::new("tbl-00", "tabulate", "verb");
        let cols = vec![
            TableColumn {
                id: "c1".into(),
                name: "Key".into(),
                col_type: "text".into(),
                width: Some(120.0),
                width_px: 120,
            },
            TableColumn {
                id: "c2".into(),
                name: "Value".into(),
                col_type: "text".into(),
                width: None,
                width_px: 100,
            },
        ];
        TableBlock::new(entity, cols)
    }

    #[test]
    fn table_row_validation() {
        let entity = NomtuRef::new("id", "w", "k");
        let cols = vec![
            TableColumn {
                id: "c1".into(),
                name: "Name".into(),
                col_type: "text".into(),
                width: None,
                width_px: 100,
            },
            TableColumn {
                id: "c2".into(),
                name: "Age".into(),
                col_type: "number".into(),
                width: None,
                width_px: 80,
            },
        ];
        let mut t = TableBlock::new(entity, cols);
        assert!(t
            .add_row(vec![
                SlotValue::Text("Alice".into()),
                SlotValue::Number(30.0)
            ])
            .is_ok());
        assert!(t.add_row(vec![SlotValue::Text("Bob".into())]).is_err());
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn table_col_count() {
        let t = make_two_col_table();
        assert_eq!(t.col_count(), 2);
    }

    #[test]
    fn table_new_has_zero_rows() {
        let t = make_two_col_table();
        assert_eq!(t.rows.len(), 0);
        assert!(t.rows.is_empty());
    }

    #[test]
    fn table_add_multiple_rows() {
        let mut t = make_two_col_table();
        for i in 0..5u32 {
            let row = vec![
                SlotValue::Text(format!("key-{i}")),
                SlotValue::Text(format!("val-{i}")),
            ];
            assert!(t.add_row(row).is_ok());
        }
        assert_eq!(t.rows.len(), 5);
    }

    #[test]
    fn table_add_row_too_many_cells_errors() {
        let mut t = make_two_col_table();
        let row = vec![
            SlotValue::Text("a".into()),
            SlotValue::Text("b".into()),
            SlotValue::Text("extra".into()),
        ];
        assert!(t.add_row(row).is_err());
        assert_eq!(t.rows.len(), 0);
    }

    #[test]
    fn table_add_row_empty_when_no_cols() {
        let entity = NomtuRef::new("tbl-01", "tabulate", "verb");
        let mut t = TableBlock::new(entity, vec![]);
        assert!(t.add_row(vec![]).is_ok());
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn table_column_width_optional() {
        let t = make_two_col_table();
        assert_eq!(t.columns[0].width, Some(120.0));
        assert!(t.columns[1].width.is_none());
    }

    #[test]
    fn table_entity_preserved() {
        let entity = NomtuRef::new("tbl-ent", "organize", "verb");
        let t = TableBlock::new(entity, vec![]);
        assert_eq!(t.entity.id, "tbl-ent");
        assert_eq!(t.entity.word, "organize");
    }

    #[test]
    fn table_row_data_accessible() {
        let mut t = make_two_col_table();
        t.add_row(vec![
            SlotValue::Text("hello".into()),
            SlotValue::Text("world".into()),
        ])
        .unwrap();
        assert_eq!(t.rows[0][0].as_text(), Some("hello"));
        assert_eq!(t.rows[0][1].as_text(), Some("world"));
    }

    // ── wave AH: new tests ────────────────────────────────────────────────────

    #[test]
    fn table_new_has_zero_rows_zero_cols() {
        let t = make_table("tbl-01");
        assert_eq!(t.row_count(), 0);
        assert_eq!(t.col_count(), 0);
    }

    #[test]
    fn table_add_column_increments_col_count() {
        let mut t = make_table("tbl-02");
        assert_eq!(t.col_count(), 0);
        t.add_column(make_col("Name", "text", 100));
        assert_eq!(t.col_count(), 1);
        t.add_column(make_col("Score", "number", 80));
        assert_eq!(t.col_count(), 2);
    }

    #[test]
    fn table_add_row_increments_row_count() {
        let mut t = make_table("tbl-03");
        t.add_column(make_col("Name", "text", 100));
        assert_eq!(t.row_count(), 0);
        t.add_typed_row(make_row("r1", &["Alice"]));
        assert_eq!(t.row_count(), 1);
        t.add_typed_row(make_row("r2", &["Bob"]));
        assert_eq!(t.row_count(), 2);
    }

    #[test]
    fn table_cell_at_valid_index_returns_some() {
        let mut t = make_table("tbl-04");
        t.add_column(make_col("A", "text", 100));
        t.add_column(make_col("B", "text", 100));
        t.add_typed_row(make_row("r1", &["hello", "world"]));
        assert_eq!(t.cell(0, 0), Some("hello"));
        assert_eq!(t.cell(0, 1), Some("world"));
    }

    #[test]
    fn table_cell_at_invalid_row_returns_none() {
        let mut t = make_table("tbl-05");
        t.add_column(make_col("A", "text", 100));
        t.add_typed_row(make_row("r1", &["val"]));
        assert_eq!(t.cell(1, 0), None);
        assert_eq!(t.cell(99, 0), None);
    }

    #[test]
    fn table_cell_at_invalid_col_returns_none() {
        let mut t = make_table("tbl-06");
        t.add_column(make_col("A", "text", 100));
        t.add_typed_row(make_row("r1", &["val"]));
        assert_eq!(t.cell(0, 1), None);
        assert_eq!(t.cell(0, 99), None);
    }

    #[test]
    fn table_column_types_text_number_bool_date() {
        let mut t = make_table("tbl-07");
        t.add_column(make_col("Col1", "text", 100));
        t.add_column(make_col("Col2", "number", 80));
        t.add_column(make_col("Col3", "boolean", 60));
        t.add_column(make_col("Col4", "date", 120));
        assert_eq!(t.columns[0].col_type, "text");
        assert_eq!(t.columns[1].col_type, "number");
        assert_eq!(t.columns[2].col_type, "boolean");
        assert_eq!(t.columns[3].col_type, "date");
    }

    #[test]
    fn table_column_width_positive() {
        let col = make_col("Name", "text", 150);
        assert!(col.width_px > 0);
        assert_eq!(col.width_px, 150);
    }

    #[test]
    fn table_row_cells_count_matches_columns() {
        let mut t = make_table("tbl-09");
        t.add_column(make_col("A", "text", 100));
        t.add_column(make_col("B", "text", 100));
        t.add_column(make_col("C", "text", 100));
        let row = make_row("r1", &["x", "y", "z"]);
        assert_eq!(row.cells.len(), t.col_count());
        t.add_typed_row(row);
    }

    #[test]
    fn table_multiple_columns_multiple_rows() {
        let mut t = make_table("tbl-10");
        t.add_column(make_col("A", "text", 100));
        t.add_column(make_col("B", "number", 80));
        t.add_typed_row(make_row("r1", &["alpha", "1"]));
        t.add_typed_row(make_row("r2", &["beta", "2"]));
        t.add_typed_row(make_row("r3", &["gamma", "3"]));
        assert_eq!(t.col_count(), 2);
        assert_eq!(t.row_count(), 3);
        assert_eq!(t.cell(1, 0), Some("beta"));
        assert_eq!(t.cell(2, 1), Some("3"));
    }

    #[test]
    fn table_entity_is_nomturef_not_option() {
        let entity = NomtuRef::new("eid", "tabulate", "verb");
        let t = TableBlock::new(entity, vec![]);
        // entity is NomtuRef (not Option<NomtuRef>) — fields accessible directly
        assert_eq!(t.entity.id, "eid");
        assert_eq!(t.entity.word, "tabulate");
        assert_eq!(t.entity.kind, "verb");
    }

    #[test]
    fn table_clear_rows_empties_table() {
        let mut t = make_table("tbl-12");
        t.add_column(make_col("A", "text", 100));
        t.add_typed_row(make_row("r1", &["x"]));
        t.add_typed_row(make_row("r2", &["y"]));
        assert_eq!(t.row_count(), 2);
        t.clear_typed_rows();
        assert_eq!(t.row_count(), 0);
    }

    #[test]
    fn table_column_name_nonempty() {
        let col = make_col("MyColumn", "text", 100);
        assert!(!col.name.is_empty());
        assert_eq!(col.name, "MyColumn");
    }

    #[test]
    fn table_row_id_unique() {
        let r1 = make_row("row-001", &["a"]);
        let r2 = make_row("row-002", &["b"]);
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn table_clone_equal() {
        let mut t = make_table("tbl-15");
        t.add_column(make_col("Name", "text", 120));
        t.add_typed_row(make_row("r1", &["Alice"]));
        let cloned = t.clone();
        assert_eq!(cloned.entity.id, t.entity.id);
        assert_eq!(cloned.col_count(), t.col_count());
        assert_eq!(cloned.row_count(), t.row_count());
        assert_eq!(cloned.cell(0, 0), t.cell(0, 0));
    }

    #[test]
    fn table_col_count_after_multiple_adds() {
        let mut t = make_table("tbl-16");
        for i in 0..7 {
            t.add_column(make_col(&format!("col-{i}"), "text", 100));
        }
        assert_eq!(t.col_count(), 7);
    }

    #[test]
    fn table_row_count_after_multiple_adds() {
        let mut t = make_table("tbl-17");
        t.add_column(make_col("A", "text", 100));
        for i in 0..10 {
            t.add_typed_row(make_row(&format!("r{i}"), &[&format!("val-{i}")]));
        }
        assert_eq!(t.row_count(), 10);
    }

    #[test]
    fn table_cell_update_if_supported() {
        let mut t = make_table("tbl-18");
        t.add_column(make_col("A", "text", 100));
        t.add_typed_row(make_row("r1", &["original"]));
        assert_eq!(t.cell(0, 0), Some("original"));
        // Update via direct access to typed_rows
        t.typed_rows[0].cells[0] = "updated".to_string();
        assert_eq!(t.cell(0, 0), Some("updated"));
    }

    // ── wave AI: new table tests ─────────────────────────────────────────────────

    #[test]
    fn table_row_id_is_stable() {
        let r = make_row("stable-id-42", &["val"]);
        // Same row retains its ID across field reads
        assert_eq!(r.id, "stable-id-42");
        let id_copy = r.id.clone();
        assert_eq!(r.id, id_copy);
    }

    #[test]
    fn table_empty_row_has_empty_cells() {
        let r = make_row("empty-row", &[]);
        assert!(r.cells.is_empty());
    }

    #[test]
    fn table_add_two_columns_two_rows() {
        let mut t = make_table("tbl-2c2r");
        t.add_column(make_col("Name", "text", 120));
        t.add_column(make_col("Score", "number", 80));
        t.add_typed_row(make_row("r1", &["Alice", "95"]));
        t.add_typed_row(make_row("r2", &["Bob", "87"]));
        assert_eq!(t.col_count(), 2);
        assert_eq!(t.row_count(), 2);
    }

    #[test]
    fn table_cell_first_row_first_col() {
        let mut t = make_table("tbl-frf");
        t.add_column(make_col("X", "text", 100));
        t.add_column(make_col("Y", "text", 100));
        t.add_typed_row(make_row("r1", &["first", "second"]));
        assert_eq!(t.cell(0, 0), Some("first"));
    }

    #[test]
    fn table_cell_last_row_last_col() {
        let mut t = make_table("tbl-lrl");
        t.add_column(make_col("A", "text", 100));
        t.add_column(make_col("B", "text", 100));
        t.add_typed_row(make_row("r1", &["a0", "b0"]));
        t.add_typed_row(make_row("r2", &["a1", "b1"]));
        assert_eq!(t.cell(1, 1), Some("b1"));
    }

    #[test]
    fn table_entity_nomturef_word_nonempty() {
        let entity = NomtuRef::new("tbl-eid", "tabulate", "verb");
        let t = TableBlock::new(entity, vec![]);
        assert!(!t.entity.word.is_empty());
        assert_eq!(t.entity.word, "tabulate");
    }

    #[test]
    fn table_entity_nomturef_kind_nonempty() {
        let entity = NomtuRef::new("tbl-eid-2", "organize", "verb");
        let t = TableBlock::new(entity, vec![]);
        assert!(!t.entity.kind.is_empty());
        assert_eq!(t.entity.kind, "verb");
    }
}
