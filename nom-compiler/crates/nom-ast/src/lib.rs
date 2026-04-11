//! nom-ast: Abstract Syntax Tree types for the Nom programming language.
//!
//! Nom uses writing-style syntax: classifiers start declarations,
//! blank lines separate them. No braces, no semicolons.
//!
//! Design inspired by studying: Rust (HIR/MIR), Go (SSA), Swift (SIL),
//! Python (ast module), TypeScript (ts.Node).

use serde::{Deserialize, Serialize};

/// A complete .nom source file — a collection of declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: Option<String>,
    pub locale: Option<String>,
    pub declarations: Vec<Declaration>,
}

/// A declaration starts with a classifier keyword.
/// Everything until the next classifier or blank line belongs to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Declaration {
    pub classifier: Classifier,
    pub name: Identifier,
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// The 10 classifier keywords that start declarations.
/// Inspired by Vietnamese noun classifiers (con, cái, người, etc.)
/// but using English words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Classifier {
    System,
    Flow,
    Store,
    Graph,
    Agent,
    Test,
    Nom,
    Gate,
    Pool,
    View,
}

impl Classifier {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "system" => Some(Self::System),
            "flow" => Some(Self::Flow),
            "store" => Some(Self::Store),
            "graph" => Some(Self::Graph),
            "agent" => Some(Self::Agent),
            "test" => Some(Self::Test),
            "nom" => Some(Self::Nom),
            "gate" => Some(Self::Gate),
            "pool" => Some(Self::Pool),
            "view" => Some(Self::View),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Flow => "flow",
            Self::Store => "store",
            Self::Graph => "graph",
            Self::Agent => "agent",
            Self::Test => "test",
            Self::Nom => "nom",
            Self::Gate => "gate",
            Self::Pool => "pool",
            Self::View => "view",
        }
    }
}

/// Statements within a declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    /// need hash::argon2 where security>0.9
    Need(NeedStmt),
    /// require latency<50ms
    Require(RequireStmt),
    /// effects only [network database]
    Effects(EffectsStmt),
    /// flow request->hash->store->response
    Flow(FlowStmt),
    /// describe "convert data into irreversible string"
    Describe(DescribeStmt),
    /// contract block (inside nom declarations)
    Contract(ContractStmt),
    /// implement rust { ... }
    Implement(ImplementStmt),
    /// Test statements: given/when/then/and
    Given(TestGivenStmt),
    When(TestWhenStmt),
    Then(TestThenStmt),
    And(TestAndStmt),

    // ── Graph-native statements ─────────────────────────────────────────────
    /// node user(name text, age number)
    GraphNode(GraphNodeStmt),
    /// edge follows(from user, to user, weight real)
    GraphEdge(GraphEdgeStmt),
    /// query friends_of(user) = user->follows->user
    GraphQuery(GraphQueryStmt),
    /// constraint no_self_follow = follows.from != follows.to
    GraphConstraint(GraphConstraintStmt),

    // ── Agent-native statements ─────────────────────────────────────────────
    /// capability [network observe]
    AgentCapability(AgentCapabilityStmt),
    /// supervise restart_on_failure max_retries=3
    AgentSupervise(AgentSuperviseStmt),
    /// receive message->classify->route
    AgentReceive(AgentReceiveStmt),
    /// state active
    AgentState(AgentStateStmt),
    /// schedule every 5m check_health
    AgentSchedule(AgentScheduleStmt),
}

/// need hash::argon2 where security>0.9
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedStmt {
    pub reference: NomRef,
    pub constraint: Option<Constraint>,
    pub span: Span,
}

/// A reference to a .nomtu word, optionally specialized.
/// hash         → word only
/// hash::argon2 → word with variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomRef {
    pub word: Identifier,
    pub variant: Option<Identifier>,
    pub span: Span,
}

/// require latency<50ms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequireStmt {
    pub constraint: Constraint,
    pub span: Span,
}

/// effects only [network database]
/// effects good [cachehit]
/// effects bad [timeout]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectsStmt {
    pub modifier: Option<EffectModifier>,
    pub effects: Vec<Identifier>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectModifier {
    Only,
    Good,
    Bad,
}

/// flow request->hash->store->response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStmt {
    pub chain: FlowChain,
    pub span: Span,
}

