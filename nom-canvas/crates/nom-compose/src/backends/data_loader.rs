#![deny(unsafe_code)]

/// DataLoader backend — batch + streaming data ingestion pattern.
/// No external dependency (stub implementation).

#[derive(Debug, Clone, PartialEq)]
pub enum DataSourceKind {
    File { path: String },
    InMemory { name: String },
    Database { uri: String },
    Stream { endpoint: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadStrategy {
    Eager,
    Lazy,
    Streaming,
}

#[derive(Debug, Clone)]
pub struct DataLoaderConfig {
    pub source: DataSourceKind,
    pub strategy: LoadStrategy,
    pub batch_size: usize,
    pub prefetch_count: usize,
    pub shuffle: bool,
}

impl DataLoaderConfig {
    pub fn new(source: DataSourceKind) -> Self {
        Self {
            source,
            strategy: LoadStrategy::Eager,
            batch_size: 32,
            prefetch_count: 2,
            shuffle: false,
        }
    }

    pub fn with_strategy(mut self, s: LoadStrategy) -> Self {
        self.strategy = s;
        self
    }

    /// Sets batch_size; enforces a minimum of 1.
    pub fn with_batch_size(mut self, n: usize) -> Self {
        self.batch_size = n.max(1);
        self
    }

    pub fn with_shuffle(mut self) -> Self {
        self.shuffle = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct DataBatch {
    pub index: usize,
    pub items: Vec<String>,
    pub is_last: bool,
}

impl DataBatch {
    pub fn new(index: usize, items: Vec<String>) -> Self {
        Self {
            index,
            items,
            is_last: false,
        }
    }

    pub fn mark_last(mut self) -> Self {
        self.is_last = true;
        self
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug)]
pub struct DataLoader {
    pub config: DataLoaderConfig,
    pub batches_served: usize,
    pub total_items: usize,
}

impl DataLoader {
    pub fn new(config: DataLoaderConfig) -> Self {
        Self {
            config,
            batches_served: 0,
            total_items: 0,
        }
    }

    /// Wraps `items` in a `DataBatch`, increments counters, and returns the batch.
    pub fn next_batch(&mut self, items: Vec<String>) -> DataBatch {
        let b = DataBatch::new(self.batches_served, items);
        self.batches_served += 1;
        self.total_items += b.items.len();
        b
    }

    pub fn reset(&mut self) {
        self.batches_served = 0;
        self.total_items = 0;
    }

    pub fn is_streaming(&self) -> bool {
        self.config.strategy == LoadStrategy::Streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_new_and_with_strategy() {
        let cfg = DataLoaderConfig::new(DataSourceKind::InMemory {
            name: "test".into(),
        })
        .with_strategy(LoadStrategy::Lazy);

        assert_eq!(cfg.strategy, LoadStrategy::Lazy);
        assert_eq!(cfg.batch_size, 32);
        assert_eq!(cfg.prefetch_count, 2);
        assert!(!cfg.shuffle);
    }

    #[test]
    fn config_batch_size_minimum_one() {
        let cfg = DataLoaderConfig::new(DataSourceKind::InMemory {
            name: "t".into(),
        })
        .with_batch_size(0);

        assert_eq!(cfg.batch_size, 1, "batch_size must be at least 1");
    }

    #[test]
    fn data_batch_new_and_item_count() {
        let batch = DataBatch::new(0, vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(batch.index, 0);
        assert_eq!(batch.item_count(), 3);
        assert!(!batch.is_last);
        assert!(!batch.is_empty());
    }

    #[test]
    fn data_batch_mark_last_and_is_empty() {
        let batch = DataBatch::new(1, vec![]).mark_last();
        assert!(batch.is_last);
        assert!(batch.is_empty());
        assert_eq!(batch.item_count(), 0);
    }

    #[test]
    fn data_loader_new_and_next_batch() {
        let cfg = DataLoaderConfig::new(DataSourceKind::File {
            path: "/data/set.csv".into(),
        });
        let mut loader = DataLoader::new(cfg);
        let batch = loader.next_batch(vec!["x".into(), "y".into()]);
        assert_eq!(batch.index, 0);
        assert_eq!(batch.item_count(), 2);
        assert_eq!(loader.batches_served, 1);
        assert_eq!(loader.total_items, 2);
    }

    #[test]
    fn data_loader_batches_served_increments() {
        let cfg = DataLoaderConfig::new(DataSourceKind::Database {
            uri: "db://localhost/nom".into(),
        });
        let mut loader = DataLoader::new(cfg);
        loader.next_batch(vec!["a".into()]);
        loader.next_batch(vec!["b".into(), "c".into()]);
        loader.next_batch(vec![]);
        assert_eq!(loader.batches_served, 3);
        assert_eq!(loader.total_items, 3);
    }

    #[test]
    fn data_loader_is_streaming_and_reset() {
        let cfg = DataLoaderConfig::new(DataSourceKind::Stream {
            endpoint: "ws://stream.nom/feed".into(),
        })
        .with_strategy(LoadStrategy::Streaming);
        let mut loader = DataLoader::new(cfg);
        assert!(loader.is_streaming());

        loader.next_batch(vec!["event1".into()]);
        loader.next_batch(vec!["event2".into()]);
        assert_eq!(loader.batches_served, 2);

        loader.reset();
        assert_eq!(loader.batches_served, 0);
        assert_eq!(loader.total_items, 0);
        assert!(loader.is_streaming(), "strategy unchanged after reset");
    }
}
