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

    // ── Imperative statements (general-purpose) ────────────────────────────
    /// let x: type = value
    Let(LetStmt),
    /// x = value  (reassignment)
    Assign(AssignStmt),
    /// if/else expression
    If(IfExpr),
    /// for loop
    For(ForStmt),
    /// while loop
    While(WhileStmt),
    /// match expression
    Match(MatchExpr),
    /// return [expr]
    Return(Option<Expr>),
    /// fn name(params) -> type { body }
    FnDef(FnDef),
    /// struct Name { fields }
    StructDef(StructDef),
    /// enum Name { variants }
    EnumDef(EnumDef),
    /// Bare expression (function call, etc.)
    ExprStmt(Expr),
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

/// Flow qualifiers (inspired by Vietnamese aspect markers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowQualifier {
    /// Default: runs once, completes (Vietnamese: đã - completed)
    Once,
    /// Streaming: produces values over time (Vietnamese: đang - ongoing)
    Stream,
    /// Scheduled: runs at a future time or interval (Vietnamese: sẽ - prospective)
    Scheduled,
}

impl Default for FlowQualifier {
    fn default() -> Self {
        Self::Once
    }
}

impl FlowQualifier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Stream => "stream",
            Self::Scheduled => "scheduled",
        }
    }
}

/// Fault handling strategy for a flow (inspired by Erlang/OTP supervision)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnFailStrategy {
    /// Abort the entire flow on failure (default)
    Abort,
    /// Restart from a specific node in the flow
    RestartFrom(Identifier),
    /// Retry the failed node N times before aborting
    Retry(u32),
    /// Skip the failed node and continue
    Skip,
    /// Escalate to parent flow
    Escalate,
}

impl Default for OnFailStrategy {
    fn default() -> Self {
        Self::Abort
    }
}

/// flow request->hash->store->response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStmt {
    pub qualifier: FlowQualifier,
    pub chain: FlowChain,
    #[serde(default)]
    pub on_fail: OnFailStrategy,
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
    pub label: Option<String>,
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
    pub expr: GraphQueryExpr,
    pub span: Span,
}

/// Graph query expressions form a recursive traversal/set-algebra tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphQueryExpr {
    Ref(NomRef),
    Traverse(GraphTraverseExpr),
    SetOp(GraphSetExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphTraverseExpr {
    pub source: Box<GraphQueryExpr>,
    pub edge: NomRef,
    pub target: Box<GraphQueryExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSetExpr {
    pub op: GraphSetOp,
    pub operands: Vec<GraphQueryExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphSetOp {
    Union,
    Intersection,
    Difference,
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

// ── Memory allocation hints (ADOPT-3: Zig-inspired explicit allocator) ──────

/// Memory allocation hint for a .nomtu implementation.
/// Inspired by Zig's explicit allocator passing.
/// The compiler uses the flow graph topology to validate and infer optimal strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryHint {
    /// Stack allocation — no heap, value-type semantics (default for small types)
    Stack,
    /// Arena allocation — bulk alloc, bulk free (optimal for linear flow chains)
    Arena,
    /// Pool allocation — pre-allocated fixed-size blocks (optimal for repeated flows)
    Pool,
    /// Heap allocation — general purpose, GC or refcounted (fallback)
    Heap,
    /// Compiler decides based on flow graph analysis
    Auto,
}

impl Default for MemoryHint {
    fn default() -> Self {
        Self::Auto
    }
}

// ── Persistent immutable collections (ADOPT-8: Clojure-inspired) ────────────

/// Marker for persistent (structurally-shared) collection types.
/// Inspired by Clojure's persistent data structures (HAMTs).
/// When a .nomtu contract specifies `collection: persistent`, the compiler
/// guarantees that data passed between flow nodes is never mutated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionKind {
    /// Standard mutable collection (default)
    Mutable,
    /// Persistent immutable collection with structural sharing
    Persistent,
    /// Frozen: immutable snapshot of a mutable collection
    Frozen,
}

impl Default for CollectionKind {
    fn default() -> Self {
        Self::Mutable
    }
}

// ── Range constraints (ADOPT-9: Ada-inspired range subtypes) ────────────────

/// A range constraint on a value: lower <= value <= upper
/// Used in contract pre/post conditions for compile-time range checking.
/// Inspired by Ada's range-constrained subtypes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeConstraint {
    /// The value being constrained (e.g., "security_score", "latency_ms")
    pub target: Identifier,
    /// Lower bound (inclusive), None means no lower bound
    pub lower: Option<Literal>,
    /// Upper bound (inclusive), None means no upper bound
    pub upper: Option<Literal>,
    pub span: Span,
}

// ── Type system ─────────────────────────────────────────────────────────────

/// A type annotation: `text`, `number`, `list[text]`, `fn(a: text) -> number`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeExpr {
    /// Simple named type: `text`, `number`, `bool`, `bytes`
    Named(Identifier),
    /// Generic type: `list[text]`, `map[text, number]`, `option[text]`
    Generic(Identifier, Vec<TypeExpr>),
    /// Function type: `fn(text, number) -> bool`
    Function { params: Vec<TypeExpr>, ret: Box<TypeExpr> },
    /// Tuple type: `(text, number)`
    Tuple(Vec<TypeExpr>),
    /// Reference type: `&text`, `&mut text`
    Ref { mutable: bool, inner: Box<TypeExpr> },
    /// The unit type: `()`
    Unit,
}

