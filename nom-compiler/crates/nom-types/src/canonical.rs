//! Canonicalization + entry-id hashing for v2 `Entry` identity.
//!
//! Design:
//! - The canonical byte stream is a tag/length/payload encoding of the
//!   AST. Each variant gets a unique one-byte tag; each string/number
//!   is length-prefixed with a LEB128-style varint so no two distinct
//!   trees can share a byte encoding.
//! - Whitespace, comments and [`Span`] metadata are never written.
//!   nom-ast already doesn't preserve trivia, so we just skip spans.
//! - Map-like collections (struct fields, contract params) are kept in
//!   declaration order — reordering them would be a semantic change in
//!   Nom, so we DON'T sort. For match arms we also preserve order
//!   because match-arm order is semantically significant in Nom.
//! - The output is stable across runs (no HashMap iteration, no
//!   `Debug`/`Display` formatting, no float `to_string`).
//!
//! The hash is SHA-256 over:
//!     canonical_bytes(declaration) || 0xFF || canonical_contract(contract)
//!
//! so two entries that share an AST but have different contracts get
//! different ids.

use sha2::{Digest, Sha256};

use crate::Contract;
use nom_ast::*;

// ── Tag enumeration ─────────────────────────────────────────────────
// One byte per AST shape. Adding new variants appends — never
// reorder, never recycle; doing so would invalidate every id ever
// computed.
const TAG_DECL: u8 = 0x01;
const TAG_IDENT: u8 = 0x02;
const TAG_CONTRACT: u8 = 0x03;

const TAG_STMT_NEED: u8 = 0x10;
const TAG_STMT_REQUIRE: u8 = 0x11;
const TAG_STMT_EFFECTS: u8 = 0x12;
const TAG_STMT_FLOW: u8 = 0x13;
const TAG_STMT_DESCRIBE: u8 = 0x14;
const TAG_STMT_CONTRACT: u8 = 0x15;
const TAG_STMT_IMPLEMENT: u8 = 0x16;
const TAG_STMT_GIVEN: u8 = 0x17;
const TAG_STMT_WHEN: u8 = 0x18;
const TAG_STMT_THEN: u8 = 0x19;
const TAG_STMT_AND: u8 = 0x1A;
const TAG_STMT_GRAPH_NODE: u8 = 0x1B;
const TAG_STMT_GRAPH_EDGE: u8 = 0x1C;
const TAG_STMT_GRAPH_QUERY: u8 = 0x1D;
const TAG_STMT_GRAPH_CONSTRAINT: u8 = 0x1E;
const TAG_STMT_AGENT_CAP: u8 = 0x1F;
const TAG_STMT_AGENT_SUP: u8 = 0x20;
const TAG_STMT_AGENT_RECV: u8 = 0x21;
const TAG_STMT_AGENT_STATE: u8 = 0x22;
const TAG_STMT_AGENT_SCHED: u8 = 0x23;
const TAG_STMT_LET: u8 = 0x24;
const TAG_STMT_ASSIGN: u8 = 0x25;
const TAG_STMT_IF: u8 = 0x26;
const TAG_STMT_FOR: u8 = 0x27;
const TAG_STMT_WHILE: u8 = 0x28;
const TAG_STMT_MATCH: u8 = 0x29;
const TAG_STMT_RETURN: u8 = 0x2A;
const TAG_STMT_FN: u8 = 0x2B;
const TAG_STMT_STRUCT: u8 = 0x2C;
const TAG_STMT_ENUM: u8 = 0x2D;
const TAG_STMT_EXPR: u8 = 0x2E;
const TAG_STMT_TRAIT: u8 = 0x2F;
const TAG_STMT_IMPL: u8 = 0x30;
const TAG_STMT_USE: u8 = 0x31;
const TAG_STMT_MOD: u8 = 0x32;

