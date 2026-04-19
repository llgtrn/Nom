/// Lifecycle events emitted during body-only ingestion.
#[derive(Debug, Clone, PartialEq)]
pub enum IngestionEvent {
    Started { source_id: String },
    TokenExtracted { token: String },
    EntityResolved { entity_id: String, kind: String },
    EdgeCreated { from: String, to: String, edge_kind: String },
    Completed { entity_count: u32, edge_count: u32 },
    Failed { reason: String },
}

/// Body-only ingestion pipeline: extract → resolve → connect.
pub struct IngestionPipeline {
    pub source_count: u32,
    pub entity_count: u32,
    pub edge_count: u32,
    pub events: Vec<IngestionEvent>,
}

impl IngestionPipeline {
    pub fn new() -> Self {
        Self {
            source_count: 0,
            entity_count: 0,
            edge_count: 0,
            events: Vec::new(),
        }
    }

    /// Ingest a source body: tokenize, resolve entities, create edges.
    ///
    /// Returns the events emitted for this ingestion run.
    pub fn ingest(&mut self, source_id: &str, body: &str) -> Vec<IngestionEvent> {
        let mut run: Vec<IngestionEvent> = Vec::new();

        run.push(IngestionEvent::Started {
            source_id: source_id.to_string(),
        });

        // Tokenize: split on whitespace, lowercase, drop empty
        let tokens: Vec<String> = body
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();

        for token in &tokens {
            run.push(IngestionEvent::TokenExtracted {
                token: token.clone(),
            });
        }

        // Resolve unique tokens to entities (kind = "concept")
        let mut seen: Vec<String> = Vec::new();
        for token in &tokens {
            if !seen.contains(token) {
                seen.push(token.clone());
                run.push(IngestionEvent::EntityResolved {
                    entity_id: token.clone(),
                    kind: "concept".to_string(),
                });
            }
        }
        let resolved_count = seen.len() as u32;

        // Edges for adjacent token pairs (consecutive in original token list)
        let mut edge_count: u32 = 0;
        for pair in tokens.windows(2) {
            run.push(IngestionEvent::EdgeCreated {
                from: pair[0].clone(),
                to: pair[1].clone(),
                edge_kind: "adjacent".to_string(),
            });
            edge_count += 1;
        }

        run.push(IngestionEvent::Completed {
            entity_count: resolved_count,
            edge_count,
        });

        // Update pipeline-level counters
        self.source_count += 1;
        self.entity_count += resolved_count;
        self.edge_count += edge_count;
        self.events.extend(run.clone());

        run
    }

    pub fn total_events(&self) -> usize {
        self.events.len()
    }

    pub fn last_event(&self) -> Option<&IngestionEvent> {
        self.events.last()
    }
}

impl Default for IngestionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Merge / eliminate / evolve transitions for body-only lifecycle (§5).
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleTransition {
    Merge { into: String },
    Eliminate,
    Evolve { new_kind: String },
}

/// Records lifecycle transitions for entities.
pub struct LifecycleManager {
    pub transitions: Vec<(String, LifecycleTransition)>,
}

impl LifecycleManager {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
        }
    }

    pub fn record(&mut self, entity_id: &str, transition: LifecycleTransition) {
        self.transitions.push((entity_id.to_string(), transition));
    }

    pub fn transitions_for(&self, entity_id: &str) -> Vec<&LifecycleTransition> {
        self.transitions
            .iter()
            .filter(|(id, _)| id == entity_id)
            .map(|(_, t)| t)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.transitions.len()
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingestion_pipeline_new_empty() {
        let p = IngestionPipeline::new();
        assert_eq!(p.source_count, 0);
        assert_eq!(p.entity_count, 0);
        assert_eq!(p.edge_count, 0);
        assert!(p.events.is_empty());
    }

    #[test]
    fn ingestion_ingest_emits_started_and_completed() {
        let mut p = IngestionPipeline::new();
        let events = p.ingest("src-1", "hello world");
        assert!(matches!(events.first(), Some(IngestionEvent::Started { .. })));
        assert!(matches!(events.last(), Some(IngestionEvent::Completed { .. })));
    }

    #[test]
    fn ingestion_ingest_extracts_tokens() {
        let mut p = IngestionPipeline::new();
        let events = p.ingest("src-2", "foo bar baz");
        let extracted: Vec<&str> = events
            .iter()
            .filter_map(|e| {
                if let IngestionEvent::TokenExtracted { token } = e {
                    Some(token.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(extracted, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn ingestion_pipeline_entity_count() {
        let mut p = IngestionPipeline::new();
        // "foo foo bar" → 2 unique entities
        p.ingest("src-3", "foo foo bar");
        assert_eq!(p.entity_count, 2);
    }

    #[test]
    fn ingestion_last_event_is_completed() {
        let mut p = IngestionPipeline::new();
        p.ingest("src-4", "alpha beta");
        assert!(matches!(
            p.last_event(),
            Some(IngestionEvent::Completed { .. })
        ));
    }

    #[test]
    fn lifecycle_manager_record() {
        let mut lm = LifecycleManager::new();
        lm.record("ent-1", LifecycleTransition::Eliminate);
        assert_eq!(lm.count(), 1);
    }

    #[test]
    fn lifecycle_manager_transitions_for() {
        let mut lm = LifecycleManager::new();
        lm.record("ent-1", LifecycleTransition::Eliminate);
        lm.record("ent-2", LifecycleTransition::Merge { into: "ent-1".to_string() });
        lm.record("ent-1", LifecycleTransition::Evolve { new_kind: "action".to_string() });

        let for_ent1 = lm.transitions_for("ent-1");
        assert_eq!(for_ent1.len(), 2);
        assert_eq!(lm.transitions_for("ent-2").len(), 1);
    }

    #[test]
    fn lifecycle_manager_count() {
        let mut lm = LifecycleManager::new();
        assert_eq!(lm.count(), 0);
        lm.record("x", LifecycleTransition::Eliminate);
        lm.record("y", LifecycleTransition::Eliminate);
        assert_eq!(lm.count(), 2);
    }
}
