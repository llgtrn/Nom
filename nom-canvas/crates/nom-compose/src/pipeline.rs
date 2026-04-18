#![deny(unsafe_code)]

/// A single named output value produced by a pipeline component.
#[derive(Debug, Clone)]
pub struct ComponentOutput {
    pub name: String,
    pub value: String,
}

/// A self-contained processing unit in a Haystack-style pipeline.
/// Components declare their output port names and transform a list of
/// string inputs into a list of [`ComponentOutput`] values.
pub trait PipelineComponent: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, inputs: &[String]) -> Vec<ComponentOutput>;
    fn output_names(&self) -> Vec<&str>;
}

// ---------------------------------------------------------------------------
// Concrete components
// ---------------------------------------------------------------------------

/// Splits the first input string into fixed-size character chunks.
pub struct TextSplitter {
    pub chunk_size: usize,
}

impl PipelineComponent for TextSplitter {
    fn name(&self) -> &str {
        "text_splitter"
    }

    fn run(&self, inputs: &[String]) -> Vec<ComponentOutput> {
        let text = inputs.first().map(String::as_str).unwrap_or("");
        let chunk_size = self.chunk_size.max(1);
        let chunks: Vec<String> = text
            .chars()
            .collect::<Vec<char>>()
            .chunks(chunk_size)
            .map(|c| c.iter().collect())
            .collect();
        let joined = chunks.join("|");
        vec![ComponentOutput {
            name: "chunks".to_string(),
            value: joined,
        }]
    }

    fn output_names(&self) -> Vec<&str> {
        vec!["chunks"]
    }
}

/// Returns the top-k mock retrieval results for the first input query.
pub struct DocumentRetriever {
    pub top_k: usize,
}

impl PipelineComponent for DocumentRetriever {
    fn name(&self) -> &str {
        "document_retriever"
    }

    fn run(&self, inputs: &[String]) -> Vec<ComponentOutput> {
        let query = inputs.first().map(String::as_str).unwrap_or("");
        let docs: Vec<String> = (0..self.top_k)
            .map(|i| format!("doc_{i}:{query}"))
            .collect();
        vec![ComponentOutput {
            name: "documents".to_string(),
            value: docs.join("|"),
        }]
    }

    fn output_names(&self) -> Vec<&str> {
        vec!["documents"]
    }
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// A sequential pipeline that threads outputs of one component into the next.
/// Follows the Haystack-pattern: each component receives the previous
/// component's output values as its inputs.
pub struct ComponentPipeline {
    pub components: Vec<Box<dyn PipelineComponent>>,
}

impl ComponentPipeline {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Append a component and return `&mut Self` for chaining.
    pub fn add_component(&mut self, c: Box<dyn PipelineComponent>) -> &mut Self {
        self.components.push(c);
        self
    }

    /// Run all components in sequence.  The initial input is wrapped into a
    /// single-element `Vec`; each component's output values feed the next.
    pub fn run(&self, input: &str) -> Vec<ComponentOutput> {
        let mut current_inputs: Vec<String> = vec![input.to_string()];
        let mut last_outputs: Vec<ComponentOutput> = Vec::new();

        for component in &self.components {
            last_outputs = component.run(&current_inputs);
            current_inputs = last_outputs.iter().map(|o| o.value.clone()).collect();
        }

        last_outputs
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Pre-built pipeline: `TextSplitter(chunk_size=100)` followed by
    /// `DocumentRetriever(top_k=3)`.
    pub fn with_defaults() -> Self {
        let mut p = Self::new();
        p.add_component(Box::new(TextSplitter { chunk_size: 100 }));
        p.add_component(Box::new(DocumentRetriever { top_k: 3 }));
        p
    }
}

impl Default for ComponentPipeline {
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
    fn text_splitter_splits_input() {
        let splitter = TextSplitter { chunk_size: 3 };
        let outputs = splitter.run(&["abcdef".to_string()]);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "chunks");
        // "abcdef" with chunk_size=3 → "abc|def"
        assert_eq!(outputs[0].value, "abc|def");
    }

    #[test]
    fn text_splitter_empty_input() {
        let splitter = TextSplitter { chunk_size: 10 };
        let outputs = splitter.run(&[]);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "chunks");
        assert_eq!(outputs[0].value, "");
    }

    #[test]
    fn document_retriever_returns_top_k() {
        let retriever = DocumentRetriever { top_k: 5 };
        let outputs = retriever.run(&["query".to_string()]);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "documents");
        // 5 docs separated by "|"
        let docs: Vec<&str> = outputs[0].value.split('|').collect();
        assert_eq!(docs.len(), 5);
    }

    #[test]
    fn pipeline_empty() {
        let pipeline = ComponentPipeline::new();
        let outputs = pipeline.run("hello");
        assert!(outputs.is_empty(), "empty pipeline must produce no outputs");
    }

    #[test]
    fn pipeline_add_component_count() {
        let mut pipeline = ComponentPipeline::new();
        pipeline.add_component(Box::new(TextSplitter { chunk_size: 50 }));
        pipeline.add_component(Box::new(DocumentRetriever { top_k: 2 }));
        assert_eq!(pipeline.component_count(), 2);
    }

    #[test]
    fn pipeline_run_with_defaults() {
        let pipeline = ComponentPipeline::with_defaults();
        let outputs = pipeline.run("test query for pipeline");
        // Last component is DocumentRetriever(top_k=3) → 1 output named "documents"
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "documents");
        // top_k=3 → 3 pipe-separated entries
        let docs: Vec<&str> = outputs[0].value.split('|').collect();
        assert_eq!(docs.len(), 3);
    }

    #[test]
    fn pipeline_with_defaults_has_two() {
        let pipeline = ComponentPipeline::with_defaults();
        assert_eq!(pipeline.component_count(), 2);
    }

    #[test]
    fn component_output_fields() {
        let output = ComponentOutput {
            name: "result".to_string(),
            value: "42".to_string(),
        };
        assert_eq!(output.name, "result");
        assert_eq!(output.value, "42");
        // Clone must work
        let cloned = output.clone();
        assert_eq!(cloned.name, output.name);
        assert_eq!(cloned.value, output.value);
    }
}
