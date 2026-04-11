//! nom-planner: Generates execution plans from verified Nom composition graphs.
//!
//! Takes a verified [`SourceFile`] and produces a [`CompositionPlan`] that
//! describes:
//!   - The ordered set of nodes (word invocations)
//!   - Memory strategy (stack vs heap allocation hints)
//!   - Concurrency strategy (sequential, parallel, pipeline)
//!   - Effect summary
//!   - The .nomiz serialized form

use nom_ast::{Declaration, FlowChain, FlowStep, NomRef, SourceFile, Statement};
use nom_resolver::{Resolver, WordEntry};
use nom_verifier::Verifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("verification failed with {0} findings")]
    VerificationFailed(usize),
    #[error("resolver error: {0}")]
    Resolver(#[from] nom_resolver::ResolverError),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Memory allocation strategy for a flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryStrategy {
    /// All intermediate values fit on the stack.
    Stack,
    /// Large or dynamic values require heap allocation.
    Heap,
    /// Mixed: some steps use stack, others heap.
    Mixed,
}

/// Execution concurrency strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConcurrencyStrategy {
    /// Steps run one after another.
    Sequential,
    /// Independent steps can run in parallel.
    Parallel,
    /// Steps are pipelined (producer–consumer queues).
    Pipeline,
}

/// A single planned node: one word invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub id: usize,
    pub word: String,
    pub variant: Option<String>,
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub effects: Vec<String>,
    /// Actual implementation code body (if available from dictionary).
    pub impl_body: Option<String>,
    /// Source language of the implementation body (e.g. "rust", "python").
    pub impl_language: Option<String>,
}

/// An edge between two plan nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEdge {
    pub from: usize,
    pub to: usize,
}

/// A branch in the execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBranch {
    pub condition: String,
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<PlanEdge>,
}

/// Execution plan for a single flow declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPlan {
    pub name: String,
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<PlanEdge>,
    pub branches: Vec<PlanBranch>,
    pub memory_strategy: MemoryStrategy,
    pub concurrency_strategy: ConcurrencyStrategy,
    /// Union of all effects produced by nodes in this flow.
    pub effect_summary: Vec<String>,
}

/// The overall composition plan for a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionPlan {
    pub source_path: Option<String>,
    pub flows: Vec<FlowPlan>,
    /// Serialised form (`.nomiz`).
    pub nomiz: String,
}

impl CompositionPlan {
    /// Serialise to `.nomiz` format (JSON for now, compact).
    pub fn to_nomiz(&self) -> Result<String, PlanError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialise from `.nomiz` bytes.
    pub fn from_nomiz(bytes: &str) -> Result<Self, PlanError> {
        Ok(serde_json::from_str(bytes)?)
    }
}

/// The planner ties together resolver, verifier, and plan generation.
pub struct Planner<'r> {
    resolver: &'r Resolver,
}

impl<'r> Planner<'r> {
    pub fn new(resolver: &'r Resolver) -> Self {
        Self { resolver }
    }

    /// Plan a source file. Runs verification first; fails if there are errors.
    pub fn plan(&self, source: &SourceFile) -> Result<CompositionPlan, PlanError> {
        let verifier = Verifier::new(self.resolver);
        let vresult = verifier.verify(source);
        if !vresult.ok() {
            return Err(PlanError::VerificationFailed(vresult.findings.len()));
        }
        self.plan_unchecked(source)
    }

    /// Plan without running verification (useful when already verified).
    pub fn plan_unchecked(&self, source: &SourceFile) -> Result<CompositionPlan, PlanError> {
        let mut flows = Vec::new();
        for decl in &source.declarations {
            if let Some(flow_plan) = self.plan_declaration(decl)? {
                flows.push(flow_plan);
            }
        }
        let mut plan = CompositionPlan {
            source_path: source.path.clone(),
            flows,
            nomiz: String::new(),
        };
        plan.nomiz = plan.to_nomiz()?;
        Ok(plan)
    }

