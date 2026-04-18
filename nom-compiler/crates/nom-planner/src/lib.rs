//! nom-planner: Generates execution plans from verified Nom composition graphs.
//!
//! Takes a verified [`SourceFile`] and produces a [`CompositionPlan`] that
//! describes:
//!   - The ordered set of nodes (word invocations)
//!   - Memory strategy (stack vs heap allocation hints)
//!   - Concurrency strategy (sequential, parallel, pipeline)
//!   - Effect summary
//!   - The .nomiz serialized form

use nom_ast::{
    BranchArm, BranchBlock, BranchCondition, Classifier, Declaration, Expr, FlowChain, FlowStep,
    GraphQueryExpr, GraphSetExpr, GraphSetOp, GraphTraverseExpr, Literal, NomRef, OnFailStrategy,
    SourceFile, Statement,
};
use nom_resolver::{Resolver, WordEntry};
use nom_verifier::Verifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

fn default_on_fail() -> String {
    // Shared with self-host planner.nom::default_on_fail() via the
    // parity test in nom-cli/tests/self_host_rust_parity.rs.
    nom_types::self_host_tags::DEFAULT_ON_FAIL.to_owned()
}

fn on_fail_strategy_to_string(strategy: &OnFailStrategy) -> String {
    match strategy {
        OnFailStrategy::Abort => nom_types::self_host_tags::DEFAULT_ON_FAIL.to_owned(),
        OnFailStrategy::RestartFrom(id) => format!("restart_from:{}", id.name),
        OnFailStrategy::Retry(n) => format!("retry:{}", n),
        OnFailStrategy::Skip => "skip".to_owned(),
        OnFailStrategy::Escalate => "escalate".to_owned(),
    }
}

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("verification failed with {} findings", .0.len())]
    VerificationFailed(Vec<String>),
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