const TAG_EXPR_IDENT: u8 = 0x40;
const TAG_EXPR_LIT: u8 = 0x41;
const TAG_EXPR_FIELD: u8 = 0x42;
const TAG_EXPR_BIN: u8 = 0x43;
const TAG_EXPR_CALL: u8 = 0x44;
const TAG_EXPR_UNARY: u8 = 0x45;
const TAG_EXPR_INDEX: u8 = 0x46;
const TAG_EXPR_RANGE: u8 = 0x47;
const TAG_EXPR_METHOD: u8 = 0x48;
const TAG_EXPR_IF: u8 = 0x49;
const TAG_EXPR_MATCH: u8 = 0x4A;
const TAG_EXPR_BLOCK: u8 = 0x4B;
const TAG_EXPR_CLOSURE: u8 = 0x4C;
const TAG_EXPR_ARRAY: u8 = 0x4D;
const TAG_EXPR_TUPLE: u8 = 0x4E;
const TAG_EXPR_AWAIT: u8 = 0x4F;
const TAG_EXPR_CAST: u8 = 0x50;
const TAG_EXPR_TRY: u8 = 0x51;
const TAG_EXPR_STRUCT_INIT: u8 = 0x52;

const TAG_LIT_NUMBER: u8 = 0x60;
const TAG_LIT_INTEGER: u8 = 0x61;
const TAG_LIT_TEXT: u8 = 0x62;
const TAG_LIT_BOOL: u8 = 0x63;
const TAG_LIT_NONE: u8 = 0x64;

const TAG_TY_NAMED: u8 = 0x70;
const TAG_TY_GENERIC: u8 = 0x71;
const TAG_TY_FN: u8 = 0x72;
const TAG_TY_TUPLE: u8 = 0x73;
const TAG_TY_REF: u8 = 0x74;
const TAG_TY_UNIT: u8 = 0x75;

const TAG_PAT_WILD: u8 = 0x80;
const TAG_PAT_LIT: u8 = 0x81;
const TAG_PAT_BIND: u8 = 0x82;
const TAG_PAT_VARIANT: u8 = 0x83;

const TAG_SOME: u8 = 0xA0;
const TAG_NONE: u8 = 0xA1;

// ── Public API ──────────────────────────────────────────────────────

/// Return the canonical byte encoding of a single top-level
/// declaration. Whitespace- and span-invariant by construction.
pub fn canonical_bytes(decl: &Declaration) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    write_decl(&mut out, decl);
    out
}

/// Return `hex(sha256(canonical_bytes(decl) || 0xFF || canonical_contract(contract)))`.
pub fn entry_id(decl: &Declaration, contract: &Contract) -> String {
    let mut hasher = Sha256::new();
    let decl_bytes = canonical_bytes(decl);
    hasher.update(&decl_bytes);
    hasher.update([0xFF]);
    let mut cbuf = Vec::with_capacity(64);
    write_contract(&mut cbuf, contract);
    hasher.update(&cbuf);
    let digest = hasher.finalize();
    hex(&digest)
}

/// Hex-encode a byte slice (lowercase, no separators).
fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

// ── Varint / length-prefixed primitives ─────────────────────────────