    fn plan_declaration(&self, decl: &Declaration) -> Result<Option<FlowPlan>, PlanError> {
        let mut all_nodes: Vec<PlanNode> = Vec::new();
        let mut all_edges: Vec<PlanEdge> = Vec::new();
        let mut branches: Vec<PlanBranch> = Vec::new();
        let mut effect_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut node_counter = 0usize;

        for stmt in &decl.statements {
            if let Statement::Flow(flow_stmt) = stmt {
                self.plan_chain(
                    &flow_stmt.chain,
                    &mut all_nodes,
                    &mut all_edges,
                    &mut branches,
                    &mut effect_set,
                    &mut node_counter,
                )?;
            }
        }

        if all_nodes.is_empty() && branches.is_empty() {
            return Ok(None);
        }

        // Determine memory strategy
        let memory_strategy = if all_nodes.iter().any(|n| {
            n.output_type.as_deref().map(|t| t.contains("bytes") || t.contains("buffer")).unwrap_or(false)
        }) {
            MemoryStrategy::Heap
        } else {
            MemoryStrategy::Stack
        };

        // Determine concurrency strategy
        let concurrency_strategy = if !branches.is_empty() {
            ConcurrencyStrategy::Parallel
        } else if all_nodes.len() > 3 {
            ConcurrencyStrategy::Pipeline
        } else {
            ConcurrencyStrategy::Sequential
        };

        let mut effect_summary: Vec<String> = effect_set.into_iter().collect();
        effect_summary.sort();

        Ok(Some(FlowPlan {
            name: decl.name.name.clone(),
            nodes: all_nodes,
            edges: all_edges,
            branches,
            memory_strategy,
            concurrency_strategy,
            effect_summary,
        }))
    }

    fn plan_chain(
        &self,
        chain: &FlowChain,
        nodes: &mut Vec<PlanNode>,
        edges: &mut Vec<PlanEdge>,
        branches: &mut Vec<PlanBranch>,
        effect_set: &mut std::collections::HashSet<String>,
        counter: &mut usize,
    ) -> Result<(), PlanError> {
        let mut prev_id: Option<usize> = None;

        for step in &chain.steps {
            match step {
                FlowStep::Ref(nom_ref) => {
                    let entry = self.resolver.resolve(nom_ref).ok();
                    let node = self.make_node(*counter, nom_ref, entry.as_ref());
                    if let Some(ref e) = entry {
                        for eff in &e.effects {
                            effect_set.insert(eff.clone());
                        }
                    }
                    let id = node.id;
                    nodes.push(node);
                    if let Some(prev) = prev_id {
                        edges.push(PlanEdge { from: prev, to: id });
                    }
                    prev_id = Some(id);
                    *counter += 1;
                }
                FlowStep::Branch(block) => {
                    for arm in &block.arms {
                        let mut branch_nodes = Vec::new();
                        let mut branch_edges = Vec::new();
                        let mut branch_effects: std::collections::HashSet<String> = std::collections::HashSet::new();
                        let mut branch_counter = *counter;
                        self.plan_chain(
                            &arm.chain,
                            &mut branch_nodes,
                            &mut branch_edges,
                            &mut Vec::new(),
                            &mut branch_effects,
                            &mut branch_counter,
                        )?;
                        *counter = branch_counter;
                        effect_set.extend(branch_effects);
                        branches.push(PlanBranch {
                            condition: format!("{:?}", arm.condition),
                            nodes: branch_nodes,
                            edges: branch_edges,
                        });
                    }
                }
                _ => {
                    // Literals and calls: create a placeholder node
                    let placeholder = PlanNode {
                        id: *counter,
                        word: format!("<literal_{}>", counter),
                        variant: None,
                        input_type: None,
                        output_type: Some("any".to_owned()),
                        effects: vec![],
                        impl_body: None,
                        impl_language: None,
                    };
                    let id = placeholder.id;
                    nodes.push(placeholder);
                    if let Some(prev) = prev_id {
                        edges.push(PlanEdge { from: prev, to: id });
                    }
                    prev_id = Some(id);
                    *counter += 1;
                }
            }
        }
        Ok(())
    }