/// Runtime-oriented agent metadata collected from an agent declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetadataPlan {
    pub capabilities: Vec<String>,
    pub state: Option<String>,
    pub supervision: Option<AgentSupervisionPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSupervisionPlan {
    pub strategy: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphFieldPlan {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodePlan {
    pub name: String,
    pub fields: Vec<GraphFieldPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgePlan {
    pub name: String,
    pub from_type: String,
    pub to_type: String,
    pub fields: Vec<GraphFieldPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConstraintPlan {
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryPlan {
    pub name: String,
    pub params: Vec<GraphQueryParamPlan>,
    pub expr: GraphQueryExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryParamPlan {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphMetadataPlan {
    pub nodes: Vec<GraphNodePlan>,
    pub edges: Vec<GraphEdgePlan>,
    pub constraints: Vec<GraphConstraintPlan>,
    pub queries: Vec<GraphQueryPlan>,
}

/// Execution plan for a single flow declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPlan {
    pub name: String,
    pub classifier: String,
    pub agent: Option<AgentMetadataPlan>,
    pub graph: Option<GraphMetadataPlan>,
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<PlanEdge>,
    pub branches: Vec<PlanBranch>,
    pub memory_strategy: MemoryStrategy,
    pub concurrency_strategy: ConcurrencyStrategy,
    /// Flow execution qualifier (once/stream/scheduled)
    #[serde(default)]
    pub qualifier: String,
    /// Fault handling strategy: "abort", "restart_from:<node>", "retry:<n>", "skip", "escalate"
    #[serde(default = "default_on_fail")]
    pub on_fail: String,
    /// Union of all effects produced by nodes in this flow.
    pub effect_summary: Vec<String>,
    /// Imperative statements (fn, struct, enum, let, if, etc.) serialized as AST.
    /// These are emitted directly by the codegen alongside flow code.
    #[serde(default)]
    pub imperative_stmts: Vec<nom_ast::Statement>,
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
            let details: Vec<String> = vresult
                .findings
                .iter()
                .map(|f| format!("[{}] {}", f.declaration, f.error))
                .collect();
            return Err(PlanError::VerificationFailed(details));
        }
        self.plan_unchecked(source)
    }

    /// Plan without running verification (useful when already verified).
    pub fn plan_unchecked(&self, source: &SourceFile) -> Result<CompositionPlan, PlanError> {
        let mut flows = Vec::new();
        for decl in &source.declarations {
            flows.extend(self.plan_declaration(decl)?);
        }
        let mut plan = CompositionPlan {
            source_path: source.path.clone(),
            flows,
            nomiz: String::new(),
        };
        plan.nomiz = plan.to_nomiz()?;
        Ok(plan)
    }

    /// Primary compiler pipeline integration for GAP-6.
    /// Derives an execution plan directly from `nom-concept` S6 PipelineOutput,
    /// bypassing the legacy parser and AST bridge.
    pub fn plan_from_pipeline_output(
        &self,
        pipeline_out: &nom_concept::stages::PipelineOutput,
    ) -> Result<CompositionPlan, PlanError> {
        let mut flows = Vec::new();

        match pipeline_out {
            nom_concept::stages::PipelineOutput::Nomtu(nomtu) => {
                for item in &nomtu.items {
                    if let nom_concept::NomtuItem::Composition(comp) = item {
                        let mut steps = Vec::new();
                        for e_ref in &comp.composes {
                            // Upcast EntityRef to NomRef for the legacy resolver
                            let nom_ref = nom_ast::NomRef {
                                word: nom_ast::Identifier {
                                    name: e_ref.word.clone(),
                                    span: nom_ast::Span::default(),
                                },
                                variant: None, // No variant support in basic typed-slot currently
                                span: nom_ast::Span::default(),
                            };
                            steps.push(nom_ast::FlowStep::Ref(nom_ref));
                        }

                        let chain = nom_ast::FlowChain { steps };
                        let flow =
                            self.plan_chain_as_flow(&comp.word, "system", None, None, &chain)?;
                        flows.push(flow);
                    }
                    // Imperative inner functions / singular entities are deferred for full LLVM lowering.
                }
            }
            nom_concept::stages::PipelineOutput::Nom(_) => {
                // Concept files just define indexes right now, no execution flows.
            }
        }

        let mut plan = CompositionPlan {
            source_path: None,
            flows,
            nomiz: String::new(),
        };
        plan.nomiz = plan.to_nomiz()?;
        Ok(plan)
    }

    fn plan_declaration(&self, decl: &Declaration) -> Result<Vec<FlowPlan>, PlanError> {
        let mut flow_plans = Vec::new();
        let mut declaration_flow_count = 0usize;
        let mut receive_count = 0usize;
        let mut schedule_count = 0usize;
        let mut imperative_stmts: Vec<Statement> = Vec::new();
        let classifier = decl.classifier.as_str().to_owned();
        let agent_metadata = self.extract_agent_metadata(decl);
        let graph_metadata = self.extract_graph_metadata(decl);

        for stmt in &decl.statements {
            match stmt {
                Statement::Flow(flow_stmt) => {
                    let name = if declaration_flow_count == 0 {
                        decl.name.name.clone()
                    } else {
                        format!("{}__flow_{}", decl.name.name, declaration_flow_count + 1)
                    };
                    let qualifier = flow_stmt.qualifier.as_str().to_owned();
                    let mut plan = self.plan_chain_as_flow(
                        &name,
                        &classifier,
                        agent_metadata.clone(),
                        None,
                        &flow_stmt.chain,
                    )?;
                    plan.qualifier = qualifier;
                    plan.on_fail = on_fail_strategy_to_string(&flow_stmt.on_fail);
                    flow_plans.push(plan);
                    declaration_flow_count += 1;
                }
                Statement::AgentReceive(receive_stmt) => {
                    let name = if receive_count == 0 {
                        format!("{}__receive", decl.name.name)
                    } else {
                        format!("{}__receive_{}", decl.name.name, receive_count + 1)
                    };
                    flow_plans.push(self.plan_chain_as_flow(
                        &name,
                        &classifier,
                        agent_metadata.clone(),
                        None,
                        &receive_stmt.chain,
                    )?);
                    receive_count += 1;
                }
                Statement::AgentSchedule(schedule_stmt) => {
                    let base = sanitize_plan_name(&schedule_stmt.interval);
                    let name = if schedule_count == 0 {
                        format!("{}__schedule__{}", decl.name.name, base)
                    } else {
                        format!(
                            "{}__schedule__{}_{}",
                            decl.name.name,
                            base,
                            schedule_count + 1
                        )
                    };
                    flow_plans.push(self.plan_chain_as_flow(
                        &name,
                        &classifier,
                        agent_metadata.clone(),
                        None,
                        &schedule_stmt.action,
                    )?);
                    schedule_count += 1;
                }
                Statement::GraphQuery(query_stmt) => {
                    let name = format!("{}__{}", decl.name.name, query_stmt.name.name);
                    let query_chain = self
                        .graph_query_expr_to_flow_chain(&query_stmt.expr)
                        .unwrap_or(FlowChain { steps: Vec::new() });
                    flow_plans.push(self.plan_chain_as_flow(
                        &name,
                        &classifier,
                        agent_metadata.clone(),
                        None,
                        &query_chain,
                    )?);
                }
                // Imperative statements — collect them
                Statement::FnDef(_)
                | Statement::StructDef(_)
                | Statement::EnumDef(_)
                | Statement::Let(_)
                | Statement::If(_)
                | Statement::For(_)
                | Statement::While(_)
                | Statement::Match(_)
                | Statement::Return(_)
                | Statement::ExprStmt(_)
                | Statement::Assign(_) => {
                    imperative_stmts.push(stmt.clone());
                }
                _ => {}
            }
        }

        // If there are imperative statements but no flow, create a module for them
        if !imperative_stmts.is_empty() && declaration_flow_count == 0 {
            flow_plans.push(FlowPlan {
                name: decl.name.name.clone(),
                classifier: classifier.clone(),
                agent: agent_metadata.clone(),
                graph: None,
                nodes: Vec::new(),
                edges: Vec::new(),
                branches: Vec::new(),
                memory_strategy: MemoryStrategy::Stack,
                concurrency_strategy: ConcurrencyStrategy::Sequential,
                qualifier: "once".to_owned(),
                on_fail: "abort".to_owned(),
                effect_summary: Vec::new(),
                imperative_stmts: imperative_stmts.clone(),
            });
        } else if !imperative_stmts.is_empty() {
            // Attach imperative stmts to the first flow plan
            if let Some(first) = flow_plans.first_mut() {
                first.imperative_stmts = imperative_stmts.clone();
            }
        }

        if decl.classifier == Classifier::Graph
            && graph_metadata.is_some()
            && flow_plans.iter().all(|flow| flow.name != decl.name.name)
        {
            flow_plans.insert(
                0,
                self.plan_chain_as_flow(
                    &decl.name.name,
                    &classifier,
                    agent_metadata,
                    graph_metadata,
                    &FlowChain { steps: Vec::new() },
                )?,
            );
        }

        Ok(flow_plans)
    }

    fn plan_chain_as_flow(
        &self,
        name: &str,
        classifier: &str,
        agent: Option<AgentMetadataPlan>,
        graph: Option<GraphMetadataPlan>,
        chain: &FlowChain,
    ) -> Result<FlowPlan, PlanError> {
        let mut all_nodes: Vec<PlanNode> = Vec::new();
        let mut all_edges: Vec<PlanEdge> = Vec::new();
        let mut branches: Vec<PlanBranch> = Vec::new();
        let mut effect_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut node_counter = 0usize;

        self.plan_chain(
            chain,
            &mut all_nodes,
            &mut all_edges,
            &mut branches,
            &mut effect_set,
            &mut node_counter,
        )?;

        if all_nodes.is_empty() && branches.is_empty() {
            return Ok(FlowPlan {
                name: name.to_owned(),
                classifier: classifier.to_owned(),
                agent,
                graph,
                nodes: all_nodes,
                edges: all_edges,
                branches,
                memory_strategy: MemoryStrategy::Stack,
                concurrency_strategy: ConcurrencyStrategy::Sequential,
                qualifier: "once".to_owned(),
                on_fail: "abort".to_owned(),
                effect_summary: Vec::new(),
                imperative_stmts: Vec::new(),
            });
        }

        // Determine memory strategy
        let memory_strategy = infer_memory_strategy(
            &FlowPlan {
                name: name.to_owned(),
                classifier: classifier.to_owned(),
                agent: agent.clone(),
                graph: graph.clone(),
                nodes: all_nodes.clone(),
                edges: all_edges.clone(),
                branches: branches.clone(),
                memory_strategy: MemoryStrategy::Stack, // dummy
                concurrency_strategy: ConcurrencyStrategy::Sequential, // dummy
                qualifier: "once".to_owned(),
                on_fail: "abort".to_owned(),
                effect_summary: vec![],
                imperative_stmts: vec![],
            },
            self.resolver,
        );

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

        Ok(FlowPlan {
            name: name.to_owned(),
            classifier: classifier.to_owned(),
            agent,
            graph,
            nodes: all_nodes,
            edges: all_edges,
            branches,
            memory_strategy,
            concurrency_strategy,
            qualifier: "once".to_owned(),
            on_fail: "abort".to_owned(),
            effect_summary,
            imperative_stmts: Vec::new(),
        })
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
                        let mut branch_effects: std::collections::HashSet<String> =
                            std::collections::HashSet::new();
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
                if let Ok(Some(imp)) = self.resolver.get_impl(&node.word, node.variant.as_deref()) {
                    node.impl_body = imp.body;
                    node.impl_language = Some(imp.language);
                }
            }
            for branch in &mut flow.branches {
                for node in &mut branch.nodes {
                    if node.impl_body.is_some() {
                        continue;
                    }
                    if let Ok(Some(imp)) =
                        self.resolver.get_impl(&node.word, node.variant.as_deref())
                    {
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

    fn extract_agent_metadata(&self, decl: &Declaration) -> Option<AgentMetadataPlan> {
        if decl.classifier != Classifier::Agent {
            return None;
        }

        let mut metadata = AgentMetadataPlan::default();
        for stmt in &decl.statements {
            match stmt {
                Statement::AgentCapability(capability_stmt) => {
                    metadata.capabilities.extend(
                        capability_stmt
                            .capabilities
                            .iter()
                            .map(|capability| capability.name.clone()),
                    );
                }
                Statement::AgentState(state_stmt) => {
                    metadata.state = Some(state_stmt.state.name.clone());
                }
                Statement::AgentSupervise(supervise_stmt) => {
                    metadata.supervision = Some(AgentSupervisionPlan {
                        strategy: supervise_stmt.strategy.name.clone(),
                        params: supervise_stmt
                            .params
                            .iter()
                            .map(|(key, value)| (key.name.clone(), expr_to_string(value)))
                            .collect(),
                    });
                }
                _ => {}
            }
        }

        Some(metadata)
    }

    fn extract_graph_metadata(&self, decl: &Declaration) -> Option<GraphMetadataPlan> {
        if decl.classifier != Classifier::Graph {
            return None;
        }

        let mut metadata = GraphMetadataPlan::default();
        for stmt in &decl.statements {
            match stmt {
                Statement::GraphNode(node_stmt) => {
                    metadata.nodes.push(GraphNodePlan {
                        name: node_stmt.name.name.clone(),
                        fields: node_stmt
                            .fields
                            .iter()
                            .map(|field| GraphFieldPlan {
                                name: field.name.name.clone(),
                                ty: field.typ.as_ref().map(|ty| ty.name.clone()),
                            })
                            .collect(),
                    });
                }
                Statement::GraphEdge(edge_stmt) => {
                    metadata.edges.push(GraphEdgePlan {
                        name: edge_stmt.name.name.clone(),
                        from_type: edge_stmt.from_type.name.clone(),
                        to_type: edge_stmt.to_type.name.clone(),
                        fields: edge_stmt
                            .fields
                            .iter()
                            .map(|field| GraphFieldPlan {
                                name: field.name.name.clone(),
                                ty: field.typ.as_ref().map(|ty| ty.name.clone()),
                            })
                            .collect(),
                    });
                }
                Statement::GraphConstraint(constraint_stmt) => {
                    metadata.constraints.push(GraphConstraintPlan {
                        name: constraint_stmt.name.name.clone(),
                        expr: constraint_stmt.expr.clone(),
                    });
                }
                Statement::GraphQuery(query_stmt) => {
                    metadata.queries.push(GraphQueryPlan {
                        name: query_stmt.name.name.clone(),
                        params: query_stmt
                            .params
                            .iter()
                            .map(|param| GraphQueryParamPlan {
                                name: param.name.name.clone(),
                                ty: param.typ.as_ref().map(|ty| ty.name.clone()),
                            })
                            .collect(),
                        expr: query_stmt.expr.clone(),
                    });
                }
                _ => {}
            }
        }

        if metadata.nodes.is_empty()
            && metadata.edges.is_empty()
            && metadata.constraints.is_empty()
            && metadata.queries.is_empty()
        {
            None
        } else {
            Some(metadata)
        }
    }

    fn graph_query_expr_to_flow_chain(&self, expr: &GraphQueryExpr) -> Option<FlowChain> {
        Self::graph_query_expr_to_flow_chain_impl(expr)
    }

    fn graph_query_expr_to_flow_chain_impl(expr: &GraphQueryExpr) -> Option<FlowChain> {
        match expr {
            GraphQueryExpr::Ref(reference) => Some(FlowChain {
                steps: vec![FlowStep::Ref(reference.clone())],
            }),
            GraphQueryExpr::Traverse(GraphTraverseExpr {
                source,
                edge,
                target,
                ..
            }) => {
                let mut source_chain = Self::graph_query_expr_to_flow_chain_impl(source)?;
                let target_chain = Self::graph_query_expr_to_flow_chain_impl(target)?;
                source_chain.steps.push(FlowStep::Ref(edge.clone()));
                source_chain.steps.extend(target_chain.steps);
                Some(source_chain)
            }
            GraphQueryExpr::SetOp(GraphSetExpr { op, operands, span }) => {
                if *op == GraphSetOp::Difference || operands.is_empty() {
                    return None;
                }
                let label = match op {
                    GraphSetOp::Union => "union",
                    GraphSetOp::Intersection => "intersect",
                    GraphSetOp::Difference => return None,
                };
                let mut arms = Vec::new();
                for operand in operands {
                    arms.push(BranchArm {
                        condition: BranchCondition::Named,
                        label: Some(label.to_owned()),
                        chain: Self::graph_query_expr_to_flow_chain_impl(operand)?,
                    });
                }
                Some(FlowChain {
                    steps: vec![FlowStep::Branch(BranchBlock { arms, span: *span })],
                })
            }
        }
    }
}

/// Infer optimal memory strategy from flow graph topology and resolver metadata.
/// - Linear chain (no branches, no cycles) → Arena (bulk alloc/free) — currently maps to Stack
/// - DAG (branches, no cycles) → Mixed (stack where possible, arena for large values)
/// - Cyclic graph → Heap (pre-allocated blocks needed)
/// - Single node or empty → Stack
/// - Uses resolver metadata: if any node has low reliability (<0.8) or low performance (<0.5), use Heap
pub fn infer_memory_strategy(flow: &FlowPlan, resolver: &nom_resolver::Resolver) -> MemoryStrategy {
    if flow.nodes.is_empty() {
        return MemoryStrategy::Stack;
    }

    // Check for cycles by detecting back-edges
    let has_cycle = detect_cycle(&flow.nodes, &flow.edges);

    if has_cycle {
        return MemoryStrategy::Heap; // Cycles need heap/pool
    }

    // Check resolver metadata for low reliability or performance
    for node in &flow.nodes {
        let nom_ref = nom_ast::NomRef {
            word: nom_ast::Identifier {
                name: node.word.clone(),
                span: nom_ast::Span::default(),
            },
            variant: node.variant.as_ref().map(|v| nom_ast::Identifier {
                name: v.clone(),
                span: nom_ast::Span::default(),
            }),
            span: nom_ast::Span::default(),
        };
        if let Ok(entry) = resolver.resolve(&nom_ref) {
            if entry.reliability < 0.8 || entry.performance < 0.5 {
                return MemoryStrategy::Heap;
            }
        }
    }

    if flow.branches.is_empty() && flow.edges.len() == flow.nodes.len().saturating_sub(1) {
        // Linear chain — arena is optimal (Stack for now until Arena variant is added)
        return MemoryStrategy::Stack;
    }

    MemoryStrategy::Mixed // DAG with branches
}

fn detect_cycle(_nodes: &[PlanNode], edges: &[PlanEdge]) -> bool {
    // Simple cycle detection: check if any edge goes backwards
    for edge in edges {
        if edge.to <= edge.from {
            return true;
        }
    }
    false
}

// ── Plan optimization passes ─────────────────────────────────────────────────

/// Remove nodes that pass data through without transformation.
///
/// A node is considered an identity node when:
/// - It has no side effects.
/// - Its `input_type` and `output_type` are both `Some` and equal (same type in, same type out).
/// - It has no implementation body (`impl_body` is `None`), meaning it is a
///   placeholder that adds no work.
///
/// Identity nodes are removed and the edges that passed through them are
/// rewired so that their predecessor connects directly to their successor.
pub fn fuse_identity_nodes(flow: &mut FlowPlan) {
    let identity_ids: std::collections::HashSet<usize> = flow
        .nodes
        .iter()
        .filter(|n| {
            n.effects.is_empty()
                && n.impl_body.is_none()
                && n.input_type.is_some()
                && n.input_type == n.output_type
        })
        .map(|n| n.id)
        .collect();

    if identity_ids.is_empty() {
        return;
    }

    // Build a predecessor map: identity_id -> what feeds into it.
    let mut pred_of: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for edge in &flow.edges {
        if identity_ids.contains(&edge.to) {
            pred_of.insert(edge.to, edge.from);
        }
    }

    // Resolve the ultimate non-identity predecessor (chain collapse).
    let resolve_pred = |mut id: usize| -> usize {
        while let Some(&p) = pred_of.get(&id) {
            id = p;
        }
        id
    };

    // Rebuild edges: skip edges whose `to` is an identity node; rewire
    // edges whose `from` is an identity node to its resolved predecessor.
    let mut new_edges: Vec<PlanEdge> = Vec::new();
    for edge in &flow.edges {
        if identity_ids.contains(&edge.to) {
            // This edge feeds an identity node — drop it.
            continue;
        }
        let new_from = if identity_ids.contains(&edge.from) {
            resolve_pred(edge.from)
        } else {
            edge.from
        };
        // Avoid self-loops that would be introduced when both ends collapse.
        if new_from != edge.to {
            new_edges.push(PlanEdge {
                from: new_from,
                to: edge.to,
            });
        }
    }

    flow.nodes.retain(|n| !identity_ids.contains(&n.id));
    flow.edges = new_edges;
}

/// Merge two consecutive pure (effect-free) map nodes into one composed node.
///
/// When node A feeds directly into node B and both have no effects, they can
/// be represented as a single node whose `word` is `"<A>∘<B>"` and whose
/// type signature spans from A's input to B's output.  The intermediate edge
/// A→B is removed and a new single node replaces the pair.
///
/// Only the first eligible pair is collapsed per call so callers can invoke
/// this in a fixed-point loop when desired.
pub fn fuse_consecutive_maps(flow: &mut FlowPlan) {
    // Find the first edge A→B where both A and B are pure (no effects).
    let pair = flow.edges.iter().find_map(|edge| {
        let from_node = flow.nodes.iter().find(|n| n.id == edge.from)?;
        let to_node = flow.nodes.iter().find(|n| n.id == edge.to)?;
        if from_node.effects.is_empty() && to_node.effects.is_empty() {
            Some((edge.from, edge.to))
        } else {
            None
        }
    });

    let (a_id, b_id) = match pair {
        Some(p) => p,
        None => return,
    };

    let a = flow.nodes.iter().find(|n| n.id == a_id).unwrap().clone();
    let b = flow.nodes.iter().find(|n| n.id == b_id).unwrap().clone();

    let composed = PlanNode {
        id: a.id, // reuse A's id; B's id is retired
        word: format!("{}∘{}", a.word, b.word),
        variant: None,
        input_type: a.input_type.clone(),
        output_type: b.output_type.clone(),
        effects: vec![],
        impl_body: None,
        impl_language: None,
    };

    // Replace A with the composed node; remove B.
    flow.nodes.retain(|n| n.id != a_id && n.id != b_id);
    flow.nodes.push(composed);
    // Sort by id so the node list stays ordered.
    flow.nodes.sort_by_key(|n| n.id);

    // Remove the A→B edge; rewire any edges from B to point to A.
    flow.edges.retain(|e| !(e.from == a_id && e.to == b_id));
    for edge in &mut flow.edges {
        if edge.from == b_id {
            edge.from = a_id;
        }
    }
}

/// Collapse a branch that has exactly one arm into a direct linear flow.
///
/// When `flow.branches` contains a single-arm branch, the branch's nodes and
/// edges are promoted into the main flow.  The branch itself is removed.
/// Multiple single-arm branches are each collapsed independently; the function
/// processes one per call.
pub fn collapse_single_branch(flow: &mut FlowPlan) {
    // Find the index of the first branch with exactly one arm.
    let idx = flow
        .branches
        .iter()
        .position(|b| !b.nodes.is_empty() && b.edges.len() <= b.nodes.len());

    let idx = match idx {
        Some(i) => i,
        None => return,
    };

    let branch = flow.branches.remove(idx);

    // Determine the id offset so we do not collide with existing node ids.
    let max_existing = flow.nodes.iter().map(|n| n.id).max().unwrap_or(0);
    let offset = max_existing + 1;

    // Re-id the branch nodes and edges.
    let reindex = |id: usize| id + offset;

    let new_nodes: Vec<PlanNode> = branch
        .nodes
        .into_iter()
        .map(|mut n| {
            n.id = reindex(n.id);
            n
        })
        .collect();

    let new_edges: Vec<PlanEdge> = branch
        .edges
        .into_iter()
        .map(|e| PlanEdge {
            from: reindex(e.from),
            to: reindex(e.to),
        })
        .collect();

    flow.nodes.extend(new_nodes);
    flow.edges.extend(new_edges);
    flow.nodes.sort_by_key(|n| n.id);
}

/// Metadata from the resolver used to guide variant specialization.
///
/// When present, `specialize_variants` prefers implementations with higher
/// quality scores and whose contracts are a superset of the required contracts.
/// When no metadata is available for a node, the function falls back to the
/// existing heuristic (pick the variant already set on the node).
#[derive(Debug, Clone, Default)]
pub struct SpecializationContext {
    /// Maps entry hash (or word for hash-less entries) to an overall quality score.
    pub entity_scores: HashMap<String, f64>,
    /// Maps entry hash (or word) to its contract predicates (pre/post conditions).
    pub entity_contracts: HashMap<String, Vec<String>>,
}

impl SpecializationContext {
    /// Build a `SpecializationContext` by querying all variants of every word
    /// referenced in `plan` from the given `resolver`.
    pub fn from_resolver(plan: &CompositionPlan, resolver: &Resolver) -> Self {
        let mut ctx = SpecializationContext::default();
        for flow in &plan.flows {
            for node in &flow.nodes {
                if let Ok(variants) = resolver.resolve_all_variants(&node.word) {
                    for entry in variants {
                        let key = entry
                            .hash
                            .clone()
                            .unwrap_or_else(|| variant_key(&entry.word, entry.variant.as_deref()));
                        ctx.entity_scores.insert(key.clone(), entry.overall_score);
                        let mut predicates = Vec::new();
                        if let Some(ref pre) = entry.pre {
                            predicates.push(pre.clone());
                        }
                        if let Some(ref post) = entry.post {
                            predicates.push(post.clone());
                        }
                        if !predicates.is_empty() {
                            ctx.entity_contracts.insert(key, predicates);
                        }
                    }
                }
            }
        }
        ctx
    }
}

fn variant_key(word: &str, variant: Option<&str>) -> String {
    match variant {
        Some(v) => format!("{}:{}", word, v),
        None => word.to_owned(),
    }
}

/// Per-node resolver metadata used to drive specialization decisions.
///
/// Callers (e.g. `nom-cli`) populate this from dictionary data and pass a
/// slice indexed by node `id` (or matched by `word`) into the specialization
/// pass.  `nom-planner` deliberately does **not** depend on `nom-dict`; the
/// caller bridges the two crates.
#[derive(Debug, Clone, Default)]
pub struct ResolverMetadata {
    /// The word this metadata belongs to (used for lookup by word).
    pub word: String,
    /// Entity kind from the dictionary (e.g. `"function"`, `"data"`,
    /// `"agent"`, `"module"`).  Drives kind-specific selection heuristics.
    pub kind: String,
    /// Contract predicates (pre- and post-conditions) for this entry.
    /// When non-empty, `specialize_variants` will insert a validation
    /// `PlanNode` before the node that carries these contracts.
    pub contracts: Vec<String>,
    /// Overall quality score in `[0.0, 1.0]`.  `None` means no score is
    /// available — fall back to heuristics.
    pub score: Option<f64>,
}

impl ResolverMetadata {
    /// Build a `ResolverMetadata` slice for every node word in `plan` by
    /// querying `resolver`.  Entries with no resolver hit are omitted.
    pub fn from_resolver(plan: &CompositionPlan, resolver: &Resolver) -> Vec<Self> {
        let mut out = Vec::new();
        for flow in &plan.flows {
            for node in &flow.nodes {
                if let Ok(variants) = resolver.resolve_all_variants(&node.word) {
                    for entry in &variants {
                        let mut contracts = Vec::new();
                        if let Some(ref pre) = entry.pre {
                            contracts.push(pre.clone());
                        }
                        if let Some(ref post) = entry.post {
                            contracts.push(post.clone());
                        }
                        out.push(ResolverMetadata {
                            word: entry.word.clone(),
                            kind: entry.kind.clone(),
                            contracts,
                            score: Some(entry.overall_score),
                        });
                    }
                }
            }
        }
        out
    }
}

/// Score adjustment applied per entity kind when selecting among variants.
///
/// * `"function"` — penalise low-performance candidates (latency-sensitive).
/// * `"data"` — penalise low-reliability candidates (correctness-sensitive).
/// * All other kinds — use raw `overall_score` unchanged.
fn kind_adjusted_score(kind: &str, base_score: f64, entry: &WordEntry) -> f64 {
    match kind {
        "function" => {
            // Functions are latency-sensitive; weight performance more heavily.
            base_score * 0.5 + entry.performance * 0.5
        }
        "data" => {
            // Data nodes are correctness-sensitive; weight reliability more heavily.
            base_score * 0.5 + entry.reliability * 0.5
        }
        _ => base_score,
    }
}

/// Specialize plan node variants using resolver metadata.
///
/// For each node in `flow` that has multiple candidate implementations
/// (resolved via all variants of the word), this pass:
/// 1. Picks the candidate with the highest `overall_score` from `ctx`.
/// 2. Among equal scores, prefers candidates whose contract predicates are a
///    superset of the currently-set node contracts (more guarantees is better).
/// 3. Falls back to leaving the node unchanged when no metadata is available.
///
/// When `resolver_meta` is supplied, additional kind-aware logic runs:
/// - Scores are adjusted per entity kind (see [`kind_adjusted_score`]).
/// - For each node whose matching `ResolverMetadata` has non-empty `contracts`,
///   a validation `PlanNode` is inserted immediately before that node in
///   `flow.nodes`.  The validation node has `word = "validate:<contract>"` and
///   carries no effects, so it is safe to fuse/elide in later passes if unused.
pub fn specialize_variants(
    flow: &mut FlowPlan,
    resolver: &Resolver,
    ctx: &SpecializationContext,
    resolver_meta: Option<&[ResolverMetadata]>,
) {
    // ── Phase 1: variant selection (existing + kind-aware) ────────────────────
    for node in &mut flow.nodes {
        let variants = match resolver.resolve_all_variants(&node.word) {
            Ok(v) if v.len() > 1 => v,
            _ => continue, // 0 or 1 variant — nothing to specialize
        };

        // Resolve the kind for this node from resolver_meta (if available).
        let node_kind = resolver_meta
            .and_then(|meta| meta.iter().find(|m| m.word == node.word))
            .map(|m| m.kind.as_str())
            .unwrap_or("");

        // Compute a score for each candidate variant.
        let best = variants.iter().max_by(|a, b| {
            let key_a = a
                .hash
                .clone()
                .unwrap_or_else(|| variant_key(&a.word, a.variant.as_deref()));
            let key_b = b
                .hash
                .clone()
                .unwrap_or_else(|| variant_key(&b.word, b.variant.as_deref()));

            let raw_a = ctx.entity_scores.get(&key_a).copied().unwrap_or(0.0);
            let raw_b = ctx.entity_scores.get(&key_b).copied().unwrap_or(0.0);

            let score_a = kind_adjusted_score(node_kind, raw_a, a);
            let score_b = kind_adjusted_score(node_kind, raw_b, b);

            // Primary: higher score wins.
            match score_a.partial_cmp(&score_b) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // Secondary: more contract predicates wins (superset preference).
                    let contracts_a = ctx
                        .entity_contracts
                        .get(&key_a)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let contracts_b = ctx
                        .entity_contracts
                        .get(&key_b)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    contracts_a.cmp(&contracts_b)
                }
                Some(ord) => ord,
            }
        });

        if let Some(winner) = best {
            // Only update when metadata actually guided the choice (score > 0).
            let key = winner
                .hash
                .clone()
                .unwrap_or_else(|| variant_key(&winner.word, winner.variant.as_deref()));
            if ctx.entity_scores.get(&key).copied().unwrap_or(0.0) > 0.0 {
                node.variant = winner.variant.clone();
                if let Some(ref t) = winner.input_type {
                    node.input_type = Some(t.clone());
                }
                if let Some(ref t) = winner.output_type {
                    node.output_type = Some(t.clone());
                }
            }
        }
    }

    // ── Phase 2: contract enforcement (requires resolver_meta) ────────────────
    //
    // For each node whose ResolverMetadata carries contracts, insert a
    // validation PlanNode *before* it.  We collect insertions first to avoid
    // mutating `flow.nodes` while iterating.
    let Some(meta) = resolver_meta else { return };

    // Build the list of (insertion_index, new_node) pairs.
    // We work in reverse index order so that earlier insertions don't shift
    // the positions of later ones.
    let mut insertions: Vec<(usize, PlanNode)> = Vec::new();
    // Choose an id base that won't collide with existing node ids.
    let max_id = flow.nodes.iter().map(|n| n.id).max().unwrap_or(0);
    let mut next_id = max_id + 1000; // generous gap to avoid collisions

    for (idx, node) in flow.nodes.iter().enumerate() {
        let node_meta = match meta.iter().find(|m| m.word == node.word) {
            Some(m) if !m.contracts.is_empty() => m,
            _ => continue,
        };

        // Deduplicate: skip if a validation node already precedes this one.
        let already_validated = idx > 0
            && flow.nodes[idx - 1]
                .word
                .starts_with(&format!("validate:{}", node.word));
        if already_validated {
            continue;
        }

        let contract_label = node_meta.contracts.join(";");
        let validation_node = PlanNode {
            id: next_id,
            word: format!("validate:{}", node.word),
            variant: None,
            input_type: node.input_type.clone(),
            output_type: node.input_type.clone(), // pass-through type
            // "validate" effect marks this node as having observable behavior
            // (it can abort the flow), preventing map-fusion from eliding it.
            effects: vec!["validate".to_owned()],
            impl_body: Some(format!(
                "// auto-generated contract check: {}",
                contract_label
            )),
            impl_language: Some("rust".to_owned()),
        };
        insertions.push((idx, validation_node));
        next_id += 1;
    }

    // Insert in reverse order so indices remain valid.
    for (idx, new_node) in insertions.into_iter().rev() {
        let val_id = new_node.id;
        let target_id = flow.nodes[idx].id;
        flow.nodes.insert(idx, new_node);

        // Rewire: any edge that pointed *to* target_id should now point to
        // the new validation node, and add an edge validation → target.
        for edge in &mut flow.edges {
            if edge.to == target_id {
                edge.to = val_id;
            }
        }
        flow.edges.push(PlanEdge {
            from: val_id,
            to: target_id,
        });
    }
}

/// Apply all optimization passes to every flow in a plan.
///
/// Pass order:
/// 1. `specialize_variants` — pick best implementation using resolver scores/contracts.
/// 2. `fuse_identity_nodes` — remove no-op pass-through nodes.
/// 3. `fuse_consecutive_maps` — compose adjacent pure map nodes.
/// 4. `collapse_single_branch` — inline trivial single-arm branches.
pub fn optimize_plan(plan: &mut CompositionPlan) {
    optimize_plan_with_context(plan, None, None, None);
}

/// Like `optimize_plan` but allows injecting a resolver, pre-built
/// `SpecializationContext`, and per-node `ResolverMetadata` for the
/// specialization pass.
///
/// `resolver_meta` is passed directly to `specialize_variants` to enable
/// kind-aware scoring and contract enforcement.  Pass `None` to run in pure
/// heuristic mode (backward-compatible with callers that don't supply metadata).
pub fn optimize_plan_with_context(
    plan: &mut CompositionPlan,
    resolver: Option<&Resolver>,
    ctx: Option<&SpecializationContext>,
    resolver_meta: Option<&[ResolverMetadata]>,
) {
    // Build context lazily if a resolver is provided but no pre-built context.
    let owned_ctx: Option<SpecializationContext>;
    let effective_ctx: Option<&SpecializationContext> = if ctx.is_some() {
        ctx
    } else if let Some(r) = resolver {
        owned_ctx = Some(SpecializationContext::from_resolver(plan, r));
        owned_ctx.as_ref()
    } else {
        None
    };

    for flow in &mut plan.flows {
        if let (Some(r), Some(c)) = (resolver, effective_ctx) {
            specialize_variants(flow, r, c, resolver_meta);
        }
        fuse_identity_nodes(flow);
        fuse_consecutive_maps(flow);
        collapse_single_branch(flow);
    }
}

fn sanitize_plan_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Ident(identifier) => identifier.name.clone(),
        Expr::Literal(Literal::Number(value)) => value.to_string(),
        Expr::Literal(Literal::Integer(value)) => value.to_string(),
        Expr::Literal(Literal::Text(value)) => format!("{value:?}"),
        Expr::Literal(Literal::Bool(value)) => value.to_string(),
        Expr::Literal(Literal::None) => "none".to_owned(),
        Expr::FieldAccess(left, field) => format!("{}.{}", expr_to_string(left), field.name),
        Expr::BinaryOp(left, op, right) => {
            let op_text = match op {
                nom_ast::BinOp::Add => "+",
                nom_ast::BinOp::Sub => "-",
                nom_ast::BinOp::Mul => "*",
                nom_ast::BinOp::Div => "/",
                nom_ast::BinOp::And => "and",
                nom_ast::BinOp::Or => "or",
                nom_ast::BinOp::Gt => ">",
                nom_ast::BinOp::Lt => "<",
                nom_ast::BinOp::Gte => ">=",
                nom_ast::BinOp::Lte => "<=",
                nom_ast::BinOp::Eq => "=",
                nom_ast::BinOp::Neq => "!=",
                nom_ast::BinOp::Mod => "%",
                nom_ast::BinOp::BitAnd => "&",
                nom_ast::BinOp::BitOr => "|",
            };
            format!(
                "{} {op_text} {}",
                expr_to_string(left),
                expr_to_string(right)
            )
        }
        Expr::Call(call) => {
            let args = call
                .args
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", call.callee.name)
        }
        // New expression types — stringify for plan metadata
        Expr::UnaryOp(op, inner) => {
            let op_str = match op {
                nom_ast::UnaryOp::Not => "!",
                nom_ast::UnaryOp::Neg => "-",
                nom_ast::UnaryOp::Ref => "&",
                nom_ast::UnaryOp::RefMut => "&mut ",
            };
            format!("{op_str}{}", expr_to_string(inner))
        }
        Expr::Index(base, idx) => format!("{}[{}]", expr_to_string(base), expr_to_string(idx)),
        Expr::MethodCall(obj, method, args) => {
            let args_str = args
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}.{}({args_str})", expr_to_string(obj), method.name)
        }
        Expr::Array(items) => {
            let inner = items
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{inner}]")
        }
        Expr::TupleExpr(items) => {
            let inner = items
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({inner})")
        }
        Expr::Await(inner) => format!("{}.await", expr_to_string(inner)),
        Expr::Cast(inner, _ty) => format!("{} as _", expr_to_string(inner)),
        Expr::Try(inner) => format!("{}?", expr_to_string(inner)),
        _ => "<expr>".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{
        AgentCapabilityStmt, AgentReceiveStmt, AgentScheduleStmt, AgentStateStmt,
        AgentSuperviseStmt, Classifier, Declaration, Expr, FlowChain, FlowStep, FlowStmt,
        GraphConstraintStmt, GraphEdgeStmt, GraphNodeStmt, GraphQueryExpr, GraphQueryStmt,
        Identifier, Literal, NomRef, SourceFile, Span, Statement,
    };
    use nom_concept::stages::run_pipeline;
    use nom_resolver::{Resolver, WordEntry};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn graph_word(name: &str, variant: Option<&str>) -> GraphQueryExpr {
        GraphQueryExpr::Ref(NomRef {
            word: Identifier::new(name, span()),
            variant: variant.map(|value| Identifier::new(value, span())),
            span: span(),
        })
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
        })
        .unwrap();
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
                qualifier: nom_ast::FlowQualifier::Once,
                chain: FlowChain {
                    steps: vec![FlowStep::Ref(NomRef {
                        word: Identifier::new("hash", span()),
                        variant: Some(Identifier::new("argon2", span())),
                        span: span(),
                    })],
                },
                on_fail: nom_ast::OnFailStrategy::Abort,
                span: span(),
            })],
            span: span(),
        };

        let source = make_source(decl);
        let plan = planner.plan_unchecked(&source).unwrap();
        assert_eq!(plan.flows.len(), 1);
        let flow = &plan.flows[0];
        assert_eq!(flow.name, "myflow");
        assert_eq!(flow.classifier, "flow");
        assert!(flow.agent.is_none());
        assert!(flow.graph.is_none());
        assert_eq!(flow.nodes.len(), 1);
        assert!(flow.effect_summary.contains(&"cpu".to_owned()));
    }

    #[test]
    fn nomiz_roundtrip() {
        let resolver = setup();
        let planner = Planner::new(&resolver);
        let source = SourceFile {
            path: None,
            locale: None,
            declarations: vec![],
        };
        let plan = planner.plan_unchecked(&source).unwrap();
        let nomiz = plan.to_nomiz().unwrap();
        let plan2 = CompositionPlan::from_nomiz(&nomiz).unwrap();
        assert_eq!(plan2.flows.len(), 0);
    }

    #[test]
    fn plan_from_pipeline_output_plans_nomtu_composition() {
        let resolver = setup();
        let planner = Planner::new(&resolver);
        let pipeline = run_pipeline(
            r#"the module password_pipeline composes
  the function hash matching "hash input" then
  the function hash matching "hash again"."#,
        )
        .expect("pipeline");

        let plan = planner.plan_from_pipeline_output(&pipeline).unwrap();

        assert_eq!(plan.flows.len(), 1);
        let flow = &plan.flows[0];
        assert_eq!(flow.name, "password_pipeline");
        assert_eq!(flow.classifier, "system");
        assert_eq!(flow.nodes.len(), 2);
        assert_eq!(flow.edges.len(), 1);
        assert_eq!(flow.nodes[0].word, "hash");
        assert_eq!(flow.nodes[1].word, "hash");
    }

    #[test]
    fn plan_agent_receive_and_schedule() {
        let resolver = setup();
        let planner = Planner::new(&resolver);

        let decl = Declaration {
            classifier: Classifier::Agent,
            name: Identifier::new("monitor", span()),
            statements: vec![
                Statement::AgentCapability(AgentCapabilityStmt {
                    capabilities: vec![
                        Identifier::new("network", span()),
                        Identifier::new("observe", span()),
                    ],
                    span: span(),
                }),
                Statement::AgentState(AgentStateStmt {
                    state: Identifier::new("active", span()),
                    span: span(),
                }),
                Statement::AgentSupervise(AgentSuperviseStmt {
                    strategy: Identifier::new("restart_on_failure", span()),
                    params: vec![(
                        Identifier::new("max_retries", span()),
                        Expr::Literal(Literal::Integer(3)),
                    )],
                    span: span(),
                }),
                Statement::AgentReceive(AgentReceiveStmt {
                    chain: FlowChain {
                        steps: vec![FlowStep::Ref(NomRef {
                            word: Identifier::new("hash", span()),
                            variant: Some(Identifier::new("argon2", span())),
                            span: span(),
                        })],
                    },
                    span: span(),
                }),
                Statement::AgentSchedule(AgentScheduleStmt {
                    interval: "5m".to_owned(),
                    action: FlowChain {
                        steps: vec![FlowStep::Ref(NomRef {
                            word: Identifier::new("hash", span()),
                            variant: Some(Identifier::new("argon2", span())),
                            span: span(),
                        })],
                    },
                    span: span(),
                }),
            ],
            span: span(),
        };

        let plan = planner.plan_unchecked(&make_source(decl)).unwrap();
        assert_eq!(plan.flows.len(), 2);
        assert_eq!(plan.flows[0].name, "monitor__receive");
        assert_eq!(plan.flows[1].name, "monitor__schedule__5m");
        assert_eq!(plan.flows[0].classifier, "agent");
        let agent = plan.flows[0].agent.as_ref().expect("agent metadata");
        assert_eq!(
            agent.capabilities,
            vec!["network".to_owned(), "observe".to_owned()]
        );
        assert_eq!(agent.state.as_deref(), Some("active"));
        assert_eq!(
            agent
                .supervision
                .as_ref()
                .map(|supervision| supervision.strategy.as_str()),
            Some("restart_on_failure")
        );
        assert!(plan.flows[0].graph.is_none());
    }

    #[test]
    fn plan_graph_query_as_flow() {
        let resolver = setup();
        let planner = Planner::new(&resolver);

        let decl = Declaration {
            classifier: Classifier::Graph,
            name: Identifier::new("social", span()),
            statements: vec![
                Statement::GraphNode(GraphNodeStmt {
                    name: Identifier::new("user", span()),
                    fields: vec![nom_ast::TypedParam {
                        name: Identifier::new("name", span()),
                        typ: Some(Identifier::new("text", span())),
                        span: span(),
                    }],
                    span: span(),
                }),
                Statement::GraphEdge(GraphEdgeStmt {
                    name: Identifier::new("follows", span()),
                    from_type: Identifier::new("user", span()),
                    to_type: Identifier::new("user", span()),
                    fields: vec![],
                    span: span(),
                }),
                Statement::GraphConstraint(GraphConstraintStmt {
                    name: Identifier::new("no_self_follow", span()),
                    expr: Expr::BinaryOp(
                        Box::new(Expr::FieldAccess(
                            Box::new(Expr::Ident(Identifier::new("follows", span()))),
                            Identifier::new("from", span()),
                        )),
                        nom_ast::BinOp::Neq,
                        Box::new(Expr::FieldAccess(
                            Box::new(Expr::Ident(Identifier::new("follows", span()))),
                            Identifier::new("to", span()),
                        )),
                    ),
                    span: span(),
                }),
                Statement::GraphQuery(GraphQueryStmt {
                    name: Identifier::new("friends_of", span()),
                    params: vec![nom_ast::TypedParam {
                        name: Identifier::new("user", span()),
                        typ: None,
                        span: span(),
                    }],
                    expr: graph_word("hash", Some("argon2")),
                    span: span(),
                }),
            ],
            span: span(),
        };

        let plan = planner.plan_unchecked(&make_source(decl)).unwrap();
        assert_eq!(plan.flows.len(), 2);
        assert_eq!(plan.flows[0].name, "social");
        let graph = plan.flows[0].graph.as_ref().expect("graph metadata");
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.constraints.len(), 1);
        assert_eq!(graph.queries.len(), 1);
        assert_eq!(plan.flows[1].name, "social__friends_of");
        assert_eq!(plan.flows[1].nodes.len(), 1);
        assert!(plan.flows[1].graph.is_none());
    }

    // ── ADOPT-3: Memory strategy inference tests ────────────────────────────

    fn make_plan_node(id: usize, word: &str) -> PlanNode {
        PlanNode {
            id,
            word: word.into(),
            variant: None,
            input_type: None,
            output_type: None,
            effects: vec![],
            impl_body: None,
            impl_language: None,
        }
    }

    fn make_test_flow(
        name: &str,
        nodes: Vec<PlanNode>,
        edges: Vec<PlanEdge>,
        branches: Vec<PlanBranch>,
    ) -> FlowPlan {
        FlowPlan {
            name: name.into(),
            classifier: "flow".into(),
            agent: None,
            graph: None,
            nodes,
            edges,
            branches,
            memory_strategy: MemoryStrategy::Stack,
            concurrency_strategy: ConcurrencyStrategy::Sequential,
            effect_summary: vec![],
            imperative_stmts: vec![],
            qualifier: "once".into(),
            on_fail: "abort".into(),
        }
    }

    #[test]
    fn empty_flow_infers_stack() {
        let resolver = setup();
        let flow = make_test_flow("empty", vec![], vec![], vec![]);
        assert_eq!(
            infer_memory_strategy(&flow, &resolver),
            MemoryStrategy::Stack
        );
    }

    #[test]
    fn linear_chain_infers_stack() {
        let resolver = setup();
        let flow = make_test_flow(
            "linear",
            vec![make_plan_node(0, "a"), make_plan_node(1, "b")],
            vec![PlanEdge { from: 0, to: 1 }],
            vec![],
        );
        assert_eq!(
            infer_memory_strategy(&flow, &resolver),
            MemoryStrategy::Stack
        );
    }

    #[test]
    fn cyclic_flow_infers_heap() {
        let resolver = setup();
        let flow = make_test_flow(
            "cyclic",
            vec![make_plan_node(0, "a"), make_plan_node(1, "b")],
            vec![PlanEdge { from: 0, to: 1 }, PlanEdge { from: 1, to: 0 }],
            vec![],
        );
        assert_eq!(
            infer_memory_strategy(&flow, &resolver),
            MemoryStrategy::Heap
        );
    }

    #[test]
    fn branching_flow_infers_mixed() {
        let resolver = setup();
        let flow = make_test_flow(
            "branching",
            vec![
                make_plan_node(0, "a"),
                make_plan_node(1, "b"),
                make_plan_node(2, "c"),
            ],
            vec![PlanEdge { from: 0, to: 1 }, PlanEdge { from: 0, to: 2 }],
            vec![PlanBranch {
                condition: "IfTrue".into(),
                nodes: vec![make_plan_node(3, "d")],
                edges: vec![],
            }],
        );
        assert_eq!(
            infer_memory_strategy(&flow, &resolver),
            MemoryStrategy::Mixed
        );
    }

    #[test]
    fn single_node_infers_stack() {
        let resolver = setup();
        let flow = make_test_flow("single", vec![make_plan_node(0, "a")], vec![], vec![]);
        assert_eq!(
            infer_memory_strategy(&flow, &resolver),
            MemoryStrategy::Stack
        );
    }

    // ── Optimization pass tests ───────────────────────────────────────────────

    fn make_identity_node(id: usize, ty: &str) -> PlanNode {
        PlanNode {
            id,
            word: format!("identity_{}", id),
            variant: None,
            input_type: Some(ty.to_owned()),
            output_type: Some(ty.to_owned()),
            effects: vec![],
            impl_body: None,
            impl_language: None,
        }
    }

    fn make_typed_node(id: usize, word: &str, input: &str, output: &str) -> PlanNode {
        PlanNode {
            id,
            word: word.to_owned(),
            variant: None,
            input_type: Some(input.to_owned()),
            output_type: Some(output.to_owned()),
            effects: vec![],
            impl_body: None,
            impl_language: None,
        }
    }

    fn make_effectful_node(id: usize, word: &str) -> PlanNode {
        PlanNode {
            id,
            word: word.to_owned(),
            variant: None,
            input_type: None,
            output_type: None,
            effects: vec!["io".to_owned()],
            impl_body: None,
            impl_language: None,
        }
    }

    #[test]
    fn identity_node_removal_removes_pass_through() {
        // Chain: real_node(0) → identity(1) → real_node(2)
        // After fuse_identity_nodes: real_node(0) → real_node(2)
        let nodes = vec![
            make_plan_node(0, "start"),
            make_identity_node(1, "text"),
            make_plan_node(2, "end"),
        ];
        let edges = vec![PlanEdge { from: 0, to: 1 }, PlanEdge { from: 1, to: 2 }];
        let mut flow = make_test_flow("test", nodes, edges, vec![]);

        fuse_identity_nodes(&mut flow);

        assert_eq!(flow.nodes.len(), 2, "identity node should be removed");
        assert!(!flow.nodes.iter().any(|n| n.word == "identity_1"));
        assert_eq!(flow.edges.len(), 1, "rewired edge should remain");
        assert_eq!(flow.edges[0].from, 0);
        assert_eq!(flow.edges[0].to, 2);
    }

    #[test]
    fn identity_node_removal_keeps_effectful_nodes() {
        // Effectful node must NOT be treated as identity.
        let nodes = vec![
            make_effectful_node(0, "writer"),
            make_effectful_node(1, "reader"),
        ];
        let edges = vec![PlanEdge { from: 0, to: 1 }];
        let mut flow = make_test_flow("test", nodes, edges, vec![]);

        fuse_identity_nodes(&mut flow);

        assert_eq!(flow.nodes.len(), 2, "effectful nodes must be preserved");
        assert_eq!(flow.edges.len(), 1);
    }

    #[test]
    fn fuse_consecutive_maps_composes_two_pure_nodes() {
        // Chain: parse(0) → validate(1)
        // After fuse_consecutive_maps: parse∘validate(0)
        let nodes = vec![
            make_typed_node(0, "parse", "bytes", "text"),
            make_typed_node(1, "validate", "text", "bool"),
        ];
        let edges = vec![PlanEdge { from: 0, to: 1 }];
        let mut flow = make_test_flow("test", nodes, edges, vec![]);

        fuse_consecutive_maps(&mut flow);

        assert_eq!(flow.nodes.len(), 1, "two nodes should be fused into one");
        let fused = &flow.nodes[0];
        assert_eq!(fused.word, "parse∘validate");
        assert_eq!(fused.input_type.as_deref(), Some("bytes"));
        assert_eq!(fused.output_type.as_deref(), Some("bool"));
        assert!(flow.edges.is_empty(), "internal edge should be removed");
    }

    #[test]
    fn fuse_consecutive_maps_skips_effectful_nodes() {
        // Effectful nodes must NOT be fused.
        let nodes = vec![
            make_effectful_node(0, "fetch"),
            make_effectful_node(1, "store"),
        ];
        let edges = vec![PlanEdge { from: 0, to: 1 }];
        let mut flow = make_test_flow("test", nodes, edges, vec![]);

        fuse_consecutive_maps(&mut flow);

        assert_eq!(flow.nodes.len(), 2, "effectful nodes must not be fused");
        assert_eq!(flow.edges.len(), 1);
    }

    #[test]
    fn collapse_single_branch_inlines_nodes() {
        // A branch with one arm whose nodes should be promoted to the main flow.
        let branch = PlanBranch {
            condition: "IfTrue".to_owned(),
            nodes: vec![make_plan_node(0, "branch_op")],
            edges: vec![],
        };
        let nodes = vec![make_plan_node(0, "main_op")];
        let mut flow = make_test_flow("test", nodes, vec![], vec![branch]);

        assert_eq!(flow.branches.len(), 1);
        collapse_single_branch(&mut flow);

        assert_eq!(flow.branches.len(), 0, "branch should be collapsed");
        assert_eq!(
            flow.nodes.len(),
            2,
            "branch node should be promoted to main flow"
        );
        // The promoted node's word must be preserved.
        assert!(flow.nodes.iter().any(|n| n.word == "branch_op"));
    }

    #[test]
    fn collapse_single_branch_noop_when_no_branches() {
        let nodes = vec![make_plan_node(0, "a"), make_plan_node(1, "b")];
        let edges = vec![PlanEdge { from: 0, to: 1 }];
        let mut flow = make_test_flow("test", nodes, edges, vec![]);

        collapse_single_branch(&mut flow);

        assert_eq!(flow.nodes.len(), 2);
        assert_eq!(flow.edges.len(), 1);
        assert!(flow.branches.is_empty());
    }

    #[test]
    fn optimize_plan_runs_all_passes() {
        // Build a CompositionPlan directly with pure nodes (no effects) so that
        // fuse_consecutive_maps can merge them. Using the resolver's `hash:argon2`
        // entry would not work here because it carries a `cpu` effect.
        let pure_a = make_typed_node(0, "parse", "bytes", "text");
        let pure_b = make_typed_node(1, "validate", "text", "bool");
        let flow = make_test_flow(
            "opt_flow",
            vec![pure_a, pure_b],
            vec![PlanEdge { from: 0, to: 1 }],
            vec![],
        );
        let mut plan = CompositionPlan {
            source_path: None,
            flows: vec![flow],
            nomiz: String::new(),
        };

        // Before optimization: two nodes.
        assert_eq!(plan.flows[0].nodes.len(), 2);

        optimize_plan(&mut plan);

        // After fuse_consecutive_maps: the two pure nodes are fused into one.
        assert_eq!(
            plan.flows[0].nodes.len(),
            1,
            "consecutive pure nodes should be fused by optimize_plan"
        );
        assert_eq!(plan.flows[0].nodes[0].word, "parse∘validate");
    }

    // ── Specialization pass tests ─────────────────────────────────────────────

    /// Build a resolver with two variants of the same word, one higher-scored.
    fn setup_multi_variant() -> Resolver {
        let r = Resolver::open_in_memory().unwrap();
        // low-quality variant
        r.upsert(&WordEntry {
            word: "encode".to_owned(),
            variant: Some("basic".to_owned()),
            input_type: Some("bytes".to_owned()),
            output_type: Some("text".to_owned()),
            overall_score: 0.3,
            ..WordEntry::default()
        })
        .unwrap();
        // high-quality variant
        r.upsert(&WordEntry {
            word: "encode".to_owned(),
            variant: Some("fast".to_owned()),
            input_type: Some("bytes".to_owned()),
            output_type: Some("text".to_owned()),
            overall_score: 0.9,
            ..WordEntry::default()
        })
        .unwrap();
        r
    }

    #[test]
    fn specialize_variants_picks_highest_score() {
        let resolver = setup_multi_variant();

        // Build context that includes both variants.
        let node = PlanNode {
            id: 0,
            word: "encode".to_owned(),
            variant: Some("basic".to_owned()), // starts with low-quality variant
            input_type: Some("bytes".to_owned()),
            output_type: Some("text".to_owned()),
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("enc_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        specialize_variants(&mut flow, &resolver, &ctx, None);

        assert_eq!(
            flow.nodes[0].variant.as_deref(),
            Some("fast"),
            "should pick the higher-scored variant"
        );
    }

    #[test]
    fn specialize_variants_fallback_when_no_scores() {
        let resolver = Resolver::open_in_memory().unwrap();
        // One variant with score = 0 (no metadata → fallback)
        resolver
            .upsert(&WordEntry {
                word: "noop".to_owned(),
                variant: Some("v1".to_owned()),
                overall_score: 0.0,
                ..WordEntry::default()
            })
            .unwrap();

        let node = PlanNode {
            id: 0,
            word: "noop".to_owned(),
            variant: Some("v1".to_owned()),
            input_type: None,
            output_type: None,
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("noop_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        specialize_variants(&mut flow, &resolver, &ctx, None);

        // Score is 0 — no update; variant stays unchanged.
        assert_eq!(
            flow.nodes[0].variant.as_deref(),
            Some("v1"),
            "variant must be unchanged when no metadata guides the choice"
        );
    }

    #[test]
    fn specialize_variants_prefers_more_contracts() {
        let resolver = Resolver::open_in_memory().unwrap();
        // Two variants with equal scores but different contract coverage.
        resolver
            .upsert(&WordEntry {
                word: "sign".to_owned(),
                variant: Some("simple".to_owned()),
                overall_score: 0.8,
                pre: None,
                post: None,
                ..WordEntry::default()
            })
            .unwrap();
        resolver
            .upsert(&WordEntry {
                word: "sign".to_owned(),
                variant: Some("verified".to_owned()),
                overall_score: 0.8,
                pre: Some("input_not_empty".to_owned()),
                post: Some("output_is_valid_signature".to_owned()),
                ..WordEntry::default()
            })
            .unwrap();

        let node = PlanNode {
            id: 0,
            word: "sign".to_owned(),
            variant: Some("simple".to_owned()),
            input_type: None,
            output_type: None,
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("sign_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        specialize_variants(&mut flow, &resolver, &ctx, None);

        assert_eq!(
            flow.nodes[0].variant.as_deref(),
            Some("verified"),
            "should prefer the variant with more contract predicates when scores are equal"
        );
    }

    // ── ResolverMetadata-backed specialization tests ──────────────────────────

    /// Kind = "function": the pass should weight performance, so the candidate
    /// with equal `overall_score` but higher `performance` wins.
    #[test]
    fn specialize_variants_function_kind_prefers_performance() {
        let resolver = Resolver::open_in_memory().unwrap();
        // Both variants have the same overall_score but different performance.
        resolver
            .upsert(&WordEntry {
                word: "compress".to_owned(),
                variant: Some("slow".to_owned()),
                overall_score: 0.8,
                performance: 0.4,
                reliability: 0.9,
                ..WordEntry::default()
            })
            .unwrap();
        resolver
            .upsert(&WordEntry {
                word: "compress".to_owned(),
                variant: Some("fast".to_owned()),
                overall_score: 0.8,
                performance: 0.9,
                reliability: 0.7,
                ..WordEntry::default()
            })
            .unwrap();

        let node = PlanNode {
            id: 0,
            word: "compress".to_owned(),
            variant: Some("slow".to_owned()),
            input_type: None,
            output_type: None,
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("compress_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        let meta = vec![ResolverMetadata {
            word: "compress".to_owned(),
            kind: "function".to_owned(),
            contracts: vec![],
            score: Some(0.8),
        }];
        specialize_variants(&mut flow, &resolver, &ctx, Some(&meta));

        assert_eq!(
            flow.nodes[0].variant.as_deref(),
            Some("fast"),
            "function kind should prefer the higher-performance variant"
        );
    }

    /// Kind = "data": the pass should weight reliability, so the candidate
    /// with equal `overall_score` but higher `reliability` wins.
    #[test]
    fn specialize_variants_data_kind_prefers_reliability() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .upsert(&WordEntry {
                word: "store".to_owned(),
                variant: Some("fast".to_owned()),
                overall_score: 0.8,
                performance: 0.9,
                reliability: 0.5,
                ..WordEntry::default()
            })
            .unwrap();
        resolver
            .upsert(&WordEntry {
                word: "store".to_owned(),
                variant: Some("safe".to_owned()),
                overall_score: 0.8,
                performance: 0.5,
                reliability: 0.95,
                ..WordEntry::default()
            })
            .unwrap();

        let node = PlanNode {
            id: 0,
            word: "store".to_owned(),
            variant: Some("fast".to_owned()),
            input_type: None,
            output_type: None,
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("store_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        let meta = vec![ResolverMetadata {
            word: "store".to_owned(),
            kind: "data".to_owned(),
            contracts: vec![],
            score: Some(0.8),
        }];
        specialize_variants(&mut flow, &resolver, &ctx, Some(&meta));

        assert_eq!(
            flow.nodes[0].variant.as_deref(),
            Some("safe"),
            "data kind should prefer the higher-reliability variant"
        );
    }

    /// When ResolverMetadata carries contracts, a validation node is inserted
    /// before the target node.
    #[test]
    fn specialize_variants_inserts_validation_node_for_contracts() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .upsert(&WordEntry {
                word: "transfer".to_owned(),
                variant: Some("v1".to_owned()),
                overall_score: 0.9,
                pre: Some("balance_sufficient".to_owned()),
                ..WordEntry::default()
            })
            .unwrap();

        let node = PlanNode {
            id: 0,
            word: "transfer".to_owned(),
            variant: Some("v1".to_owned()),
            input_type: Some("amount".to_owned()),
            output_type: Some("receipt".to_owned()),
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("transfer_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        let meta = vec![ResolverMetadata {
            word: "transfer".to_owned(),
            kind: "function".to_owned(),
            contracts: vec!["balance_sufficient".to_owned()],
            score: Some(0.9),
        }];
        specialize_variants(&mut flow, &resolver, &ctx, Some(&meta));

        assert_eq!(
            flow.nodes.len(),
            2,
            "a validation node should be inserted before the transfer node"
        );
        assert!(
            flow.nodes[0].word.starts_with("validate:transfer"),
            "first node should be the validation node, got: {}",
            flow.nodes[0].word
        );
        assert_eq!(
            flow.nodes[1].word, "transfer",
            "original node should follow the validation node"
        );
        // The validation → transfer edge must exist.
        let val_id = flow.nodes[0].id;
        let transfer_id = flow.nodes[1].id;
        assert!(
            flow.edges
                .iter()
                .any(|e| e.from == val_id && e.to == transfer_id),
            "validation node must have an edge to the transfer node"
        );
    }

    /// Without resolver_meta the pass behaves exactly as before (backward compat).
    #[test]
    fn specialize_variants_no_meta_is_backward_compatible() {
        let resolver = setup_multi_variant();
        let node = PlanNode {
            id: 0,
            word: "encode".to_owned(),
            variant: Some("basic".to_owned()),
            input_type: Some("bytes".to_owned()),
            output_type: Some("text".to_owned()),
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let mut flow = make_test_flow("enc_flow", vec![node], vec![], vec![]);
        let plan = CompositionPlan {
            source_path: None,
            flows: vec![flow.clone()],
            nomiz: String::new(),
        };
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        // Pass None for resolver_meta — must still pick the higher-scored variant.
        specialize_variants(&mut flow, &resolver, &ctx, None);

        assert_eq!(
            flow.nodes[0].variant.as_deref(),
            Some("fast"),
            "without resolver_meta should still pick highest score"
        );
        assert_eq!(
            flow.nodes.len(),
            1,
            "no validation nodes should be inserted without resolver_meta"
        );
    }

    /// `optimize_plan_with_context` threads resolver_meta correctly.
    #[test]
    fn optimize_plan_with_context_threads_resolver_meta() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .upsert(&WordEntry {
                word: "verify".to_owned(),
                variant: Some("v1".to_owned()),
                overall_score: 0.85,
                pre: Some("input_valid".to_owned()),
                ..WordEntry::default()
            })
            .unwrap();

        let node = PlanNode {
            id: 0,
            word: "verify".to_owned(),
            variant: Some("v1".to_owned()),
            input_type: Some("data".to_owned()),
            output_type: Some("bool".to_owned()),
            effects: vec![],
            impl_body: None,
            impl_language: None,
        };
        let flow = make_test_flow("verify_flow", vec![node], vec![], vec![]);
        let mut plan = CompositionPlan {
            source_path: None,
            flows: vec![flow],
            nomiz: String::new(),
        };
        let meta = vec![ResolverMetadata {
            word: "verify".to_owned(),
            kind: "function".to_owned(),
            contracts: vec!["input_valid".to_owned()],
            score: Some(0.85),
        }];
        let ctx = SpecializationContext::from_resolver(&plan, &resolver);
        optimize_plan_with_context(&mut plan, Some(&resolver), Some(&ctx), Some(&meta));

        // Validation node should have been inserted.
        assert_eq!(
            plan.flows[0].nodes.len(),
            2,
            "optimize_plan_with_context should thread resolver_meta and insert validation node"
        );
        assert!(plan.flows[0].nodes[0].word.starts_with("validate:verify"));
    }
}
