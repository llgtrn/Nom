//! nom fmt: Canonical formatter for .nom source files.
//!
//! One canonical style, zero configuration — inspired by Go's gofmt.
//! Parses .nom source into AST, then re-emits with canonical formatting rules:
//!
//!   1. One blank line between declarations
//!   2. Two-space indent for statements within declarations
//!   3. Classifier + name on the first line of each declaration
//!   4. No trailing whitespace
//!   5. Single newline at end of file
//!   6. Spaces around `->` in flow chains

use nom_ast::*;
use nom_parser::parse_source;

/// Format a Nom source string with canonical style.
/// Returns the formatted source, or an error message if parsing fails.
pub fn format_source(source: &str) -> Result<String, String> {
    let parsed = parse_source(source).map_err(|e| format!("{e}"))?;
    Ok(emit_source_file(&parsed))
}

fn emit_source_file(sf: &SourceFile) -> String {
    let mut out = String::new();
    for (i, decl) in sf.declarations.iter().enumerate() {
        if i > 0 {
            out.push('\n'); // blank line between declarations
        }
        emit_declaration(&mut out, decl);
    }
    // Ensure single trailing newline
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn emit_declaration(out: &mut String, decl: &Declaration) {
    out.push_str(decl.classifier.as_str());
    out.push(' ');
    out.push_str(&decl.name.name);
    out.push('\n');

    for stmt in &decl.statements {
        emit_statement(out, stmt, 1);
    }
}

fn emit_statement(out: &mut String, stmt: &Statement, indent: usize) {
    let pad = "  ".repeat(indent);
    match stmt {
        Statement::Describe(d) => {
            out.push_str(&format!("{pad}describe \"{}\"\n", escape_str(&d.text)));
        }
        Statement::Need(n) => {
            let variant = n
                .reference
                .variant
                .as_ref()
                .map(|v| format!("::{}", v.name))
                .unwrap_or_default();
            let constraint = n
                .constraint
                .as_ref()
                .map(|c| format!(" where {}", fmt_constraint(c)))
                .unwrap_or_default();
            out.push_str(&format!(
                "{pad}need {}{}{}\n",
                n.reference.word.name, variant, constraint
            ));
        }
        Statement::Require(r) => {
            out.push_str(&format!("{pad}require {}\n", fmt_constraint(&r.constraint)));
        }
        Statement::Effects(e) => {
            let modifier = match &e.modifier {
                Some(EffectModifier::Only) => "only ",
                Some(EffectModifier::Good) => "good ",
                Some(EffectModifier::Bad) => "bad ",
                None => "",
            };
            let effects: Vec<&str> = e.effects.iter().map(|id| id.name.as_str()).collect();
            out.push_str(&format!("{pad}effects {}[{}]\n", modifier, effects.join(" ")));
        }
        Statement::Flow(f) => {
            let qualifier = match f.qualifier {
                FlowQualifier::Once => "",
                FlowQualifier::Stream => "stream ",
                FlowQualifier::Scheduled => "scheduled ",
            };
            let chain = fmt_flow_chain(&f.chain);
            let onfail = match &f.on_fail {
                OnFailStrategy::Abort => String::new(),
                OnFailStrategy::RestartFrom(id) => format!(" onfail restart_from {}", id.name),
                OnFailStrategy::Retry(n) => format!(" onfail retry {n}"),
                OnFailStrategy::Skip => " onfail skip".to_string(),
                OnFailStrategy::Escalate => " onfail escalate".to_string(),
            };
            out.push_str(&format!("{pad}flow {qualifier}{chain}{onfail}\n"));
        }
        Statement::Contract(c) => {
            out.push_str(&format!("{pad}contract\n"));
            let inner = "  ".repeat(indent + 1);
            for input in &c.inputs {
                let typ = input.typ.as_ref().map(|t| format!("({})", t.name)).unwrap_or_default();
                out.push_str(&format!("{inner}in {}{}\n", input.name.name, typ));
            }
            for output in &c.outputs {
                let typ = output.typ.as_ref().map(|t| format!("({})", t.name)).unwrap_or_default();
                out.push_str(&format!("{inner}out {}{}\n", output.name.name, typ));
            }
            if !c.effects.is_empty() {
                let effects: Vec<&str> = c.effects.iter().map(|id| id.name.as_str()).collect();
                out.push_str(&format!("{inner}effects [{}]\n", effects.join(" ")));
            }
        }
        Statement::Implement(imp) => {
            out.push_str(&format!("{pad}implement {} {{\n", imp.language));
            let inner = "  ".repeat(indent + 1);
            for line in imp.code.lines() {
                if line.trim().is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&format!("{inner}{}\n", line.trim()));
                }
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        // Test statements
        Statement::Given(g) => {
            let config = fmt_kv_config(&g.config);
            out.push_str(&format!("{pad}given {}{config}\n", g.subject.name));
        }
        Statement::When(w) => {
            let config = fmt_kv_config(&w.config);
            out.push_str(&format!("{pad}when {}{config}\n", w.action.name));
        }
        Statement::Then(t) => {
            out.push_str(&format!("{pad}then {}\n", fmt_expr(&t.assertion)));
        }
        Statement::And(a) => {
            out.push_str(&format!("{pad}and {}\n", fmt_expr(&a.assertion)));
        }
        // Graph statements
        Statement::GraphNode(n) => {
            let fields = fmt_typed_params(&n.fields);
            out.push_str(&format!("{pad}node {}({})\n", n.name.name, fields));
        }
        Statement::GraphEdge(e) => {
            let fields = fmt_typed_params(&e.fields);
            let extra = if fields.is_empty() {
                String::new()
            } else {
                format!(", {fields}")
            };
            out.push_str(&format!(
                "{pad}edge {}(from {}, to {}{})\n",
                e.name.name, e.from_type.name, e.to_type.name, extra
            ));
        }
        Statement::GraphQuery(q) => {
            let params = fmt_typed_params(&q.params);
            let expr = fmt_graph_query_expr(&q.expr);
            out.push_str(&format!(
                "{pad}query {}({}) = {}\n",
                q.name.name, params, expr
            ));
        }
        Statement::GraphConstraint(c) => {
            out.push_str(&format!(
                "{pad}constraint {} = {}\n",
                c.name.name,
                fmt_expr(&c.expr)
            ));
        }
        // Agent statements
        Statement::AgentCapability(c) => {
            let caps: Vec<&str> = c.capabilities.iter().map(|id| id.name.as_str()).collect();
            out.push_str(&format!("{pad}capability [{}]\n", caps.join(" ")));
        }
        Statement::AgentSupervise(s) => {
            let params: Vec<String> = s
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", k.name, fmt_expr(v)))
                .collect();
            let params_str = if params.is_empty() {
                String::new()
            } else {
                format!(" {}", params.join(" "))
            };
            out.push_str(&format!(
                "{pad}supervise {}{params_str}\n",
                s.strategy.name
            ));
        }
        Statement::AgentReceive(r) => {
            out.push_str(&format!("{pad}receive {}\n", fmt_flow_chain(&r.chain)));
        }
        Statement::AgentState(s) => {
            out.push_str(&format!("{pad}state {}\n", s.state.name));
        }
        Statement::AgentSchedule(s) => {
            out.push_str(&format!(
                "{pad}schedule every {} {}\n",
                s.interval,
                fmt_flow_chain(&s.action)
            ));
        }
        // Imperative statements — emit Nom syntax (not Rust)
        Statement::Let(l) => {
            let mutability = if l.mutable { "let mut" } else { "let" };
            let type_ann = l
                .type_ann
                .as_ref()
                .map(|t| format!(": {}", fmt_type_expr(t)))
                .unwrap_or_default();
            out.push_str(&format!(
                "{pad}{mutability} {}{type_ann} = {}\n",
                l.name.name,
                fmt_expr(&l.value)
            ));
        }
        Statement::Assign(a) => {
            out.push_str(&format!(
                "{pad}{} = {}\n",
                fmt_expr(&a.target),
                fmt_expr(&a.value)
            ));
        }
        Statement::If(ifexpr) => {
            emit_if(out, ifexpr, indent);
        }
        Statement::For(f) => {
            out.push_str(&format!(
                "{pad}for {} in {} {{\n",
                f.binding.name,
                fmt_expr(&f.iterable)
            ));
            emit_block(out, &f.body, indent + 1);
            out.push_str(&format!("{pad}}}\n"));
        }
        Statement::While(w) => {
            out.push_str(&format!("{pad}while {} {{\n", fmt_expr(&w.condition)));
            emit_block(out, &w.body, indent + 1);
            out.push_str(&format!("{pad}}}\n"));
        }
        Statement::Match(m) => {
            emit_match(out, m, indent);
        }
        Statement::Return(expr) => {
            if let Some(e) = expr {
                out.push_str(&format!("{pad}return {}\n", fmt_expr(e)));
            } else {
                out.push_str(&format!("{pad}return\n"));
            }
        }
        Statement::FnDef(f) => {
            emit_fn_def(out, f, indent);
        }
        Statement::StructDef(s) => {
            emit_struct_def(out, s, indent);
        }
        Statement::EnumDef(e) => {
            emit_enum_def(out, e, indent);
        }
        Statement::ExprStmt(e) => {
            out.push_str(&format!("{pad}{}\n", fmt_expr(e)));
        }
        Statement::Use(u) => {
            let path_str = u.path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join("::");
            let import_str = match &u.imports {
                nom_ast::UseImport::Single(name) => name.name.clone(),
                nom_ast::UseImport::Multiple(names) => {
                    let items: Vec<&str> = names.iter().map(|n| n.name.as_str()).collect();
                    format!("{{{}}}", items.join(", "))
                }
                nom_ast::UseImport::Glob => "*".to_string(),
            };
            if path_str.is_empty() {
                out.push_str(&format!("{pad}use {import_str}\n"));
            } else {
                out.push_str(&format!("{pad}use {path_str}::{import_str}\n"));
            }
        }
        Statement::Mod(m) => {
            out.push_str(&format!("{pad}mod {}\n", m.name.name));
        }
        Statement::TraitDef(t) => {
            let vis = if t.is_pub { "pub " } else { "" };
            out.push_str(&format!("{pad}{vis}trait {} {{\n", t.name.name));
            for method in &t.methods {
                emit_fn_def(out, method, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Statement::ImplBlock(i) => {
            if let Some(ref trait_name) = i.trait_name {
                out.push_str(&format!("{pad}impl {} for {} {{\n", trait_name.name, i.target_type.name));
            } else {
                out.push_str(&format!("{pad}impl {} {{\n", i.target_type.name));
            }
            for method in &i.methods {
                emit_fn_def(out, method, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
    }
}

// ── Imperative emitters ────────────────────────────────────────────────────────

fn emit_fn_def(out: &mut String, f: &FnDef, indent: usize) {
    let pad = "  ".repeat(indent);
    let vis = if f.is_pub { "pub " } else { "" };
    let async_kw = if f.is_async { "async " } else { "" };
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.name, fmt_type_expr(&p.type_ann)))
        .collect();
    let ret = f
        .return_type
        .as_ref()
        .map(|t| format!(" -> {}", fmt_type_expr(t)))
        .unwrap_or_default();
    out.push_str(&format!(
        "{pad}{vis}{async_kw}fn {}({}){ret} {{\n",
        f.name.name,
        params.join(", ")
    ));
    emit_block(out, &f.body, indent + 1);
    out.push_str(&format!("{pad}}}\n"));
}

fn emit_struct_def(out: &mut String, s: &StructDef, indent: usize) {
    let pad = "  ".repeat(indent);
    let vis = if s.is_pub { "pub " } else { "" };
    out.push_str(&format!("{pad}{vis}struct {} {{\n", s.name.name));
    let inner = "  ".repeat(indent + 1);
    for (i, field) in s.fields.iter().enumerate() {
        let fvis = if field.is_pub { "pub " } else { "" };
        let comma = if i + 1 < s.fields.len() { "," } else { "" };
        out.push_str(&format!(
            "{inner}{fvis}{}: {}{comma}\n",
            field.name.name,
            fmt_type_expr(&field.type_ann)
        ));
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn emit_enum_def(out: &mut String, e: &EnumDef, indent: usize) {
    let pad = "  ".repeat(indent);
    let vis = if e.is_pub { "pub " } else { "" };
    out.push_str(&format!("{pad}{vis}enum {} {{\n", e.name.name));
    let inner = "  ".repeat(indent + 1);
    for (i, variant) in e.variants.iter().enumerate() {
        let comma = if i + 1 < e.variants.len() { "," } else { "" };
        if variant.fields.is_empty() {
            out.push_str(&format!("{inner}{}{comma}\n", variant.name.name));
        } else {
            let fields: Vec<String> = variant.fields.iter().map(|t| fmt_type_expr(t)).collect();
            out.push_str(&format!(
                "{inner}{}({}){comma}\n",
                variant.name.name,
                fields.join(", ")
            ));
        }
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn emit_if(out: &mut String, ifexpr: &IfExpr, indent: usize) {
    let pad = "  ".repeat(indent);
    out.push_str(&format!("{pad}if {} {{\n", fmt_expr(&ifexpr.condition)));
    emit_block(out, &ifexpr.then_body, indent + 1);
    for (cond, body) in &ifexpr.else_ifs {
        out.push_str(&format!("{pad}}} else if {} {{\n", fmt_expr(cond)));
        emit_block(out, body, indent + 1);
    }
    if let Some(body) = &ifexpr.else_body {
        out.push_str(&format!("{pad}}} else {{\n"));
        emit_block(out, body, indent + 1);
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn emit_match(out: &mut String, m: &MatchExpr, indent: usize) {
    let pad = "  ".repeat(indent);
    out.push_str(&format!("{pad}match {} {{\n", fmt_expr(&m.subject)));
    let inner = "  ".repeat(indent + 1);
    for arm in &m.arms {
        out.push_str(&format!("{inner}{} => {{\n", fmt_pattern(&arm.pattern)));
        emit_block(out, &arm.body, indent + 2);
        out.push_str(&format!("{inner}}}\n"));
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn emit_block(out: &mut String, block: &Block, indent: usize) {
    let pad = "  ".repeat(indent);
    for stmt in &block.stmts {
        match stmt {
            BlockStmt::Let(l) => {
                let mutability = if l.mutable { "let mut" } else { "let" };
                let type_ann = l
                    .type_ann
                    .as_ref()
                    .map(|t| format!(": {}", fmt_type_expr(t)))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "{pad}{mutability} {}{type_ann} = {}\n",
                    l.name.name,
                    fmt_expr(&l.value)
                ));
            }
            BlockStmt::Assign(a) => {
                out.push_str(&format!(
                    "{pad}{} = {}\n",
                    fmt_expr(&a.target),
                    fmt_expr(&a.value)
                ));
            }
            BlockStmt::Expr(e) => {
                out.push_str(&format!("{pad}{}\n", fmt_expr(e)));
            }
            BlockStmt::If(ifexpr) => {
                emit_if(out, ifexpr, indent);
            }
            BlockStmt::For(f) => {
                out.push_str(&format!(
                    "{pad}for {} in {} {{\n",
                    f.binding.name,
                    fmt_expr(&f.iterable)
                ));
                emit_block(out, &f.body, indent + 1);
                out.push_str(&format!("{pad}}}\n"));
            }
            BlockStmt::While(w) => {
                out.push_str(&format!("{pad}while {} {{\n", fmt_expr(&w.condition)));
                emit_block(out, &w.body, indent + 1);
                out.push_str(&format!("{pad}}}\n"));
            }
            BlockStmt::Match(m) => {
                emit_match(out, m, indent);
            }
            BlockStmt::Return(expr) => {
                if let Some(e) = expr {
                    out.push_str(&format!("{pad}return {}\n", fmt_expr(e)));
                } else {
                    out.push_str(&format!("{pad}return\n"));
                }
            }
            BlockStmt::Break => {
                out.push_str(&format!("{pad}break\n"));
            }
            BlockStmt::Continue => {
                out.push_str(&format!("{pad}continue\n"));
            }
        }
    }
}

// ── Formatting helpers ─────────────────────────────────────────────────────────

fn fmt_expr(e: &Expr) -> String {
    match e {
        Expr::Ident(id) => id.name.clone(),
        Expr::Literal(lit) => fmt_literal(lit),
        Expr::FieldAccess(base, field) => format!("{}.{}", fmt_expr(base), field.name),
        Expr::BinaryOp(left, op, right) => {
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::Gt => ">",
                BinOp::Lt => "<",
                BinOp::Gte => ">=",
                BinOp::Lte => "<=",
                BinOp::Eq => "==",
                BinOp::Neq => "!=",
                BinOp::BitAnd => "&",
                BinOp::BitOr => "|",
            };
            format!("{} {op_str} {}", fmt_expr(left), fmt_expr(right))
        }
        Expr::Call(call) => {
            let args: Vec<String> = call.args.iter().map(fmt_expr).collect();
            format!("{}({})", call.callee.name, args.join(", "))
        }
        Expr::UnaryOp(op, inner) => {
            let op_str = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
                UnaryOp::Ref => "&",
                UnaryOp::RefMut => "&mut ",
            };
            format!("{op_str}{}", fmt_expr(inner))
        }
        Expr::Index(base, idx) => format!("{}[{}]", fmt_expr(base), fmt_expr(idx)),
        Expr::MethodCall(obj, method, args) => {
            let args_str: Vec<String> = args.iter().map(fmt_expr).collect();
            format!("{}.{}({})", fmt_expr(obj), method.name, args_str.join(", "))
        }
        Expr::Array(items) => {
            let inner: Vec<String> = items.iter().map(fmt_expr).collect();
            format!("[{}]", inner.join(", "))
        }
        Expr::TupleExpr(items) => {
            let inner: Vec<String> = items.iter().map(fmt_expr).collect();
            format!("({})", inner.join(", "))
        }
        Expr::Await(inner) => format!("{}.await", fmt_expr(inner)),
        Expr::Cast(inner, ty) => format!("{} as {}", fmt_expr(inner), fmt_type_expr(ty)),
        Expr::Try(inner) => format!("{}?", fmt_expr(inner)),
        Expr::IfExpr(_) => "<if-expr>".to_string(),
        Expr::MatchExpr(_) => "<match-expr>".to_string(),
        Expr::Block(_) => "<block>".to_string(),
        Expr::Closure(params, body) => {
            let params_str: Vec<String> = params
                .iter()
                .map(|p| format!("{}: {}", p.name.name, fmt_type_expr(&p.type_ann)))
                .collect();
            format!("|{}| {}", params_str.join(", "), fmt_expr(body))
        }
    }
}

fn fmt_literal(lit: &Literal) -> String {
    match lit {
        Literal::Number(f) => {
            let s = f.to_string();
            if s.contains('.') {
                s
            } else {
                format!("{s}.0")
            }
        }
        Literal::Integer(n) => n.to_string(),
        Literal::Text(s) => format!("\"{}\"", escape_str(s)),
        Literal::Bool(b) => b.to_string(),
        Literal::None => "none".to_string(),
    }
}

fn fmt_constraint(c: &Constraint) -> String {
    let op_str = match c.op {
        CompareOp::Gt => ">",
        CompareOp::Lt => "<",
        CompareOp::Gte => ">=",
        CompareOp::Lte => "<=",
        CompareOp::Eq => "=",
        CompareOp::Neq => "!=",
    };
    format!("{} {} {}", fmt_expr(&c.left), op_str, fmt_expr(&c.right))
}

fn fmt_flow_chain(chain: &FlowChain) -> String {
    let steps: Vec<String> = chain.steps.iter().map(fmt_flow_step).collect();
    steps.join(" -> ")
}

fn fmt_flow_step(step: &FlowStep) -> String {
    match step {
        FlowStep::Ref(r) => {
            let variant = r
                .variant
                .as_ref()
                .map(|v| format!("::{}", v.name))
                .unwrap_or_default();
            format!("{}{}", r.word.name, variant)
        }
        FlowStep::Literal(lit) => fmt_literal(lit),
        FlowStep::Branch(block) => {
            let arms: Vec<String> = block
                .arms
                .iter()
                .map(|arm| {
                    let cond = match arm.condition {
                        BranchCondition::IfTrue => "iftrue",
                        BranchCondition::IfFalse => "iffalse",
                        BranchCondition::Named => {
                            arm.label.as_deref().unwrap_or("_")
                        }
                    };
                    format!("{} -> {}", cond, fmt_flow_chain(&arm.chain))
                })
                .collect();
            format!("{{ {} }}", arms.join(", "))
        }
        FlowStep::Call(call) => {
            let args: Vec<String> = call.args.iter().map(fmt_expr).collect();
            format!("{}({})", call.callee.name, args.join(", "))
        }
    }
}

fn fmt_type_expr(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(id) => id.name.clone(),
        TypeExpr::Generic(name, args) => {
            let args_str: Vec<String> = args.iter().map(fmt_type_expr).collect();
            format!("{}[{}]", name.name, args_str.join(", "))
        }
        TypeExpr::Function { params, ret } => {
            let params_str: Vec<String> = params.iter().map(fmt_type_expr).collect();
            format!("fn({}) -> {}", params_str.join(", "), fmt_type_expr(ret))
        }
        TypeExpr::Tuple(items) => {
            let inner: Vec<String> = items.iter().map(fmt_type_expr).collect();
            format!("({})", inner.join(", "))
        }
        TypeExpr::Ref { mutable, inner } => {
            if *mutable {
                format!("&mut {}", fmt_type_expr(inner))
            } else {
                format!("&{}", fmt_type_expr(inner))
            }
        }
        TypeExpr::Unit => "()".to_string(),
    }
}

fn fmt_pattern(p: &Pattern) -> String {
    match p {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Literal(lit) => fmt_literal(lit),
        Pattern::Binding(id) => id.name.clone(),
        Pattern::Variant(name, fields) => {
            if fields.is_empty() {
                name.name.clone()
            } else {
                let fields_str: Vec<String> = fields.iter().map(fmt_pattern).collect();
                format!("{}({})", name.name, fields_str.join(", "))
            }
        }
    }
}

fn fmt_typed_params(params: &[TypedParam]) -> String {
    let parts: Vec<String> = params
        .iter()
        .map(|p| {
            if let Some(t) = &p.typ {
                format!("{} {}", p.name.name, t.name)
            } else {
                p.name.name.clone()
            }
        })
        .collect();
    parts.join(", ")
}

fn fmt_kv_config(config: &[(Identifier, Expr)]) -> String {
    if config.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = config
        .iter()
        .map(|(k, v)| format!(" {}={}", k.name, fmt_expr(v)))
        .collect();
    parts.join("")
}

fn fmt_graph_query_expr(expr: &GraphQueryExpr) -> String {
    match expr {
        GraphQueryExpr::Ref(r) => {
            let variant = r
                .variant
                .as_ref()
                .map(|v| format!("::{}", v.name))
                .unwrap_or_default();
            format!("{}{}", r.word.name, variant)
        }
        GraphQueryExpr::Traverse(t) => {
            format!(
                "{} -> {} -> {}",
                fmt_graph_query_expr(&t.source),
                fmt_graph_query_expr_ref_name(&t.edge),
                fmt_graph_query_expr(&t.target)
            )
        }
        GraphQueryExpr::SetOp(s) => {
            let op_str = match s.op {
                GraphSetOp::Union => "union",
                GraphSetOp::Intersection => "intersect",
                GraphSetOp::Difference => "diff",
            };
            let operands: Vec<String> = s.operands.iter().map(fmt_graph_query_expr).collect();
            format!("{}({})", op_str, operands.join(", "))
        }
    }
}

fn fmt_graph_query_expr_ref_name(r: &NomRef) -> String {
    let variant = r
        .variant
        .as_ref()
        .map(|v| format!("::{}", v.name))
        .unwrap_or_default();
    format!("{}{}", r.word.name, variant)
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_basic_declaration() {
        let source = "system   auth\n need hash::argon2\n  need store::redis\n flow request->hash->store->response\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("system auth\n"));
        assert!(formatted.contains("  need hash::argon2\n"));
        assert!(formatted.contains("  need store::redis\n"));
        assert!(formatted.contains("  flow request -> hash -> store -> response\n"));
    }

    #[test]
    fn formats_describe_and_require() {
        let source = "system auth\n  describe \"some text\"\n  require latency < 50\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("  describe \"some text\"\n"));
        assert!(formatted.contains("  require latency < 50\n"));
    }

    #[test]
    fn formats_effects() {
        let source = "system auth\n  effects only [network database]\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("  effects only [network database]\n"));
    }

    #[test]
    fn blank_line_between_declarations() {
        let source = "system a\n  need hash\n\nsystem b\n  need store\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("system a\n  need hash\n\nsystem b\n"));
    }

    #[test]
    fn ends_with_single_newline() {
        let source = "system auth\n  need hash\n\n\n\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.ends_with('\n'));
        assert!(!formatted.ends_with("\n\n"));
    }

    #[test]
    fn idempotent_on_canonical_input() {
        let source = "system auth\n  need hash::argon2 where security > 0.9\n  flow request -> hash -> response\n  require latency < 50\n  effects only [network]\n";
        let first = format_source(source).unwrap();
        let second = format_source(&first).unwrap();
        assert_eq!(first, second, "formatter should be idempotent");
    }

    #[test]
    fn formats_test_declaration() {
        let source = "test auth_works\n  given auth\n  then security > 0.5\n";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("test auth_works\n"));
        assert!(formatted.contains("  given auth\n"));
        assert!(formatted.contains("  then security > 0.5\n"));
    }
}
