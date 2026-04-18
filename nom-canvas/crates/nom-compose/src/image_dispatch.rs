/// Capabilities that an image model can provide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelCapability {
    ObjectDetection,
    Segmentation,
    DocumentParsing,
    ImageClassification,
    DepthEstimation,
}

impl ModelCapability {
    pub fn capability_name(&self) -> &str {
        match self {
            ModelCapability::ObjectDetection => "object_detection",
            ModelCapability::Segmentation => "segmentation",
            ModelCapability::DocumentParsing => "document_parsing",
            ModelCapability::ImageClassification => "image_classification",
            ModelCapability::DepthEstimation => "depth_estimation",
        }
    }
}

/// Describes a single image model with its capability and confidence threshold.
#[derive(Debug, Clone)]
pub struct ModelDescriptor {
    pub name: String,
    pub capability: ModelCapability,
    pub confidence_threshold: f32,
    pub priority: u32,
}

impl ModelDescriptor {
    pub fn new(
        name: impl Into<String>,
        capability: ModelCapability,
        confidence_threshold: f32,
        priority: u32,
    ) -> Self {
        Self {
            name: name.into(),
            capability,
            confidence_threshold,
            priority,
        }
    }

    /// Returns true when this model's confidence threshold meets or exceeds `min_confidence`.
    pub fn is_suitable(&self, min_confidence: f32) -> bool {
        self.confidence_threshold >= min_confidence
    }
}

/// Registry of `ModelDescriptor` entries, queryable by capability.
#[derive(Debug, Default)]
pub struct ModelRegistry {
    models: Vec<ModelDescriptor>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    /// Register a new model descriptor.
    pub fn register(&mut self, model: ModelDescriptor) {
        self.models.push(model);
    }

    /// Return all descriptors whose capability matches `cap`.
    pub fn find_by_capability(&self, cap: &ModelCapability) -> Vec<&ModelDescriptor> {
        self.models
            .iter()
            .filter(|m| &m.capability == cap)
            .collect()
    }

    /// Return the highest-priority descriptor for `cap` that satisfies `min_confidence`,
    /// or `None` if no suitable model exists.
    pub fn best_for(
        &self,
        cap: &ModelCapability,
        min_confidence: f32,
    ) -> Option<&ModelDescriptor> {
        self.find_by_capability(cap)
            .into_iter()
            .filter(|m| m.is_suitable(min_confidence))
            .max_by_key(|m| m.priority)
    }

    /// Total number of registered models.
    pub fn count(&self) -> usize {
        self.models.len()
    }
}

/// A record of a single dispatch event.
#[derive(Debug, Clone)]
pub struct DispatchRecord {
    pub model_name: String,
    pub capability: String,
    pub dispatched_count: u32,
}

/// Selects the best model for a task and tracks the dispatch history.
pub struct ImageDispatcher {
    pub registry: ModelRegistry,
    pub history: Vec<DispatchRecord>,
}

impl ImageDispatcher {
    pub fn new(registry: ModelRegistry) -> Self {
        Self {
            registry,
            history: Vec::new(),
        }
    }

    /// Dispatch a request: returns the chosen model name (if any) and records the event.
    pub fn dispatch(
        &mut self,
        cap: ModelCapability,
        min_confidence: f32,
    ) -> Option<String> {
        let cap_name = cap.capability_name().to_string();
        let chosen = self.registry.best_for(&cap, min_confidence)?;
        let model_name = chosen.name.clone();

        // Update existing record or push a new one.
        if let Some(record) = self
            .history
            .iter_mut()
            .find(|r| r.model_name == model_name && r.capability == cap_name)
        {
            record.dispatched_count += 1;
        } else {
            self.history.push(DispatchRecord {
                model_name: model_name.clone(),
                capability: cap_name,
                dispatched_count: 1,
            });
        }

        Some(model_name)
    }

