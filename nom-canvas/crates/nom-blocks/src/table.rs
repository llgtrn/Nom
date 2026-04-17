//! Table (database) block schema with multi-view support.
#![deny(unsafe_code)]

use crate::block_schema::{BlockSchema, Role};
use crate::flavour::{NOTE, TABLE};

pub type ColumnId = String;
pub type RowId = String;

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnKind {
    Text,
    Number,
    Select { options: Vec<String> },
    MultiSelect { options: Vec<String> },
    Date,
    Checkbox,
    Relation { target_table: String },
    RichText,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub id: ColumnId,
    pub name: String,
    pub kind: ColumnKind,
    pub width_px: u32,
}

impl Column {
    pub fn text(id: impl Into<ColumnId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: ColumnKind::Text,
            width_px: 160,
        }
    }

    pub fn number(id: impl Into<ColumnId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: ColumnKind::Number,
            width_px: 100,
        }
    }

    pub fn select(
        id: impl Into<ColumnId>,
        name: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: ColumnKind::Select { options },
            width_px: 140,
        }
    }

    pub fn is_selectable(&self) -> bool {
        matches!(
            self.kind,
            ColumnKind::Select { .. } | ColumnKind::MultiSelect { .. }
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    Text(String),
    Number(f64),
    Select(String),
    MultiSelect(Vec<String>),
    Date(i64),
    Checkbox(bool),
    Relation(RowId),
    Empty,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: RowId,
    pub cells: Vec<(ColumnId, CellValue)>,
}

impl Row {
    pub fn new(id: impl Into<RowId>) -> Self {
        Self {
            id: id.into(),
            cells: Vec::new(),
        }
    }

    pub fn with_cell(mut self, col: impl Into<ColumnId>, value: CellValue) -> Self {
        self.cells.push((col.into(), value));
        self
    }

    pub fn cell(&self, col: &str) -> Option<&CellValue> {
        self.cells.iter().find(|(k, _)| k == col).map(|(_, v)| v)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewKind {
    Grid,
    Kanban { group_by: ColumnId },
    Calendar { date_col: ColumnId },
}

#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub id: String,
    pub name: String,
    pub kind: ViewKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableProps {
    pub title: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub views: Vec<View>,
}

impl TableProps {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            columns: Vec::new(),
            rows: Vec::new(),
            views: vec![View {
                id: "grid-1".to_string(),
                name: "Grid".to_string(),
                kind: ViewKind::Grid,
            }],
        }
    }

    pub fn add_column(&mut self, col: Column) {
        self.columns.push(col);
    }

    pub fn add_row(&mut self, row: Row) {
        self.rows.push(row);
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn find_column(&self, id: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.id == id)
    }
}

pub fn table_schema() -> BlockSchema {
    BlockSchema {
        flavour: TABLE,
        version: 1,
        role: Role::Content,
        parents: &[NOTE],
        children: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_text_default_width_160() {
        let col = Column::text("c1", "Name");
        assert_eq!(col.width_px, 160);
        assert_eq!(col.id, "c1");
        assert_eq!(col.name, "Name");
    }

    #[test]
    fn column_number_default_width_100() {
        let col = Column::number("c2", "Score");
        assert_eq!(col.width_px, 100);
    }

    #[test]
    fn column_select_default_width_140() {
        let col = Column::select("c3", "Status", vec!["Todo".into(), "Done".into()]);
        assert_eq!(col.width_px, 140);
    }

    #[test]
    fn column_select_is_selectable_true() {
        let col = Column::select("c3", "Status", vec!["A".into()]);
        assert!(col.is_selectable());
    }

    #[test]
    fn column_multi_select_is_selectable_true() {
        let col = Column {
            id: "c4".into(),
            name: "Tags".into(),
            kind: ColumnKind::MultiSelect {
                options: vec!["x".into()],
            },
            width_px: 140,
        };
        assert!(col.is_selectable());
    }

    #[test]
    fn column_text_is_selectable_false() {
        let col = Column::text("c1", "Name");
        assert!(!col.is_selectable());
    }

    #[test]
    fn row_new_empty_cells() {
        let row = Row::new("r1");
        assert_eq!(row.id, "r1");
        assert!(row.cells.is_empty());
    }

    #[test]
    fn row_with_cell_chains_and_lookup() {
        let row = Row::new("r1")
            .with_cell("c1", CellValue::Text("hello".into()))
            .with_cell("c2", CellValue::Number(42.0));

        assert_eq!(row.cell("c1"), Some(&CellValue::Text("hello".into())));
        assert_eq!(row.cell("c2"), Some(&CellValue::Number(42.0)));
        assert_eq!(row.cell("c3"), None);
    }

    #[test]
    fn table_props_new_has_default_grid_view() {
        let t = TableProps::new("My Table");
        assert_eq!(t.views.len(), 1);
        assert_eq!(t.views[0].id, "grid-1");
        assert_eq!(t.views[0].kind, ViewKind::Grid);
    }

    #[test]
    fn add_column_increments_column_count() {
        let mut t = TableProps::new("T");
        assert_eq!(t.column_count(), 0);
        t.add_column(Column::text("c1", "Name"));
        assert_eq!(t.column_count(), 1);
        t.add_column(Column::number("c2", "Age"));
        assert_eq!(t.column_count(), 2);
    }

    #[test]
    fn add_row_increments_row_count() {
        let mut t = TableProps::new("T");
        assert_eq!(t.row_count(), 0);
        t.add_row(Row::new("r1"));
        assert_eq!(t.row_count(), 1);
    }

    #[test]
    fn find_column_hit_and_miss() {
        let mut t = TableProps::new("T");
        t.add_column(Column::text("col-a", "Alpha"));
        assert!(t.find_column("col-a").is_some());
        assert_eq!(t.find_column("col-a").unwrap().name, "Alpha");
        assert!(t.find_column("col-z").is_none());
    }

    #[test]
    fn table_schema_role_is_content() {
        let s = table_schema();
        assert_eq!(s.role, Role::Content);
        assert_eq!(s.flavour, TABLE);
        assert_eq!(s.version, 1);
        assert_eq!(s.parents, &[NOTE]);
        assert!(s.children.is_empty());
    }
}
