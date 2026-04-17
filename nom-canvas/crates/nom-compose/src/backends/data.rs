#![deny(unsafe_code)]
use nom_blocks::compose::data_block::{DataBlock, ColumnSpec};
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct DataInput {
    pub entity: NomtuRef,
    pub rows: Vec<Vec<String>>,
    pub schema: Vec<ColumnSpec>,
}

pub struct DataBackend;

impl DataBackend {
    pub fn compose(input: DataInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> DataBlock {
        sink.emit(ComposeEvent::Started { backend: "data".into(), entity_id: input.entity.id.clone() });
        // Serialize rows as CSV-like bytes
        let csv: String = input.rows.iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n");
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "serializing".into() });
        let artifact_hash = store.write(csv.as_bytes());
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        let row_count = input.rows.len() as u64;
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        DataBlock {
            entity: input.entity,
            artifact_hash,
            row_count,
            schema: input.schema,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn data_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = DataInput {
            entity: NomtuRef { id: "dat1".into(), word: "table".into(), kind: "data".into() },
            rows: vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ],
            schema: vec![
                ColumnSpec { name: "name".into(), col_type: "text".into() },
                ColumnSpec { name: "age".into(), col_type: "integer".into() },
            ],
        };
        let block = DataBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.row_count, 2);
        assert_eq!(block.schema.len(), 2);
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn data_compose_csv_content() {
        let mut store = InMemoryStore::new();
        let input = DataInput {
            entity: NomtuRef { id: "dat2".into(), word: "csv".into(), kind: "data".into() },
            rows: vec![
                vec!["x".into(), "1".into()],
                vec!["y".into(), "2".into()],
            ],
            schema: vec![
                ColumnSpec { name: "label".into(), col_type: "text".into() },
                ColumnSpec { name: "val".into(), col_type: "integer".into() },
            ],
        };
        let block = DataBackend::compose(input, &mut store, &LogProgressSink);
        let content = store.read(&block.artifact_hash).unwrap();
        let csv = std::str::from_utf8(&content).unwrap();
        assert!(csv.contains("x,1"));
        assert!(csv.contains("y,2"));
    }

    #[test]
    fn data_compose_empty_rows() {
        let mut store = InMemoryStore::new();
        let input = DataInput {
            entity: NomtuRef { id: "dat3".into(), word: "empty".into(), kind: "data".into() },
            rows: vec![],
            schema: vec![],
        };
        let block = DataBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.row_count, 0);
        assert!(store.exists(&block.artifact_hash));
    }
}