/// A chain of steps connected by ->
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowChain {
    pub steps: Vec<FlowStep>,
}

/// A single step in a flow chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowStep {
    /// A reference to a .nomtu word
    Ref(NomRef),
    /// A literal value (e.g., "hello world")
    Literal(Literal),
    /// A branch fork: {iftrue->X, iffalse->Y}
    Branch(BranchBlock),
    /// A function call: query(param)
    Call(CallExpr),
}

/// Branch fork in a flow: {iftrue->response200, iffalse->response404}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchBlock {
    pub arms: Vec<BranchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchArm {
    pub condition: BranchCondition,
    pub chain: FlowChain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchCondition {
    IfTrue,
    IfFalse,
    Named, // for custom branch labels
}

/// A constraint expression: security>0.9, latency<50ms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub left: Expr,
    pub op: CompareOp,
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
}

/// describe "convert data into irreversible string"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeStmt {
    pub text: String,
    pub span: Span,
}

/// contract block inside nom declarations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractStmt {
    pub inputs: Vec<TypedParam>,
    pub outputs: Vec<TypedParam>,
    pub effects: Vec<Identifier>,
    pub preconditions: Vec<Expr>,
    pub postconditions: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedParam {
    pub name: Identifier,
    pub typ: Option<Identifier>,
    pub span: Span,
}

/// implement rust { ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementStmt {
    pub language: String,
    pub code: String,
    pub span: Span,
}

// Test statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGivenStmt {
    pub subject: Identifier,
    pub config: Vec<(Identifier, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWhenStmt {
    pub action: Identifier,
    pub config: Vec<(Identifier, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestThenStmt {
    pub assertion: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAndStmt {
    pub assertion: Expr,
    pub span: Span,
}

// ── Graph-native statement types ─────────────────────────────────────────────

/// node user(name text, age number)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeStmt {
    pub name: Identifier,
    pub fields: Vec<TypedParam>,
    pub span: Span,
}

/// edge follows(from user, to user, weight real)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeStmt {
    pub name: Identifier,
    pub from_type: Identifier,
    pub to_type: Identifier,
    pub fields: Vec<TypedParam>,
    pub span: Span,
}

/// query friends_of(user) = user->follows->user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryStmt {
    pub name: Identifier,
    pub params: Vec<TypedParam>,
    pub expr: FlowChain,
    pub span: Span,
}

/// constraint no_self_follow = follows.from != follows.to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConstraintStmt {
    pub name: Identifier,
    pub expr: Expr,
    pub span: Span,
}

// ── Agent-native statement types ─────────────────────────────────────────────

/// capability [network observe]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilityStmt {
    pub capabilities: Vec<Identifier>,
    pub span: Span,
}

/// supervise restart_on_failure max_retries=3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSuperviseStmt {
    pub strategy: Identifier,
    pub params: Vec<(Identifier, Expr)>,
    pub span: Span,
}

/// receive message->classify->route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReceiveStmt {
    pub chain: FlowChain,
    pub span: Span,
}

/// state active
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateStmt {
    pub state: Identifier,
    pub span: Span,
}

/// schedule every 5m check_health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScheduleStmt {
    pub interval: String,
    pub action: FlowChain,
    pub span: Span,
}

/// A function call: query(param1, param2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Identifier,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// An expression (used in constraints, assertions, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Ident(Identifier),
    Literal(Literal),
    FieldAccess(Box<Expr>, Identifier),
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    Call(CallExpr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
}

/// A literal value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    Number(f64),
    Integer(i64),
    Text(String),
    Bool(bool),
    None,
}

/// An identifier (word name, variable, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

impl Identifier {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

/// Source location for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifier_roundtrip() {
        for name in [
            "system", "flow", "store", "graph", "agent", "test", "nom", "gate", "pool", "view",
        ] {
            let c = Classifier::from_str(name).unwrap();
            assert_eq!(c.as_str(), name);
        }
    }

    #[test]
    fn unknown_classifier_returns_none() {
        assert!(Classifier::from_str("unknown").is_none());
        assert!(Classifier::from_str("if").is_none());
        assert!(Classifier::from_str("class").is_none());
    }
}