    /// Enrich plan nodes with implementation bodies from the resolver's
    /// `implementations` table. Nodes that have no matching implementation
    /// are left unchanged (impl_body stays None).
    pub fn enrich_with_implementations(&self, plan: &mut CompositionPlan) {
        for flow in &mut plan.flows {
            for node in &mut flow.nodes {
                if node.impl_body.is_some() {
                    continue;
                }
                if let Ok(Some(imp)) = self.resolver.get_impl(
                    &node.word,
                    node.variant.as_deref(),
                ) {
                    node.impl_body = imp.body;
                    node.impl_language = Some(imp.language);
                }
            }
            for branch in &mut flow.branches {
                for node in &mut branch.nodes {
                    if node.impl_body.is_some() {
                        continue;
                    }
                    if let Ok(Some(imp)) = self.resolver.get_impl(
                        &node.word,
                        node.variant.as_deref(),
                    ) {
                        node.impl_body = imp.body;
                        node.impl_language = Some(imp.language);
                    }
                }
            }
        }
    }

    fn make_node(&self, id: usize, nom_ref: &NomRef, entry: Option<&WordEntry>) -> PlanNode {
        if let Some(e) = entry {
            PlanNode {
                id,
                word: e.word.clone(),
                variant: e.variant.clone(),
                input_type: e.input_type.clone(),
                output_type: e.output_type.clone(),
                effects: e.effects.clone(),
                impl_body: None,
                impl_language: None,
            }
        } else {
            PlanNode {
                id,
                word: nom_ref.word.name.clone(),
                variant: nom_ref.variant.as_ref().map(|v| v.name.clone()),
                input_type: None,
                output_type: None,
                effects: vec![],
                impl_body: None,
                impl_language: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{
        Classifier, Declaration, FlowChain, FlowStep, FlowStmt, Identifier, NomRef, SourceFile,
        Span, Statement,
    };
    use nom_resolver::{Resolver, WordEntry};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn setup() -> Resolver {
        let r = Resolver::open_in_memory().unwrap();
        r.upsert(&WordEntry {
            word: "hash".to_owned(),
            variant: Some("argon2".to_owned()),
            input_type: Some("bytes".to_owned()),
            output_type: Some("hash".to_owned()),
            effects: vec!["cpu".to_owned()],
            security: 0.95,
            performance: 0.7,
            reliability: 0.99,
            ..WordEntry::default()
        }).unwrap();
        r
    }

    fn make_source(decl: Declaration) -> SourceFile {
        SourceFile {
            path: None,
            locale: None,
            declarations: vec![decl],
        }
    }

    #[test]
    fn plan_single_flow() {
        let resolver = setup();
        let planner = Planner::new(&resolver);

        let decl = Declaration {
            classifier: Classifier::Flow,
            name: Identifier::new("myflow", span()),
            statements: vec![Statement::Flow(FlowStmt {
                chain: FlowChain {
                    steps: vec![FlowStep::Ref(NomRef {
                        word: Identifier::new("hash", span()),
                        variant: Some(Identifier::new("argon2", span())),
                        span: span(),
                    })],
                },
                span: span(),
            })],
            span: span(),
        };

        let source = make_source(decl);
        let plan = planner.plan_unchecked(&source).unwrap();
        assert_eq!(plan.flows.len(), 1);
        let flow = &plan.flows[0];
        assert_eq!(flow.name, "myflow");
        assert_eq!(flow.nodes.len(), 1);
        assert!(flow.effect_summary.contains(&"cpu".to_owned()));
    }

    #[test]
    fn nomiz_roundtrip() {
        let resolver = setup();
        let planner = Planner::new(&resolver);
        let source = SourceFile { path: None, locale: None, declarations: vec![] };
        let plan = planner.plan_unchecked(&source).unwrap();
        let nomiz = plan.to_nomiz().unwrap();
        let plan2 = CompositionPlan::from_nomiz(&nomiz).unwrap();
        assert_eq!(plan2.flows.len(), 0);
    }
}
