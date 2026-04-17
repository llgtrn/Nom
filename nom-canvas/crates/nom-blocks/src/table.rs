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
}
