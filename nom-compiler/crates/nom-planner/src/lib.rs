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
use thiserror::Error;

fn default_on_fail() -> String {
    "abort".to_owned()
}

fn on_fail_strategy_to_string(strategy: &OnFailStrategy) -> String {
    match strategy {
        OnFailStrategy::Abort => "abort".to_owned(),
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
        let memory_strategy = if all_nodes.iter().any(|n| {
            n.output_type
                .as_deref()
                .map(|t| t.contains("bytes") || t.contains("buffer"))
                .unwrap_or(false)
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
            let args_str = args.iter().map(expr_to_string).collect::<Vec<_>>().join(", ");
            format!("{}.{}({args_str})", expr_to_string(obj), method.name)
        }
        Expr::Array(items) => {
            let inner = items.iter().map(expr_to_string).collect::<Vec<_>>().join(", ");
            format!("[{inner}]")
        }
        Expr::TupleExpr(items) => {
            let inner = items.iter().map(expr_to_string).collect::<Vec<_>>().join(", ");
            format!("({inner})")
        }
        Expr::Await(inner) => format!("{}.await", expr_to_string(inner)),
        Expr::Cast(inner, _ty) => format!("{} as _", expr_to_string(inner)),
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
}