    /// Total number of distinct dispatch records accumulated.
    pub fn dispatch_count(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod image_dispatch_tests {
    use super::*;

    // 1. ModelDescriptor::is_suitable returns true when threshold >= min_confidence
    #[test]
    fn model_descriptor_is_suitable_true() {
        let desc = ModelDescriptor::new("model-a", ModelCapability::ObjectDetection, 0.85, 1);
        assert!(desc.is_suitable(0.80), "0.85 >= 0.80 must be suitable");
        assert!(desc.is_suitable(0.85), "exact match must be suitable");
    }

    // 2. ModelDescriptor::is_suitable returns false when threshold < min_confidence
    #[test]
    fn model_descriptor_is_suitable_false() {
        let desc = ModelDescriptor::new("model-b", ModelCapability::Segmentation, 0.60, 1);
        assert!(!desc.is_suitable(0.70), "0.60 < 0.70 must not be suitable");
    }

    // 3. ModelRegistry::register_and_count
    #[test]
    fn model_registry_register_and_count() {
        let mut registry = ModelRegistry::new();
        assert_eq!(registry.count(), 0);
        registry.register(ModelDescriptor::new(
            "m1",
            ModelCapability::DocumentParsing,
            0.9,
            10,
        ));
        registry.register(ModelDescriptor::new(
            "m2",
            ModelCapability::ImageClassification,
            0.75,
            5,
        ));
        assert_eq!(registry.count(), 2);
    }

    // 4. ModelRegistry::find_by_capability returns only matching entries
    #[test]
    fn model_registry_find_by_capability() {
        let mut registry = ModelRegistry::new();
        registry.register(ModelDescriptor::new(
            "det-1",
            ModelCapability::ObjectDetection,
            0.8,
            1,
        ));
        registry.register(ModelDescriptor::new(
            "seg-1",
            ModelCapability::Segmentation,
            0.7,
            1,
        ));
        registry.register(ModelDescriptor::new(
            "det-2",
            ModelCapability::ObjectDetection,
            0.9,
            2,
        ));

        let results = registry.find_by_capability(&ModelCapability::ObjectDetection);
        assert_eq!(results.len(), 2, "must find exactly 2 ObjectDetection models");
        assert!(results.iter().all(|m| m.capability == ModelCapability::ObjectDetection));
    }

    // 5. ModelRegistry::best_for returns highest-priority suitable model
    #[test]
    fn model_registry_best_for_highest_priority() {
        let mut registry = ModelRegistry::new();
        registry.register(ModelDescriptor::new(
            "low-prio",
            ModelCapability::DepthEstimation,
            0.8,
            1,
        ));
        registry.register(ModelDescriptor::new(
            "high-prio",
            ModelCapability::DepthEstimation,
            0.85,
            10,
        ));

        let best = registry.best_for(&ModelCapability::DepthEstimation, 0.75);
        assert!(best.is_some(), "must find a suitable model");
        assert_eq!(best.unwrap().name, "high-prio");
    }

    // 6. ModelRegistry::best_for returns None when registry is empty
    #[test]
    fn model_registry_best_for_none_when_empty() {
        let registry = ModelRegistry::new();
        let result = registry.best_for(&ModelCapability::ImageClassification, 0.5);
        assert!(result.is_none(), "empty registry must return None");
    }

    // 7. ImageDispatcher::dispatch records dispatch events correctly
    #[test]
    fn image_dispatcher_dispatch_records() {
        let mut registry = ModelRegistry::new();
        registry.register(ModelDescriptor::new(
            "classify-model",
            ModelCapability::ImageClassification,
            0.9,
            5,
        ));

        let mut dispatcher = ImageDispatcher::new(registry);
        let result = dispatcher.dispatch(ModelCapability::ImageClassification, 0.8);
        assert_eq!(result, Some("classify-model".to_string()));
        assert_eq!(dispatcher.history.len(), 1);
        assert_eq!(dispatcher.history[0].model_name, "classify-model");
        assert_eq!(dispatcher.history[0].dispatched_count, 1);

        // Second dispatch increments the count on the same record.
        dispatcher.dispatch(ModelCapability::ImageClassification, 0.8);
        assert_eq!(dispatcher.history.len(), 1);
        assert_eq!(dispatcher.history[0].dispatched_count, 2);
    }

    // 8. ImageDispatcher::dispatch_count reflects number of distinct records
    #[test]
    fn image_dispatcher_dispatch_count() {
        let mut registry = ModelRegistry::new();
        registry.register(ModelDescriptor::new(
            "seg-model",
            ModelCapability::Segmentation,
            0.8,
            3,
        ));
        registry.register(ModelDescriptor::new(
            "depth-model",
            ModelCapability::DepthEstimation,
            0.75,
            3,
        ));

        let mut dispatcher = ImageDispatcher::new(registry);
        assert_eq!(dispatcher.dispatch_count(), 0);

        dispatcher.dispatch(ModelCapability::Segmentation, 0.7);
        assert_eq!(dispatcher.dispatch_count(), 1);

        dispatcher.dispatch(ModelCapability::DepthEstimation, 0.7);
        assert_eq!(dispatcher.dispatch_count(), 2);

        // Repeat call does NOT add a new record.
        dispatcher.dispatch(ModelCapability::Segmentation, 0.7);
        assert_eq!(dispatcher.dispatch_count(), 2);
    }

    // 9. ModelCapability::capability_name returns expected strings
    #[test]
    fn model_capability_capability_name() {
        assert_eq!(
            ModelCapability::ObjectDetection.capability_name(),
            "object_detection"
        );
        assert_eq!(
            ModelCapability::Segmentation.capability_name(),
            "segmentation"
        );
        assert_eq!(
            ModelCapability::DocumentParsing.capability_name(),
            "document_parsing"
        );
        assert_eq!(
            ModelCapability::ImageClassification.capability_name(),
            "image_classification"
        );
        assert_eq!(
            ModelCapability::DepthEstimation.capability_name(),
            "depth_estimation"
        );
    }
}
