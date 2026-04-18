//! LLaMA-style RAG pipeline composition types.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStage {
    Retrieval,
    Reranking,
    Augmentation,
    Generation,
    Postprocessing,
}

impl PipelineStage {
    pub fn is_llm_stage(&self) -> bool {
        matches!(self, PipelineStage::Augmentation | PipelineStage::Generation)
    }

    pub fn stage_index(&self) -> usize {
        match self {
            PipelineStage::Retrieval => 0,
            PipelineStage::Reranking => 1,
            PipelineStage::Augmentation => 2,
            PipelineStage::Generation => 3,
            PipelineStage::Postprocessing => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlamaPipelineNode {
    pub stage: PipelineStage,
    pub name: String,
    pub enabled: bool,
}

impl LlamaPipelineNode {
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
}

#[derive(Debug, Clone, Default)]
pub struct LlamaPipeline {
    pub nodes: Vec<LlamaPipelineNode>,
}

impl LlamaPipeline {
    pub fn add_node(&mut self, node: LlamaPipelineNode) {
        self.nodes.push(node);
    }

    pub fn enabled_nodes(&self) -> Vec<&LlamaPipelineNode> {
        self.nodes.iter().filter(|n| n.enabled).collect()
    }

    pub fn stage_order(&self) -> Vec<&LlamaPipelineNode> {
        let mut sorted: Vec<&LlamaPipelineNode> = self.nodes.iter().collect();
        sorted.sort_by_key(|n| n.stage.stage_index());
        sorted
    }

    pub fn has_generation(&self) -> bool {
        self.nodes
            .iter()
            .any(|n| n.enabled && n.stage == PipelineStage::Generation)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PipelineCombinator {
    pub pipelines: Vec<LlamaPipeline>,
}

impl PipelineCombinator {
    pub fn add_pipeline(&mut self, p: LlamaPipeline) {
        self.pipelines.push(p);
    }

    pub fn merge(&self) -> LlamaPipeline {
        let mut merged = LlamaPipeline::default();
        for pipeline in &self.pipelines {
            for node in &pipeline.nodes {
                merged.nodes.push(node.clone());
            }
        }
        merged
    }
}

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub text: String,
    pub source_count: usize,
    pub latency_ms: u64,
}

impl PipelineOutput {
    pub fn is_fast(&self) -> bool {
        self.latency_ms < 1000
    }
}

#[cfg(test)]
mod llama_compose_tests {
    use super::*;

    #[test]
    fn stage_is_llm_stage() {
        assert!(!PipelineStage::Retrieval.is_llm_stage());
        assert!(!PipelineStage::Reranking.is_llm_stage());
        assert!(PipelineStage::Augmentation.is_llm_stage());
        assert!(PipelineStage::Generation.is_llm_stage());
        assert!(!PipelineStage::Postprocessing.is_llm_stage());
    }

    #[test]
    fn stage_index_ordering() {
        assert!(
            PipelineStage::Retrieval.stage_index()
                < PipelineStage::Reranking.stage_index()
        );
        assert!(
            PipelineStage::Reranking.stage_index()
                < PipelineStage::Augmentation.stage_index()
        );
        assert!(
            PipelineStage::Augmentation.stage_index()
                < PipelineStage::Generation.stage_index()
        );
        assert!(
            PipelineStage::Generation.stage_index()
                < PipelineStage::Postprocessing.stage_index()
        );
        assert_eq!(PipelineStage::Retrieval.stage_index(), 0);
        assert_eq!(PipelineStage::Postprocessing.stage_index(), 4);
    }

    #[test]
    fn node_enable_disable() {
        let mut node = LlamaPipelineNode {
            stage: PipelineStage::Retrieval,
            name: "retriever".to_string(),
            enabled: true,
        };
        assert!(node.enabled);
        node.disable();
        assert!(!node.enabled);
        node.enable();
        assert!(node.enabled);
    }

    #[test]
    fn pipeline_enabled_nodes_count() {
        let mut pipeline = LlamaPipeline::default();
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Retrieval,
            name: "retriever".to_string(),
            enabled: true,
        });
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Generation,
            name: "generator".to_string(),
            enabled: false,
        });
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Reranking,
            name: "reranker".to_string(),
            enabled: true,
        });
        let enabled = pipeline.enabled_nodes();
        assert_eq!(enabled.len(), 2);
    }

    #[test]
    fn stage_order_sorted() {
        let mut pipeline = LlamaPipeline::default();
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Postprocessing,
            name: "post".to_string(),
            enabled: true,
        });
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Retrieval,
            name: "retrieve".to_string(),
            enabled: true,
        });
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Generation,
            name: "generate".to_string(),
            enabled: true,
        });
        let ordered = pipeline.stage_order();
        let indices: Vec<usize> = ordered.iter().map(|n| n.stage.stage_index()).collect();
        let mut expected = indices.clone();
        expected.sort();
        assert_eq!(indices, expected, "stage_order must return nodes in ascending stage_index order");
    }

    #[test]
    fn has_generation_true() {
        let mut pipeline = LlamaPipeline::default();
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Retrieval,
            name: "retriever".to_string(),
            enabled: true,
        });
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Generation,
            name: "generator".to_string(),
            enabled: true,
        });
        assert!(pipeline.has_generation());
    }

    #[test]
    fn has_generation_false() {
        let mut pipeline = LlamaPipeline::default();
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Retrieval,
            name: "retriever".to_string(),
            enabled: true,
        });
        // Generation node exists but is disabled
        pipeline.add_node(LlamaPipelineNode {
            stage: PipelineStage::Generation,
            name: "generator".to_string(),
            enabled: false,
        });
        assert!(!pipeline.has_generation());
    }

    #[test]
    fn combinator_merge_node_count() {
        let mut p1 = LlamaPipeline::default();
        p1.add_node(LlamaPipelineNode {
            stage: PipelineStage::Retrieval,
            name: "r1".to_string(),
            enabled: true,
        });
        p1.add_node(LlamaPipelineNode {
            stage: PipelineStage::Reranking,
            name: "rr1".to_string(),
            enabled: true,
        });

        let mut p2 = LlamaPipeline::default();
        p2.add_node(LlamaPipelineNode {
            stage: PipelineStage::Generation,
            name: "g1".to_string(),
            enabled: true,
        });
        p2.add_node(LlamaPipelineNode {
            stage: PipelineStage::Postprocessing,
            name: "pp1".to_string(),
            enabled: true,
        });
        p2.add_node(LlamaPipelineNode {
            stage: PipelineStage::Augmentation,
            name: "aug1".to_string(),
            enabled: true,
        });

        let mut combinator = PipelineCombinator::default();
        combinator.add_pipeline(p1);
        combinator.add_pipeline(p2);

        let merged = combinator.merge();
        assert_eq!(merged.nodes.len(), 5, "merged pipeline must contain all nodes from all sub-pipelines");
    }

    #[test]
    fn output_is_fast() {
        let fast = PipelineOutput {
            text: "result".to_string(),
            source_count: 3,
            latency_ms: 500,
        };
        assert!(fast.is_fast());

        let slow = PipelineOutput {
            text: "result".to_string(),
            source_count: 3,
            latency_ms: 1000,
        };
        assert!(!slow.is_fast());

        let borderline = PipelineOutput {
            text: "result".to_string(),
            source_count: 0,
            latency_ms: 999,
        };
        assert!(borderline.is_fast());
    }
}