fn write_u64(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn write_bool(out: &mut Vec<u8>, b: bool) {
    out.push(if b { 1 } else { 0 });
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_u64(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn write_opt_str(out: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(v) => {
            out.push(TAG_SOME);
            write_str(out, v);
        }
        None => out.push(TAG_NONE),
    }
}

fn write_list<T, F: Fn(&mut Vec<u8>, &T)>(out: &mut Vec<u8>, items: &[T], f: F) {
    write_u64(out, items.len() as u64);
    for it in items {
        f(out, it);
    }
}

fn write_ident(out: &mut Vec<u8>, id: &Identifier) {
    out.push(TAG_IDENT);
    write_str(out, &id.name);
}

// ── Contract ────────────────────────────────────────────────────────

fn write_contract(out: &mut Vec<u8>, c: &Contract) {
    out.push(TAG_CONTRACT);
    write_opt_str(out, c.input_type.as_deref());
    write_opt_str(out, c.output_type.as_deref());
    write_opt_str(out, c.pre.as_deref());
    write_opt_str(out, c.post.as_deref());
}

// ── Declaration ─────────────────────────────────────────────────────

fn write_decl(out: &mut Vec<u8>, d: &Declaration) {
    out.push(TAG_DECL);
    out.push(d.classifier as u8);
    write_ident(out, &d.name);
    write_list(out, &d.statements, write_stmt);
}

// ── Statements ──────────────────────────────────────────────────────

fn write_stmt(out: &mut Vec<u8>, s: &Statement) {
    match s {
        Statement::Need(n) => {
            out.push(TAG_STMT_NEED);
            write_nomref(out, &n.reference);
            match &n.constraint {
                Some(c) => {
                    out.push(TAG_SOME);
                    write_constraint(out, c);
                }
                None => out.push(TAG_NONE),
            }
        }
        Statement::Require(r) => {
            out.push(TAG_STMT_REQUIRE);
            write_constraint(out, &r.constraint);
        }
        Statement::Effects(e) => {
            out.push(TAG_STMT_EFFECTS);
            match e.modifier {
                Some(m) => {
                    out.push(TAG_SOME);
                    out.push(m as u8);
                }
                None => out.push(TAG_NONE),
            }
            write_list(out, &e.effects, write_ident);
        }
        Statement::Flow(f) => {
            out.push(TAG_STMT_FLOW);
            out.push(f.qualifier as u8);
            write_flow_chain(out, &f.chain);
        }
        Statement::Describe(d) => {
            out.push(TAG_STMT_DESCRIBE);
            write_str(out, &d.text);
        }
        Statement::Contract(c) => {
            out.push(TAG_STMT_CONTRACT);
            write_list(out, &c.inputs, write_typed_param);
            write_list(out, &c.outputs, write_typed_param);
            write_list(out, &c.effects, write_ident);
            write_list(out, &c.preconditions, write_expr);
            write_list(out, &c.postconditions, write_expr);
        }
        Statement::Implement(i) => {
            out.push(TAG_STMT_IMPLEMENT);
            write_str(out, &i.language);
            write_str(out, &i.code);
        }
        Statement::Given(g) => {
            out.push(TAG_STMT_GIVEN);
            write_ident(out, &g.subject);
            write_u64(out, g.config.len() as u64);
            for (k, v) in &g.config {
                write_ident(out, k);
                write_expr(out, v);
            }
        }
        Statement::When(w) => {
            out.push(TAG_STMT_WHEN);
            write_ident(out, &w.action);
            write_u64(out, w.config.len() as u64);
            for (k, v) in &w.config {
                write_ident(out, k);
                write_expr(out, v);
            }
        }
        Statement::Then(t) => {
            out.push(TAG_STMT_THEN);
            write_expr(out, &t.assertion);
        }
        Statement::And(a) => {
            out.push(TAG_STMT_AND);
            write_expr(out, &a.assertion);
        }
        Statement::GraphNode(n) => {
            out.push(TAG_STMT_GRAPH_NODE);
            write_ident(out, &n.name);
            write_list(out, &n.fields, write_typed_param);
        }
        Statement::GraphEdge(e) => {
            out.push(TAG_STMT_GRAPH_EDGE);
            write_ident(out, &e.name);
            write_ident(out, &e.from_type);
            write_ident(out, &e.to_type);
            write_list(out, &e.fields, write_typed_param);
        }
        Statement::GraphQuery(q) => {
            out.push(TAG_STMT_GRAPH_QUERY);
            write_ident(out, &q.name);
            write_list(out, &q.params, write_typed_param);
            write_graph_query_expr(out, &q.expr);
        }
        Statement::GraphConstraint(c) => {
            out.push(TAG_STMT_GRAPH_CONSTRAINT);
            write_ident(out, &c.name);
            write_expr(out, &c.expr);
        }
        Statement::AgentCapability(c) => {
            out.push(TAG_STMT_AGENT_CAP);
            write_list(out, &c.capabilities, write_ident);
        }
        Statement::AgentSupervise(s) => {
            out.push(TAG_STMT_AGENT_SUP);
            write_ident(out, &s.strategy);
            write_u64(out, s.params.len() as u64);
            for (k, v) in &s.params {
                write_ident(out, k);
                write_expr(out, v);
            }
        }
        Statement::AgentReceive(r) => {
            out.push(TAG_STMT_AGENT_RECV);
            write_flow_chain(out, &r.chain);
        }
        Statement::AgentState(s) => {
            out.push(TAG_STMT_AGENT_STATE);
            write_ident(out, &s.state);
        }
        Statement::AgentSchedule(s) => {
            out.push(TAG_STMT_AGENT_SCHED);
            write_str(out, &s.interval);
            write_flow_chain(out, &s.action);
        }
        Statement::Let(l) => {
            out.push(TAG_STMT_LET);
            write_ident(out, &l.name);
            write_bool(out, l.mutable);
            match &l.type_ann {
                Some(t) => {
                    out.push(TAG_SOME);
                    write_type(out, t);
                }
                None => out.push(TAG_NONE),
            }
            write_expr(out, &l.value);
        }
        Statement::Assign(a) => {
            out.push(TAG_STMT_ASSIGN);
            write_expr(out, &a.target);
            write_expr(out, &a.value);
        }
        Statement::If(i) => {
            out.push(TAG_STMT_IF);
            write_if_expr(out, i);
        }
        Statement::For(f) => {
            out.push(TAG_STMT_FOR);
            write_ident(out, &f.binding);
            write_expr(out, &f.iterable);
            write_block(out, &f.body);
        }
        Statement::While(w) => {
            out.push(TAG_STMT_WHILE);
            write_expr(out, &w.condition);
            write_block(out, &w.body);
        }
        Statement::Match(m) => {
            out.push(TAG_STMT_MATCH);
            write_match(out, m);
        }
        Statement::Return(e) => {
            out.push(TAG_STMT_RETURN);
            match e {
                Some(v) => {
                    out.push(TAG_SOME);
                    write_expr(out, v);
                }
                None => out.push(TAG_NONE),
            }
        }
        Statement::FnDef(f) => {
            out.push(TAG_STMT_FN);
            write_fn_def(out, f);
        }
        Statement::StructDef(s) => {
            out.push(TAG_STMT_STRUCT);
            write_ident(out, &s.name);
            write_bool(out, s.is_pub);
            write_u64(out, s.fields.len() as u64);
            for f in &s.fields {
                write_ident(out, &f.name);
                write_type(out, &f.type_ann);
                write_bool(out, f.is_pub);
            }
        }
        Statement::EnumDef(e) => {
            out.push(TAG_STMT_ENUM);
            write_ident(out, &e.name);
            write_bool(out, e.is_pub);
            write_u64(out, e.variants.len() as u64);
            for v in &e.variants {
                write_ident(out, &v.name);
                write_list(out, &v.fields, write_type);
            }
        }
        Statement::ExprStmt(e) => {
            out.push(TAG_STMT_EXPR);
            write_expr(out, e);
        }
        Statement::TraitDef(t) => {
            out.push(TAG_STMT_TRAIT);
            write_ident(out, &t.name);
            write_bool(out, t.is_pub);
            write_list(out, &t.methods, write_fn_def);
        }
        Statement::ImplBlock(i) => {
            out.push(TAG_STMT_IMPL);
            match &i.trait_name {
                Some(id) => {
                    out.push(TAG_SOME);
                    write_ident(out, id);
                }
                None => out.push(TAG_NONE),
            }
            write_ident(out, &i.target_type);
            write_list(out, &i.methods, write_fn_def);
        }
        Statement::Use(u) => {
            out.push(TAG_STMT_USE);
            write_list(out, &u.path, write_ident);
            match &u.imports {
                UseImport::Single(id) => {
                    out.push(0);
                    write_ident(out, id);
                }
                UseImport::Multiple(ids) => {
                    out.push(1);
                    write_list(out, ids, write_ident);
                }
                UseImport::Glob => out.push(2),
            }
        }
        Statement::Mod(m) => {
            out.push(TAG_STMT_MOD);
            write_ident(out, &m.name);
        }
    }
}

fn write_nomref(out: &mut Vec<u8>, r: &NomRef) {
    write_ident(out, &r.word);
    match &r.variant {
        Some(v) => {
            out.push(TAG_SOME);
            write_ident(out, v);
        }
        None => out.push(TAG_NONE),
    }
}

fn write_typed_param(out: &mut Vec<u8>, p: &TypedParam) {
    write_ident(out, &p.name);
    match &p.typ {
        Some(t) => {
            out.push(TAG_SOME);
            write_ident(out, t);
        }
        None => out.push(TAG_NONE),
    }
}

fn write_constraint(out: &mut Vec<u8>, c: &Constraint) {
    write_expr(out, &c.left);
    out.push(c.op as u8);
    write_expr(out, &c.right);
}

fn write_flow_chain(out: &mut Vec<u8>, c: &FlowChain) {
    write_list(out, &c.steps, write_flow_step);
}

fn write_flow_step(out: &mut Vec<u8>, s: &FlowStep) {
    match s {
        FlowStep::Ref(r) => {
            out.push(0);
            write_nomref(out, r);
        }
        FlowStep::Literal(l) => {
            out.push(1);
            write_lit(out, l);
        }
        FlowStep::Branch(b) => {
            out.push(2);
            write_u64(out, b.arms.len() as u64);
            for arm in &b.arms {
                out.push(arm.condition as u8);
                write_opt_str(out, arm.label.as_deref());
                write_flow_chain(out, &arm.chain);
            }
        }
        FlowStep::Call(c) => {
            out.push(3);
            write_ident(out, &c.callee);
            write_list(out, &c.args, write_expr);
        }
    }
}

fn write_graph_query_expr(out: &mut Vec<u8>, q: &GraphQueryExpr) {
    match q {
        GraphQueryExpr::Ref(r) => {
            out.push(0);
            write_nomref(out, r);
        }
        GraphQueryExpr::Traverse(t) => {
            out.push(1);
            write_graph_query_expr(out, &t.source);
            write_nomref(out, &t.edge);
            write_graph_query_expr(out, &t.target);
        }
        GraphQueryExpr::SetOp(s) => {
            out.push(2);
            out.push(s.op as u8);
            write_list(out, &s.operands, write_graph_query_expr);
        }
    }
}

// ── Types ───────────────────────────────────────────────────────────

fn write_type(out: &mut Vec<u8>, t: &TypeExpr) {
    match t {
        TypeExpr::Named(id) => {
            out.push(TAG_TY_NAMED);
            write_ident(out, id);
        }
        TypeExpr::Generic(id, args) => {
            out.push(TAG_TY_GENERIC);
            write_ident(out, id);
            write_list(out, args, write_type);
        }
        TypeExpr::Function { params, ret } => {
            out.push(TAG_TY_FN);
            write_list(out, params, write_type);
            write_type(out, ret);
        }
        TypeExpr::Tuple(items) => {
            out.push(TAG_TY_TUPLE);
            write_list(out, items, write_type);
        }
        TypeExpr::Ref { mutable, inner } => {
            out.push(TAG_TY_REF);
            write_bool(out, *mutable);
            write_type(out, inner);
        }
        TypeExpr::Unit => out.push(TAG_TY_UNIT),
    }
}

// ── Expressions ─────────────────────────────────────────────────────

fn write_expr(out: &mut Vec<u8>, e: &Expr) {
    match e {
        Expr::Ident(id) => {
            out.push(TAG_EXPR_IDENT);
            write_ident(out, id);
        }
        Expr::Literal(l) => {
            out.push(TAG_EXPR_LIT);
            write_lit(out, l);
        }
        Expr::FieldAccess(obj, field) => {
            out.push(TAG_EXPR_FIELD);
            write_expr(out, obj);
            write_ident(out, field);
        }
        Expr::BinaryOp(l, op, r) => {
            out.push(TAG_EXPR_BIN);
            out.push(*op as u8);
            write_expr(out, l);
            write_expr(out, r);
        }
        Expr::Call(c) => {
            out.push(TAG_EXPR_CALL);
            write_ident(out, &c.callee);
            write_list(out, &c.args, write_expr);
        }
        Expr::UnaryOp(op, inner) => {
            out.push(TAG_EXPR_UNARY);
            out.push(*op as u8);
            write_expr(out, inner);
        }
        Expr::Index(obj, idx) => {
            out.push(TAG_EXPR_INDEX);
            write_expr(out, obj);
            write_expr(out, idx);
        }
        Expr::Range(lo, hi) => {
            out.push(TAG_EXPR_RANGE);
            write_expr(out, lo);
            write_expr(out, hi);
        }
        Expr::MethodCall(recv, name, args) => {
            out.push(TAG_EXPR_METHOD);
            write_expr(out, recv);
            write_ident(out, name);
            write_list(out, args, write_expr);
        }
        Expr::IfExpr(i) => {
            out.push(TAG_EXPR_IF);
            write_if_expr(out, i);
        }
        Expr::MatchExpr(m) => {
            out.push(TAG_EXPR_MATCH);
            write_match(out, m);
        }
        Expr::Block(b) => {
            out.push(TAG_EXPR_BLOCK);
            write_block(out, b);
        }
        Expr::Closure(params, body) => {
            out.push(TAG_EXPR_CLOSURE);
            write_list(out, params, write_fn_param);
            write_expr(out, body);
        }
        Expr::Array(items) => {
            out.push(TAG_EXPR_ARRAY);
            write_list(out, items, write_expr);
        }
        Expr::TupleExpr(items) => {
            out.push(TAG_EXPR_TUPLE);
            write_list(out, items, write_expr);
        }
        Expr::Await(inner) => {
            out.push(TAG_EXPR_AWAIT);
            write_expr(out, inner);
        }
        Expr::Cast(e, t) => {
            out.push(TAG_EXPR_CAST);
            write_expr(out, e);
            write_type(out, t);
        }
        Expr::Try(e) => {
            out.push(TAG_EXPR_TRY);
            write_expr(out, e);
        }
        Expr::StructInit { name, fields } => {
            out.push(TAG_EXPR_STRUCT_INIT);
            write_ident(out, name);
            write_u64(out, fields.len() as u64);
            // Struct-init field order is semantically irrelevant (LLVM
            // backend reorders to declaration order), so sort for
            // canonicalization.
            let mut idx: Vec<usize> = (0..fields.len()).collect();
            idx.sort_by(|&a, &b| fields[a].0.name.cmp(&fields[b].0.name));
            for i in idx {
                write_ident(out, &fields[i].0);
                write_expr(out, &fields[i].1);
            }
        }
    }
}

fn write_lit(out: &mut Vec<u8>, l: &Literal) {
    match l {
        Literal::Number(n) => {
            out.push(TAG_LIT_NUMBER);
            // Canonicalise via IEEE-754 bits; this is deterministic.
            // Normalise NaN to a single bit pattern so two NaNs hash
            // the same — semantically they're "not a number", and
            // Rust considers all NaNs equal under PartialEq::ne only.
            let bits = if n.is_nan() {
                f64::NAN.to_bits()
            } else {
                n.to_bits()
            };
            for b in bits.to_le_bytes() {
                out.push(b);
            }
        }
        Literal::Integer(i) => {
            out.push(TAG_LIT_INTEGER);
            for b in i.to_le_bytes() {
                out.push(b);
            }
        }
        Literal::Text(t) => {
            out.push(TAG_LIT_TEXT);
            write_str(out, t);
        }
        Literal::Bool(b) => {
            out.push(TAG_LIT_BOOL);
            write_bool(out, *b);
        }
        Literal::None => out.push(TAG_LIT_NONE),
    }
}

fn write_if_expr(out: &mut Vec<u8>, i: &IfExpr) {
    write_expr(out, &i.condition);
    write_block(out, &i.then_body);
    write_u64(out, i.else_ifs.len() as u64);
    for (c, b) in &i.else_ifs {
        write_expr(out, c);
        write_block(out, b);
    }
    match &i.else_body {
        Some(b) => {
            out.push(TAG_SOME);
            write_block(out, b);
        }
        None => out.push(TAG_NONE),
    }
}

fn write_match(out: &mut Vec<u8>, m: &MatchExpr) {
    write_expr(out, &m.subject);
    // Match-arm ORDER IS SEMANTICALLY SIGNIFICANT — do not sort.
    write_u64(out, m.arms.len() as u64);
    for arm in &m.arms {
        write_pattern(out, &arm.pattern);
        write_block(out, &arm.body);
    }
}

fn write_pattern(out: &mut Vec<u8>, p: &Pattern) {
    match p {
        Pattern::Wildcard => out.push(TAG_PAT_WILD),
        Pattern::Literal(l) => {
            out.push(TAG_PAT_LIT);
            write_lit(out, l);
        }
        Pattern::Binding(id) => {
            out.push(TAG_PAT_BIND);
            write_ident(out, id);
        }
        Pattern::Variant(name, subs) => {
            out.push(TAG_PAT_VARIANT);
            write_ident(out, name);
            write_list(out, subs, write_pattern);
        }
    }
}

fn write_block(out: &mut Vec<u8>, b: &Block) {
    write_u64(out, b.stmts.len() as u64);
    for st in &b.stmts {
        match st {
            BlockStmt::Let(l) => {
                out.push(0);
                write_ident(out, &l.name);
                write_bool(out, l.mutable);
                match &l.type_ann {
                    Some(t) => {
                        out.push(TAG_SOME);
                        write_type(out, t);
                    }
                    None => out.push(TAG_NONE),
                }
                write_expr(out, &l.value);
            }
            BlockStmt::Assign(a) => {
                out.push(1);
                write_expr(out, &a.target);
                write_expr(out, &a.value);
            }
            BlockStmt::Expr(e) => {
                out.push(2);
                write_expr(out, e);
            }
            BlockStmt::If(i) => {
                out.push(3);
                write_if_expr(out, i);
            }
            BlockStmt::For(f) => {
                out.push(4);
                write_ident(out, &f.binding);
                write_expr(out, &f.iterable);
                write_block(out, &f.body);
            }
            BlockStmt::While(w) => {
                out.push(5);
                write_expr(out, &w.condition);
                write_block(out, &w.body);
            }
            BlockStmt::Match(m) => {
                out.push(6);
                write_match(out, m);
            }
            BlockStmt::Return(e) => {
                out.push(7);
                match e {
                    Some(v) => {
                        out.push(TAG_SOME);
                        write_expr(out, v);
                    }
                    None => out.push(TAG_NONE),
                }
            }
            BlockStmt::Break => out.push(8),
            BlockStmt::Continue => out.push(9),
        }
    }
}

fn write_fn_def(out: &mut Vec<u8>, f: &FnDef) {
    write_ident(out, &f.name);
    write_list(out, &f.params, write_fn_param);
    match &f.return_type {
        Some(t) => {
            out.push(TAG_SOME);
            write_type(out, t);
        }
        None => out.push(TAG_NONE),
    }
    write_block(out, &f.body);
    write_bool(out, f.is_async);
    write_bool(out, f.is_pub);
}

fn write_fn_param(out: &mut Vec<u8>, p: &FnParam) {
    write_ident(out, &p.name);
    write_type(out, &p.type_ann);
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Identifier {
        Identifier::new("x", Span::default())
    }

    fn span() -> Span {
        Span::default()
    }

    fn diff_span() -> Span {
        Span::new(100, 200, 42, 7)
    }

    fn sample_fn(lit: i64) -> Declaration {
        Declaration {
            classifier: Classifier::Nom,
            name: Identifier::new("f", span()),
            statements: vec![Statement::FnDef(FnDef {
                name: Identifier::new("f", span()),
                params: vec![FnParam {
                    name: Identifier::new("x", span()),
                    type_ann: TypeExpr::Named(Identifier::new("int", span())),
                }],
                return_type: Some(TypeExpr::Named(Identifier::new("int", span()))),
                body: Block {
                    stmts: vec![BlockStmt::Expr(Expr::BinaryOp(
                        Box::new(Expr::Ident(id())),
                        BinOp::Add,
                        Box::new(Expr::Literal(Literal::Integer(lit))),
                    ))],
                    span: span(),
                },
                is_async: false,
                is_pub: false,
                span: span(),
            })],
            span: span(),
        }
    }

    #[test]
    fn span_invariance() {
        let a = sample_fn(1);
        let mut b = sample_fn(1);
        // mangle spans
        b.span = diff_span();
        if let Statement::FnDef(f) = &mut b.statements[0] {
            f.span = diff_span();
            f.name.span = diff_span();
            f.body.span = diff_span();
        }
        assert_eq!(canonical_bytes(&a), canonical_bytes(&b));
        assert_eq!(
            entry_id(&a, &Contract::default()),
            entry_id(&b, &Contract::default())
        );
    }

    #[test]
    fn literal_change_flips_hash() {
        let a = sample_fn(1);
        let b = sample_fn(2);
        assert_ne!(
            entry_id(&a, &Contract::default()),
            entry_id(&b, &Contract::default())
        );
    }

    #[test]
    fn contract_participates_in_id() {
        let a = sample_fn(1);
        let base = entry_id(&a, &Contract::default());
        let with_pre = entry_id(
            &a,
            &Contract {
                pre: Some("x > 0".to_string()),
                ..Contract::default()
            },
        );
        assert_ne!(base, with_pre);
    }

    #[test]
    fn determinism() {
        let a = sample_fn(1);
        let c = Contract::default();
        assert_eq!(entry_id(&a, &c), entry_id(&a, &c));
    }

    #[test]
    fn hex_len_is_64() {
        let a = sample_fn(1);
        let id = entry_id(&a, &Contract::default());
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