// ── Imperative statements ───────────────────────────────────────────────────

/// let x: text = "hello"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetStmt {
    pub name: Identifier,
    pub mutable: bool,
    pub type_ann: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

/// x = new_value  (assignment to existing binding)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignStmt {
    pub target: Expr,
    pub value: Expr,
    pub span: Span,
}

/// if condition { ... } else if ... { ... } else { ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_body: Block,
    pub else_ifs: Vec<(Expr, Block)>,
    pub else_body: Option<Block>,
    pub span: Span,
}

/// for item in iterable { ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForStmt {
    pub binding: Identifier,
    pub iterable: Expr,
    pub body: Block,
    pub span: Span,
}

/// while condition { ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

/// match expr { pattern => body, ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExpr {
    pub subject: Box<Expr>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
}

/// Pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    /// Wildcard: `_`
    Wildcard,
    /// Literal match: `42`, `"hello"`, `true`
    Literal(Literal),
    /// Binding: `x`
    Binding(Identifier),
    /// Variant: `Some(x)`, `Err(e)`
    Variant(Identifier, Vec<Pattern>),
}

/// A block of statements (imperative body)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub stmts: Vec<BlockStmt>,
    pub span: Span,
}

/// Statements that can appear inside a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockStmt {
    Let(LetStmt),
    Assign(AssignStmt),
    Expr(Expr),
    If(IfExpr),
    For(ForStmt),
    While(WhileStmt),
    Match(MatchExpr),
    Return(Option<Expr>),
    Break,
    Continue,
}

/// fn name(param: type, ...) -> return_type { body }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnDef {
    pub name: Identifier,
    pub params: Vec<FnParam>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub is_async: bool,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnParam {
    pub name: Identifier,
    pub type_ann: TypeExpr,
}

/// struct Name { field: type, ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub name: Identifier,
    pub fields: Vec<StructField>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub name: Identifier,
    pub type_ann: TypeExpr,
    pub is_pub: bool,
}

/// enum Name { Variant1, Variant2(type), ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: Identifier,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: Identifier,
    pub fields: Vec<TypeExpr>,
}

// ── Expressions (extended) ──────────────────────────────────────────────────

/// A function call: query(param1, param2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Identifier,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// An expression (used in constraints, assertions, and imperative code)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Ident(Identifier),
    Literal(Literal),
    FieldAccess(Box<Expr>, Identifier),
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    Call(CallExpr),
    /// Unary operators: !expr, -expr
    UnaryOp(UnaryOp, Box<Expr>),
    /// Index: expr[index]
    Index(Box<Expr>, Box<Expr>),
    /// Method call: expr.method(args)
    MethodCall(Box<Expr>, Identifier, Vec<Expr>),
    /// If expression (returns a value)
    IfExpr(Box<IfExpr>),
    /// Match expression (returns a value)
    MatchExpr(Box<MatchExpr>),
    /// Block expression: { stmts; tail_expr }
    Block(Box<Block>),
    /// Closure: |params| body
    Closure(Vec<FnParam>, Box<Expr>),
    /// Array literal: [1, 2, 3]
    Array(Vec<Expr>),
    /// Tuple: (a, b, c)
    TupleExpr(Vec<Expr>),
    /// Await: expr.await
    Await(Box<Expr>),
    /// Type cast: expr as Type
    Cast(Box<Expr>, Box<TypeExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
    BitAnd,
    BitOr,
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

    #[test]
    fn collection_kind_default_is_mutable() {
        assert_eq!(CollectionKind::default(), CollectionKind::Mutable);
    }

    #[test]
    fn memory_hint_default_is_auto() {
        assert_eq!(MemoryHint::default(), MemoryHint::Auto);
    }

    #[test]
    fn memory_hint_variants_distinct() {
        let variants = [
            MemoryHint::Stack,
            MemoryHint::Arena,
            MemoryHint::Pool,
            MemoryHint::Heap,
            MemoryHint::Auto,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
