#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use crate::slot::SlotValue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableColumn {
    pub id: String,
    pub name: String,
    pub col_type: String,
    pub width: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableBlock {
    pub entity: NomtuRef,
    pub columns: Vec<TableColumn>,
    pub rows: Vec<Vec<SlotValue>>,
}

impl TableBlock {
    pub fn new(entity: NomtuRef, columns: Vec<TableColumn>) -> Self {
        Self {
            entity,
            columns,
            rows: Vec::new(),
        }
    }
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
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    pub fn col_count(&self) -> usize {
        self.columns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_row_validation() {
        let entity = NomtuRef::new("id", "w", "k");
        let cols = vec![
            TableColumn {
                id: "c1".into(),
                name: "Name".into(),
                col_type: "text".into(),
                width: None,
            },
            TableColumn {
                id: "c2".into(),
                name: "Age".into(),
                col_type: "number".into(),
                width: None,
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
        assert_eq!(t.row_count(), 1);
    }

    fn make_two_col_table() -> TableBlock {
        let entity = NomtuRef::new("tbl-00", "tabulate", "verb");
        let cols = vec![
            TableColumn {
                id: "c1".into(),
                name: "Key".into(),
                col_type: "text".into(),
                width: Some(120.0),
            },
            TableColumn {
                id: "c2".into(),
                name: "Value".into(),
                col_type: "text".into(),
                width: None,
            },
        ];
        TableBlock::new(entity, cols)
    }

    #[test]
    fn table_col_count() {
        let t = make_two_col_table();
        assert_eq!(t.col_count(), 2);
    }

    #[test]
    fn table_new_has_zero_rows() {
        let t = make_two_col_table();
        assert_eq!(t.row_count(), 0);
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
        assert_eq!(t.row_count(), 5);
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
        assert_eq!(t.row_count(), 0);
    }

    #[test]
    fn table_add_row_empty_when_no_cols() {
        let entity = NomtuRef::new("tbl-01", "tabulate", "verb");
        let mut t = TableBlock::new(entity, vec![]);
        assert!(t.add_row(vec![]).is_ok());
        assert_eq!(t.row_count(), 1);
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
}
