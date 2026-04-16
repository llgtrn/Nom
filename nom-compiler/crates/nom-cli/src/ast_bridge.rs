//! Bridges the `nom-concept` AST (PipelineOutput) to the legacy `nom-ast::SourceFile`.
//! This is a temporary validation bypass layer designed to fulfill GAP-4 (nom-parser deletion)
//! while backend modules (nom-llvm, nom-resolver) migrate fully to `.nom` and `.nomtu` formats.

use nom_ast::{
    BinOp, Block, BlockStmt, CallExpr, Classifier, Declaration, DescribeStmt, EffectModifier,
    EffectsStmt, Expr, FnDef, FnParam, ForStmt, Identifier, IfExpr, Literal, SourceFile, Span,
    Statement, TypeExpr, WhileStmt,
};
use nom_concept::NomtuItem;
use nom_concept::stages::PipelineOutput;

pub fn bridge_to_ast(pipeline_out: &PipelineOutput, source_path: Option<String>) -> SourceFile {
    let mut declarations = Vec::new();

    match pipeline_out {
        PipelineOutput::Nom(nom_file) => {
            // Concepts do not have a direct imperative mapping in nom-ast.
            // We map them as `Classifier::Nom` with empty bodies, preserving their name.
            for concept in &nom_file.concepts {
                declarations.push(Declaration {
                    classifier: Classifier::Nom,
                    name: Identifier {
                        name: concept.name.clone(),
                        span: Span::default(),
                    },
                    statements: vec![],
                    span: Span::default(),
                });
            }
        }
        PipelineOutput::Nomtu(nomtu_file) => {
            // Entities and Compositions.
            for item in &nomtu_file.items {
                match item {
                    NomtuItem::Entity(ent) => {
                        let classifier = Classifier::from_str(&ent.kind).unwrap_or(Classifier::Nom);
                        declarations.push(Declaration {
                            classifier,
                            name: Identifier {
                                name: ent.word.clone(),
                                span: Span::default(),
                            },
                            statements: entity_statements(ent),
                            span: Span::default(),
                        });
                    }
                    NomtuItem::Composition(comp) => {
                        declarations.push(Declaration {
                            classifier: Classifier::System,
                            name: Identifier {
                                name: comp.word.clone(),
                                span: Span::default(),
                            },
                            statements: vec![],
                            span: Span::default(),
                        });
                    }
                }
            }
        }
    }

    SourceFile {
        path: source_path,
        locale: None,
        declarations,
    }
}

fn entity_statements(ent: &nom_concept::EntityDecl) -> Vec<Statement> {
    let mut statements = Vec::new();
    if !ent.signature.trim().is_empty() {
        statements.push(Statement::Describe(DescribeStmt {
            text: ent.signature.clone(),
            span: Span::default(),
        }));
    }

    // Map contracts (requires/ensures) → Statement::Describe as prose annotations.
    // RequireStmt expects a parsed Constraint expression; since contract predicates
    // are free-form prose strings we use Describe as the canonical fallback.
    for clause in &ent.contracts {
        use nom_concept::ContractClause;
        let text = match clause {
            ContractClause::Requires(pred) => format!("requires: {}", pred),
            ContractClause::Ensures(pred) => format!("ensures: {}", pred),
        };
        statements.push(Statement::Describe(DescribeStmt {
            text,
            span: Span::default(),
        }));
    }

    // Map effects (benefit/hazard) → Statement::Effects with Good/Bad modifier.
    for effect_clause in &ent.effects {
        use nom_concept::EffectValence;
        let modifier = match effect_clause.valence {
            EffectValence::Benefit => Some(EffectModifier::Good),
            EffectValence::Hazard => Some(EffectModifier::Bad),
        };
        let effects: Vec<Identifier> = effect_clause
            .effects
            .iter()
            .map(|name| ident(&sanitize_identifier(name)))
            .collect();
        if !effects.is_empty() {
            statements.push(Statement::Effects(EffectsStmt {
                modifier,
                effects,
                span: Span::default(),
            }));
        }
    }

    if ent.kind == "function" {
        if let Some(fn_def) = fn_def_from_signature(&ent.word, &ent.signature) {
            statements.push(Statement::FnDef(fn_def));
        }
    }

    statements
}

fn fn_def_from_signature(word: &str, signature: &str) -> Option<FnDef> {
    let lower = signature.to_ascii_lowercase();
    let returns_idx = lower.find("returns")?;
    let params_src = signature[..returns_idx].trim();
    let return_src = signature[returns_idx + "returns".len()..].trim();
    let (return_type_src, explicit_return_src) = split_return_spec(return_src);
    let params = parse_params(params_src);
    let return_type = parse_type(return_type_src).unwrap_or_else(|| named_type("unit"));
    let body_stmts = if let Some(body_src) = explicit_return_src {
        parse_multi_stmt_body(body_src, &return_type)
    } else {
        let return_expr = default_expr_for_type(&return_type);
        vec![BlockStmt::Return(return_expr)]
    };

    Some(FnDef {
        name: ident(word),
        params,
        return_type: Some(return_type),
        body: Block {
            stmts: body_stmts,
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    })
}

/// Parse a multi-statement body from explicit return source.
/// Splits on `; then`, `, then`, or `;` separators. Each clause is
/// parsed as a return expression or a statement. The last clause is
/// always emitted as `BlockStmt::Return`.
fn parse_multi_stmt_body(body_src: &str, return_type: &TypeExpr) -> Vec<BlockStmt> {
    let clauses = split_body_clauses(body_src);
    if clauses.len() <= 1 {
        // Try loop statement first before falling back to return expr
        if let Some(loop_stmt) = parse_loop_stmt(body_src) {
            return vec![loop_stmt];
        }
        let expr = parse_return_expr(body_src).or_else(|| default_expr_for_type(return_type));
        return vec![BlockStmt::Return(expr)];
    }

    let mut stmts = Vec::new();
    for (i, clause) in clauses.iter().enumerate() {
        let trimmed = clause.trim();
        if trimmed.is_empty() {
            continue;
        }
        let is_last = i == clauses.len() - 1;
        if let Some(loop_stmt) = parse_loop_stmt(trimmed) {
            stmts.push(loop_stmt);
        } else if is_last {
            let expr = parse_return_expr(trimmed).or_else(|| default_expr_for_type(return_type));
            stmts.push(BlockStmt::Return(expr));
        } else if let Some(expr) = parse_return_expr(trimmed) {
            stmts.push(BlockStmt::Expr(expr));
        }
    }
    stmts
}

/// Recognise prose loop patterns and return a `BlockStmt::For` or `BlockStmt::While`.
///
/// Supported patterns:
///   "for each <var> in <collection> do <body>"  → ForStmt
///   "for <var> in <collection> do <body>"       → ForStmt
///   "while <condition> do <body>"               → WhileStmt
///   "repeat <body> until <condition>"           → WhileStmt (negated condition as guard)
fn parse_loop_stmt(expr: &str) -> Option<BlockStmt> {
    let trimmed = expr.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();

    // "for each <var> in <collection> do <body>"
    // "for <var> in <collection> do <body>"
    if lower.starts_with("for ") {
        let after_for = &trimmed["for ".len()..];
        let after_for_lower = after_for.to_ascii_lowercase();
        // Strip optional "each "
        let (binding_start, _) = if after_for_lower.starts_with("each ") {
            (&after_for["each ".len()..], true)
        } else {
            (after_for, false)
        };
        let binding_lower = binding_start.to_ascii_lowercase();
        // Find " in " to split binding from collection+body
        let in_pos = binding_lower.find(" in ")?;
        let binding_name = sanitize_identifier(&binding_start[..in_pos]);
        let after_in = &binding_start[in_pos + " in ".len()..];
        let after_in_lower = after_in.to_ascii_lowercase();
        // Find " do " to split collection from body
        let do_pos = after_in_lower.find(" do ")?;
        let collection_src = after_in[..do_pos].trim();
        let body_src = after_in[do_pos + " do ".len()..].trim();
        let iterable = parse_atom_expr(collection_src)?;
        let body_expr = parse_return_expr(body_src)?;
        return Some(BlockStmt::For(ForStmt {
            binding: ident(&binding_name),
            iterable,
            body: Block {
                stmts: vec![BlockStmt::Expr(body_expr)],
                span: Span::default(),
            },
            span: Span::default(),
        }));
    }

    // "while <condition> do <body>"
    if lower.starts_with("while ") {
        let after_while = &trimmed["while ".len()..];
        let after_while_lower = after_while.to_ascii_lowercase();
        let do_pos = after_while_lower.find(" do ")?;
        let cond_src = after_while[..do_pos].trim();
        let body_src = after_while[do_pos + " do ".len()..].trim();
        let condition = parse_condition_expr(cond_src)?;
        let body_expr = parse_return_expr(body_src)?;
        return Some(BlockStmt::While(WhileStmt {
            condition,
            body: Block {
                stmts: vec![BlockStmt::Expr(body_expr)],
                span: Span::default(),
            },
            span: Span::default(),
        }));
    }

    // "repeat <body> until <condition>"
    if lower.starts_with("repeat ") {
        let after_repeat = &trimmed["repeat ".len()..];
        let after_repeat_lower = after_repeat.to_ascii_lowercase();
        let until_pos = after_repeat_lower.find(" until ")?;
        let body_src = after_repeat[..until_pos].trim();
        let cond_src = after_repeat[until_pos + " until ".len()..].trim();
        let condition = parse_condition_expr(cond_src)?;
        let body_expr = parse_return_expr(body_src)?;
        // Model as while(condition) with body — semantically "repeat body until cond"
        // maps to a do-while; we approximate as while since BlockStmt has no do-while.
        return Some(BlockStmt::While(WhileStmt {
            condition,
            body: Block {
                stmts: vec![BlockStmt::Expr(body_expr)],
                span: Span::default(),
            },
            span: Span::default(),
        }));
    }

    None
}

/// Split a body source string into clauses.
/// Recognises "; then ", ", then ", and ";" as clause separators.
fn split_body_clauses(body_src: &str) -> Vec<&str> {
    for sep in ["; then ", ", then "] {
        if body_src.contains(sep) {
            return body_src.split(sep).collect();
        }
    }
    if body_src.contains(';') {
        return body_src.split(';').collect();
    }
    vec![body_src]
}

fn split_return_spec(return_src: &str) -> (&str, Option<&str>) {
    for marker in [" by returning ", " returning ", " with value "] {
        if let Some((ty, expr)) = return_src.split_once(marker) {
            return (ty.trim(), Some(expr.trim()));
        }
    }
    (return_src.trim(), None)
}

fn parse_params(params_src: &str) -> Vec<FnParam> {
    let cleaned = params_src
        .trim()
        .trim_start_matches("given")
        .trim()
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim();

    if cleaned.is_empty() {
        return Vec::new();
    }

    cleaned
        .split(',')
        .flat_map(|part| part.split(" and "))
        .filter_map(parse_param)
        .collect()
}

fn parse_param(raw: &str) -> Option<FnParam> {
    let part = raw
        .trim()
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim();
    if part.is_empty() {
        return None;
    }

    let (name, ty) = if let Some((name, ty)) = part.rsplit_once(" of ") {
        (name.trim(), ty.trim())
    } else if let Some((name, ty)) = part.rsplit_once(':') {
        (name.trim(), ty.trim())
    } else {
        (part, "text")
    };

    let name = sanitize_identifier(name);
    let type_ann = parse_type(ty).unwrap_or_else(|| named_type("text"));
    Some(FnParam {
        name: ident(&name),
        type_ann,
    })
}

fn parse_type(raw: &str) -> Option<TypeExpr> {
    let first = raw
        .trim()
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim_end_matches('.')
        .split(|c: char| c == ',' || c == ';')
        .next()?
        .trim();
    if first.is_empty() {
        return None;
    }

    let ty = match first {
        "int" | "integer" | "count" | "index" => "integer",
        "num" | "number" | "float" | "double" | "real" => "number",
        "str" | "string" | "text" | "greeting" | "body" | "digest" | "output" | "result" => "text",
        "bool" | "boolean" | "success" | "flag" => "bool",
        "bytes" | "byte buffer" | "buffer" => "bytes",
        other => other.split_whitespace().last().unwrap_or(other),
    };

    Some(named_type(&sanitize_identifier(ty)))
}

fn parse_return_expr(raw: &str) -> Option<Expr> {
    let expr = raw.trim().trim_end_matches('.');
    if expr.is_empty() {
        return None;
    }

    parse_match_expr(expr)
        .or_else(|| parse_if_expr(expr))
        .or_else(|| parse_binary_expr(expr))
        .or_else(|| parse_atom_expr(expr))
}

/// Recognise prose match patterns and lower them to a chained `IfExpr`.
///
/// Supported patterns:
///   "match X when X is Y then Z, when X is A then B, otherwise C"
///   "when X is Y then Z, when X is A then B, otherwise C"
///
/// Each arm becomes an if/else-if in `IfExpr { condition, else_ifs, else_body }`.
/// The `otherwise` clause maps to `else_body`.
fn parse_match_expr(expr: &str) -> Option<Expr> {
    let trimmed = expr.trim();
    let lower = trimmed.to_ascii_lowercase();

    // Must start with "match " or "when " AND contain at least one ", when " to distinguish
    // multi-arm match from a simple single if/when expression.
    let arms_src: &str;
    if lower.starts_with("match ") {
        // "match X when ..." — strip "match X" preamble, the rest is the arms
        let after_match = &trimmed["match ".len()..];
        let after_match_lower = after_match.to_ascii_lowercase();
        // Find the first " when " to start the arms section
        let when_pos = after_match_lower.find(" when ")?;
        // The subject is optional context; arms start at the " when "
        arms_src = &after_match[when_pos + 1..]; // starts with "when ..."
    } else if lower.starts_with("when ") {
        arms_src = trimmed;
    } else {
        return None;
    }

    // Must have at least one ", when " separator to be a multi-arm match
    let arms_lower = arms_src.to_ascii_lowercase();
    if !arms_lower.contains(", when ") && !arms_lower.contains("; when ") {
        return None;
    }

    // Split into raw arm strings on ", when " / "; when " / ", otherwise " / "; otherwise "
    let raw_arms: Vec<&str> = {
        let mut result: Vec<&str> = Vec::new();
        let mut remaining = arms_src;
        loop {
            let rem_lower = remaining.to_ascii_lowercase();
            // Find next separator — try "when" arms first, then "otherwise"
            let sep_pos = rem_lower
                .find(", when ")
                .or_else(|| rem_lower.find("; when "))
                .or_else(|| rem_lower.find(", otherwise "))
                .or_else(|| rem_lower.find("; otherwise "));
            match sep_pos {
                None => {
                    result.push(remaining);
                    break;
                }
                Some(pos) => {
                    result.push(&remaining[..pos]);
                    remaining = &remaining[pos + 2..]; // skip ", " or "; ", leave "when ..."/"otherwise ..."
                }
            }
        }
        result
    };

    if raw_arms.len() < 2 {
        return None;
    }

    // Parse each arm. An arm is one of:
    //   "when <cond> then <result>"   where <cond> may include "is <value>"
    //   "otherwise <result>"
    //
    // Returns (condition_expr, result_expr) or None if it is the otherwise arm.
    let parse_arm = |arm: &str| -> Option<(Option<Expr>, Expr)> {
        let arm = arm.trim();
        let arm_lower = arm.to_ascii_lowercase();
        if arm_lower.starts_with("otherwise ") {
            let result_src = &arm["otherwise ".len()..];
            let result = parse_return_expr(result_src.trim())?;
            return Some((None, result));
        }
        // Strip leading "when "
        let cond_start = if arm_lower.starts_with("when ") {
            "when ".len()
        } else {
            0
        };
        let after_when = &arm[cond_start..];
        let after_when_lower = after_when.to_ascii_lowercase();
        let then_pos = after_when_lower.find(" then ")?;
        let cond_src = after_when[..then_pos].trim();
        let result_src = after_when[then_pos + " then ".len()..].trim();
        let condition = parse_condition_expr(cond_src)?;
        let result = parse_return_expr(result_src)?;
        Some((Some(condition), result))
    };

    // Build parsed arms list
    let mut parsed: Vec<(Option<Expr>, Expr)> = raw_arms
        .iter()
        .filter_map(|arm| parse_arm(arm))
        .collect();

    if parsed.is_empty() {
        return None;
    }

    // The last arm may be the "otherwise" arm (else_body)
    let else_body: Option<Block> = {
        if parsed.last().map(|(cond, _)| cond.is_none()).unwrap_or(false) {
            let (_, result) = parsed.pop().unwrap();
            Some(Block {
                stmts: vec![BlockStmt::Return(Some(result))],
                span: Span::default(),
            })
        } else {
            None
        }
    };

    if parsed.is_empty() {
        return None;
    }

    // First arm → IfExpr condition + then_body
    let (first_cond, first_result) = parsed.remove(0);
    let first_cond = first_cond?;

    // Remaining arms → else_ifs
    let else_ifs: Vec<(Expr, Block)> = parsed
        .into_iter()
        .filter_map(|(cond, result)| {
            let cond = cond?;
            Some((
                cond,
                Block {
                    stmts: vec![BlockStmt::Return(Some(result))],
                    span: Span::default(),
                },
            ))
        })
        .collect();

    Some(Expr::IfExpr(Box::new(IfExpr {
        condition: Box::new(first_cond),
        then_body: Block {
            stmts: vec![BlockStmt::Return(Some(first_result))],
            span: Span::default(),
        },
        else_ifs,
        else_body,
        span: Span::default(),
    })))
}

/// Recognise prose conditional patterns:
///   "if COND then X else Y"
///   "when COND then X otherwise Y"
///   "COND then X else Y"  (when "when"/"if" was dropped by the pipeline lexer)
///   "COND then X otherwise Y"
fn parse_if_expr(expr: &str) -> Option<Expr> {
    let lower = expr.to_ascii_lowercase();

    // Detect optional leading keyword: "if " or "when ".
    // Note: the pipeline lexer emits Tok::When which tok_prose_repr drops,
    // so "when COND then X otherwise Y" arrives as "COND then X otherwise Y".
    let (cond_start, else_keywords): (usize, &[&str]) = if lower.starts_with("if ") {
        (3usize, &["else", "otherwise"])
    } else if lower.starts_with("when ") {
        (5usize, &["otherwise", "else"])
    } else {
        // No leading keyword — still try to detect "COND then X else/otherwise Y"
        (0usize, &["else", "otherwise"])
    };

    // Split on " then " to find condition vs then-branch.
    // Must find at least one character of condition before " then ".
    let then_marker = " then ";
    let then_pos_in_slice = lower[cond_start..].find(then_marker)?;

    // Require a non-empty condition
    if then_pos_in_slice == 0 && cond_start == 0 {
        return None;
    }

    let cond_abs_end = cond_start + then_pos_in_slice;
    let cond_src = expr[cond_start..cond_abs_end].trim();
    let after_then = &expr[cond_abs_end + then_marker.len()..];
    let after_then_lower = after_then.to_ascii_lowercase();

    // Try each else keyword variant
    let mut then_src = after_then.trim();
    let mut else_src: Option<&str> = None;
    for kw in else_keywords {
        let marker = format!(" {} ", kw);
        if let Some(pos) = after_then_lower.find(marker.as_str()) {
            then_src = after_then[..pos].trim();
            else_src = Some(after_then[pos + marker.len()..].trim());
            break;
        }
    }

    // Validate that condition looks like a real comparison, not just a plain
    // identifier that happens to precede "then" (e.g., avoids false positives
    // on short identifiers like "x then y"). When cond_start == 0 (no leading
    // keyword), require the condition to contain a known comparison marker.
    if cond_start == 0 {
        let has_comparison = [
            "is greater than",
            "is less than",
            "is equal to",
            "is not equal to",
            "does not equal",
            "equals",
            ">",
            "<",
            ">=",
            "<=",
            "==",
            "!=",
        ]
        .iter()
        .any(|m| lower[..cond_abs_end].contains(m));
        if !has_comparison {
            return None;
        }
    }

    let condition = parse_condition_expr(cond_src)?;
    let then_expr = parse_return_expr(then_src)?;
    let else_body = else_src.and_then(parse_return_expr).map(|e| Block {
        stmts: vec![BlockStmt::Return(Some(e))],
        span: Span::default(),
    });

    Some(Expr::IfExpr(Box::new(IfExpr {
        condition: Box::new(condition),
        then_body: Block {
            stmts: vec![BlockStmt::Return(Some(then_expr))],
            span: Span::default(),
        },
        else_ifs: vec![],
        else_body,
        span: Span::default(),
    })))
}

/// Parse a condition expression, recognising comparison operators in prose
/// ("is greater than", "equals", "is less than", etc.) and falling back
/// to `parse_return_expr` for arbitrary expressions.
fn parse_condition_expr(cond: &str) -> Option<Expr> {
    use nom_ast::BinOp;

    for (needle, op) in [
        (" is greater than or equal to ", BinOp::Gte),
        (" is less than or equal to ", BinOp::Lte),
        (" is greater than ", BinOp::Gt),
        (" is less than ", BinOp::Lt),
        (" is not equal to ", BinOp::Neq),
        (" does not equal ", BinOp::Neq),
        (" equals ", BinOp::Eq),
        (" is equal to ", BinOp::Eq),
        (" >= ", BinOp::Gte),
        (" <= ", BinOp::Lte),
        (" != ", BinOp::Neq),
        (" == ", BinOp::Eq),
        (" > ", BinOp::Gt),
        (" < ", BinOp::Lt),
    ] {
        let lower = cond.to_ascii_lowercase();
        if let Some(pos) = lower.find(needle) {
            let left = parse_atom_expr(cond[..pos].trim())?;
            let right = parse_atom_expr(cond[pos + needle.len()..].trim())?;
            return Some(Expr::BinaryOp(Box::new(left), op, Box::new(right)));
        }
    }

    // Fall back: treat condition as a bare expression (e.g., identifier or call)
    parse_return_expr(cond).or_else(|| parse_atom_expr(cond))
}

fn parse_binary_expr(expr: &str) -> Option<Expr> {
    for (needle, op) in [
        (" plus ", BinOp::Add),
        (" + ", BinOp::Add),
        (" minus ", BinOp::Sub),
        (" - ", BinOp::Sub),
        (" times ", BinOp::Mul),
        (" multiplied by ", BinOp::Mul),
        (" * ", BinOp::Mul),
        (" divided by ", BinOp::Div),
        (" / ", BinOp::Div),
    ] {
        if let Some((left, right)) = expr.split_once(needle) {
            let left = parse_atom_expr(left)?;
            let right = parse_atom_expr(right)?;
            return Some(Expr::BinaryOp(Box::new(left), op, Box::new(right)));
        }
    }
    None
}

fn parse_atom_expr(raw: &str) -> Option<Expr> {
    let atom = raw.trim().trim_matches('`').trim();
    if atom.is_empty() {
        return None;
    }
    if (atom.starts_with('"') && atom.ends_with('"'))
        || (atom.starts_with('\'') && atom.ends_with('\''))
    {
        return Some(Expr::Literal(Literal::Text(
            atom[1..atom.len() - 1].to_string(),
        )));
    }
    if let Some(call) = parse_call_expr(atom) {
        return Some(call);
    }
    if let Ok(value) = atom.parse::<i64>() {
        return Some(Expr::Literal(Literal::Integer(value)));
    }
    if let Ok(value) = atom.parse::<f64>() {
        return Some(Expr::Literal(Literal::Number(value)));
    }
    match atom.to_ascii_lowercase().as_str() {
        "true" => Some(Expr::Literal(Literal::Bool(true))),
        "false" => Some(Expr::Literal(Literal::Bool(false))),
        "none" | "null" => Some(Expr::Literal(Literal::None)),
        _ => Some(Expr::Ident(ident(&sanitize_identifier(atom)))),
    }
}

fn parse_call_expr(raw: &str) -> Option<Expr> {
    if let Some(open_idx) = raw.find('(') {
        if raw.ends_with(')') {
            let callee = sanitize_identifier(&raw[..open_idx]);
            if callee.is_empty() {
                return None;
            }
            let args_src = &raw[open_idx + 1..raw.len() - 1];
            let args = parse_call_args(args_src)?;
            return Some(Expr::Call(CallExpr {
                callee: ident(&callee),
                args,
                span: Span::default(),
            }));
        }
    }

    if let Some((callee, arg)) = raw.split_once(" of ") {
        let callee = sanitize_identifier(callee);
        if callee.is_empty() {
            return None;
        }
        let arg = parse_atom_expr(arg)?;
        return Some(Expr::Call(CallExpr {
            callee: ident(&callee),
            args: vec![arg],
            span: Span::default(),
        }));
    }

    None
}

fn split_args_balanced(args_src: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, ch) in args_src.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                result.push(&args_src[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(&args_src[start..]);
    result
}

fn parse_call_args(args_src: &str) -> Option<Vec<Expr>> {
    let trimmed = args_src.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }

    split_args_balanced(trimmed)
        .into_iter()
        .map(parse_return_expr)
        .collect::<Option<Vec<_>>>()
}

fn default_expr_for_type(ty: &TypeExpr) -> Option<Expr> {
    match ty {
        TypeExpr::Named(name) => match name.name.as_str() {
            "integer" => Some(Expr::Literal(Literal::Integer(0))),
            "number" => Some(Expr::Literal(Literal::Number(0.0))),
            "bool" => Some(Expr::Literal(Literal::Bool(false))),
            "text" | "bytes" => Some(Expr::Literal(Literal::Text(String::new()))),
            "unit" => None,
            _ => Some(Expr::Literal(Literal::None)),
        },
        TypeExpr::Unit => None,
        _ => Some(Expr::Literal(Literal::None)),
    }
}

fn named_type(name: &str) -> TypeExpr {
    TypeExpr::Named(ident(name))
}

fn ident(name: &str) -> Identifier {
    Identifier {
        name: name.to_string(),
        span: Span::default(),
    }
}

fn sanitize_identifier(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_concept::stages::run_pipeline;

    fn bridged_fn(source: &SourceFile) -> &FnDef {
        source.declarations[0]
            .statements
            .iter()
            .find_map(|stmt| match stmt {
                Statement::FnDef(fn_def) => Some(fn_def),
                _ => None,
            })
            .expect("FnDef")
    }

    #[test]
    fn bridges_function_signature_to_typed_fn_def() {
        let pipeline = run_pipeline("the function fetch_url is given a url of text, returns text.")
            .expect("pipeline");
        let source = bridge_to_ast(&pipeline, Some("fixture.nomtu".to_string()));

        assert_eq!(source.path.as_deref(), Some("fixture.nomtu"));
        assert_eq!(source.declarations.len(), 1);
        let decl = &source.declarations[0];
        assert_eq!(decl.name.name, "fetch_url");
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.name.name, "fetch_url");
        assert_eq!(fn_def.params.len(), 1);
        assert_eq!(fn_def.params[0].name.name, "url");
        assert!(matches!(
            &fn_def.params[0].type_ann,
            TypeExpr::Named(name) if name.name == "text"
        ));
        assert!(matches!(
            &fn_def.return_type,
            Some(TypeExpr::Named(name)) if name.name == "text"
        ));
        assert!(matches!(
            fn_def.body.stmts.as_slice(),
            [BlockStmt::Return(Some(Expr::Literal(Literal::Text(value))))] if value.is_empty()
        ));
    }

    #[test]
    fn bridges_multiple_params_and_numeric_return() {
        let pipeline = run_pipeline(
            "the function add is given left of number, right of number, returns number.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.params.len(), 2);
        assert_eq!(fn_def.params[0].name.name, "left");
        assert_eq!(fn_def.params[1].name.name, "right");
        assert!(matches!(
            fn_def.body.stmts.as_slice(),
            [BlockStmt::Return(Some(Expr::Literal(Literal::Number(0.0))))]
        ));
    }

    #[test]
    fn bridges_explicit_arithmetic_return_body() {
        let pipeline = run_pipeline(
            "the function add is given left of number, right of number, returns number by returning left plus right.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert!(matches!(
            fn_def.body.stmts.as_slice(),
            [BlockStmt::Return(Some(Expr::BinaryOp(left, BinOp::Add, right)))]
                if matches!(&**left, Expr::Ident(name) if name.name == "left")
                    && matches!(&**right, Expr::Ident(name) if name.name == "right")
        ));
    }

    #[test]
    fn bridges_explicit_call_return_body() {
        let pipeline = run_pipeline(
            "the function apply_double is given value of number, returns number by returning double_value(value).",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert!(matches!(
            fn_def.body.stmts.as_slice(),
            [BlockStmt::Return(Some(Expr::Call(call)))]
                if call.callee.name == "double_value"
                    && matches!(call.args.as_slice(), [Expr::Ident(name)] if name.name == "value")
        ));
    }

    #[test]
    fn bridges_prose_call_return_body() {
        let pipeline = run_pipeline(
            "the function apply_double is given value of number, returns number by returning double_value of value.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert!(matches!(
            fn_def.body.stmts.as_slice(),
            [BlockStmt::Return(Some(Expr::Call(call)))]
                if call.callee.name == "double_value"
                    && matches!(call.args.as_slice(), [Expr::Ident(name)] if name.name == "value")
        ));
    }

    #[test]
    fn bridges_if_then_else_conditional_return() {
        let pipeline = run_pipeline(
            "the function abs_val is given x of number, returns number by returning if x is less than 0 then negate(x) else x.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.body.stmts.len(), 1);
        match &fn_def.body.stmts[0] {
            BlockStmt::Return(Some(Expr::IfExpr(if_expr))) => {
                // condition: x < 0
                assert!(matches!(
                    if_expr.condition.as_ref(),
                    Expr::BinaryOp(left, BinOp::Lt, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "x")
                            && matches!(&**right, Expr::Literal(Literal::Integer(0)))
                ));
                // then body returns negate(x)
                assert!(matches!(
                    if_expr.then_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Call(call)))]
                        if call.callee.name == "negate"
                ));
                // else body returns x
                let else_body = if_expr.else_body.as_ref().expect("else_body");
                assert!(matches!(
                    else_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(name)))] if name.name == "x"
                ));
            }
            other => panic!("expected IfExpr return, got {:?}", other),
        }
    }

    #[test]
    fn bridges_when_then_otherwise_conditional_return() {
        let pipeline = run_pipeline(
            "the function clamp is given v of number, returns number by returning when v is greater than 100 then 100 otherwise v.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.body.stmts.len(), 1);
        match &fn_def.body.stmts[0] {
            BlockStmt::Return(Some(Expr::IfExpr(if_expr))) => {
                // condition: v > 100
                assert!(matches!(
                    if_expr.condition.as_ref(),
                    Expr::BinaryOp(left, BinOp::Gt, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "v")
                            && matches!(&**right, Expr::Literal(Literal::Integer(100)))
                ));
                // then body returns 100
                assert!(matches!(
                    if_expr.then_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Literal(Literal::Integer(
                        100
                    ))))]
                ));
                // else body returns v
                let else_body = if_expr.else_body.as_ref().expect("else_body");
                assert!(matches!(
                    else_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(name)))] if name.name == "v"
                ));
            }
            other => panic!("expected IfExpr return, got {:?}", other),
        }
    }

    #[test]
    fn bridges_if_then_without_else() {
        let pipeline = run_pipeline(
            "the function log_if_positive is given x of number, returns number by returning if x is greater than 0 then x.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.body.stmts.len(), 1);
        match &fn_def.body.stmts[0] {
            BlockStmt::Return(Some(Expr::IfExpr(if_expr))) => {
                assert!(if_expr.else_body.is_none());
                assert!(matches!(
                    if_expr.then_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(name)))] if name.name == "x"
                ));
            }
            other => panic!("expected IfExpr return, got {:?}", other),
        }
    }

    #[test]
    fn bridges_multi_statement_body_semicolon_then() {
        let pipeline = run_pipeline(
            "the function compute is given a of number, b of number, returns number by returning a plus b; then a times b.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.body.stmts.len(), 2);
        assert!(matches!(
            &fn_def.body.stmts[0],
            BlockStmt::Expr(Expr::BinaryOp(_, BinOp::Add, _))
        ));
        assert!(matches!(
            &fn_def.body.stmts[1],
            BlockStmt::Return(Some(Expr::BinaryOp(_, BinOp::Mul, _)))
        ));
    }

    #[test]
    fn bridges_multi_statement_body_comma_then() {
        let pipeline = run_pipeline(
            "the function pipeline is given x of number, returns number by returning double_value(x), then x plus 1.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.body.stmts.len(), 2);
        assert!(matches!(
            &fn_def.body.stmts[0],
            BlockStmt::Expr(Expr::Call(call)) if call.callee.name == "double_value"
        ));
        assert!(matches!(
            &fn_def.body.stmts[1],
            BlockStmt::Return(Some(Expr::BinaryOp(_, BinOp::Add, _)))
        ));
    }

    #[test]
    fn parse_for_each_loop_simple_body() {
        // Direct unit test of parse_loop_stmt for "for each <var> in <collection> do <body>"
        let result = parse_loop_stmt("for each item in items do process(item)");
        assert!(result.is_some(), "expected a loop stmt");
        match result.unwrap() {
            BlockStmt::For(for_stmt) => {
                assert_eq!(for_stmt.binding.name, "item");
                assert!(matches!(&for_stmt.iterable, Expr::Ident(n) if n.name == "items"));
                assert_eq!(for_stmt.body.stmts.len(), 1);
                assert!(matches!(
                    &for_stmt.body.stmts[0],
                    BlockStmt::Expr(Expr::Call(c)) if c.callee.name == "process"
                ));
            }
            other => panic!("expected For, got {:?}", other),
        }
    }

    #[test]
    fn parse_for_loop_without_each() {
        let result = parse_loop_stmt("for x in collection do compute(x)");
        assert!(result.is_some(), "expected a loop stmt");
        match result.unwrap() {
            BlockStmt::For(for_stmt) => {
                assert_eq!(for_stmt.binding.name, "x");
                assert!(matches!(&for_stmt.iterable, Expr::Ident(n) if n.name == "collection"));
            }
            other => panic!("expected For, got {:?}", other),
        }
    }

    #[test]
    fn parse_while_loop_with_condition() {
        let result = parse_loop_stmt("while count is greater than 0 do decrement(count)");
        assert!(result.is_some(), "expected a loop stmt");
        match result.unwrap() {
            BlockStmt::While(while_stmt) => {
                assert!(matches!(
                    &while_stmt.condition,
                    Expr::BinaryOp(left, BinOp::Gt, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "count")
                            && matches!(&**right, Expr::Literal(Literal::Integer(0)))
                ));
                assert_eq!(while_stmt.body.stmts.len(), 1);
                assert!(matches!(
                    &while_stmt.body.stmts[0],
                    BlockStmt::Expr(Expr::Call(c)) if c.callee.name == "decrement"
                ));
            }
            other => panic!("expected While, got {:?}", other),
        }
    }

    #[test]
    fn parse_repeat_until_loop() {
        let result = parse_loop_stmt("repeat work(task) until done is equal to true");
        assert!(result.is_some(), "expected a loop stmt");
        match result.unwrap() {
            BlockStmt::While(while_stmt) => {
                // condition: done == true
                assert!(matches!(
                    &while_stmt.condition,
                    Expr::BinaryOp(left, BinOp::Eq, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "done")
                            && matches!(&**right, Expr::Literal(Literal::Bool(true)))
                ));
                assert!(matches!(
                    &while_stmt.body.stmts[0],
                    BlockStmt::Expr(Expr::Call(c)) if c.callee.name == "work"
                ));
            }
            other => panic!("expected While (repeat-until), got {:?}", other),
        }
    }

    #[test]
    fn loop_as_part_of_multi_statement_body() {
        // A function whose body is: setup(x); then for each item in list do process(item)
        // We test parse_multi_stmt_body directly since we need a crafted source.
        let body_src = "setup(x); then for each item in list do process(item)";
        let return_type = named_type("unit");
        let stmts = parse_multi_stmt_body(body_src, &return_type);

        assert_eq!(stmts.len(), 2, "expected 2 stmts, got {:?}", stmts);
        assert!(matches!(
            &stmts[0],
            BlockStmt::Expr(Expr::Call(c)) if c.callee.name == "setup"
        ));
        match &stmts[1] {
            BlockStmt::For(for_stmt) => {
                assert_eq!(for_stmt.binding.name, "item");
                assert!(matches!(&for_stmt.iterable, Expr::Ident(n) if n.name == "list"));
            }
            other => panic!("expected For in multi-stmt, got {:?}", other),
        }
    }

    #[test]
    fn bridges_contract_requires_and_ensures_as_describe_stmts() {
        let pipeline = run_pipeline(
            "the function verify_token is given a token of text, returns bool.\n  requires the token is non-empty.\n  ensures the result is valid.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let decl = &source.declarations[0];

        // Collect Describe texts
        let describes: Vec<&str> = decl
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::Describe(d) => Some(d.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(
            describes.iter().any(|t| t.starts_with("requires:")),
            "expected a requires: describe stmt, got {:?}",
            describes
        );
        assert!(
            describes.iter().any(|t| t.starts_with("ensures:")),
            "expected an ensures: describe stmt, got {:?}",
            describes
        );
    }

    #[test]
    fn bridges_benefit_effect_as_effects_stmt_with_good_modifier() {
        let pipeline = run_pipeline(
            "the function fetch_cached is given a key of text, returns text.\n  benefit cache_hit.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let decl = &source.declarations[0];

        let effects_stmts: Vec<_> = decl
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::Effects(e) => Some(e),
                _ => None,
            })
            .collect();

        assert_eq!(effects_stmts.len(), 1, "expected 1 Effects stmt");
        let e = effects_stmts[0];
        assert_eq!(e.modifier, Some(EffectModifier::Good));
        assert!(
            e.effects.iter().any(|id| id.name == "cache_hit"),
            "expected cache_hit in effects, got {:?}",
            e.effects
        );
    }

    #[test]
    fn bridges_hazard_effect_as_effects_stmt_with_bad_modifier() {
        let pipeline = run_pipeline(
            "the function call_api is given a url of text, returns text.\n  hazard timeout.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let decl = &source.declarations[0];

        let effects_stmts: Vec<_> = decl
            .statements
            .iter()
            .filter_map(|s| match s {
                Statement::Effects(e) => Some(e),
                _ => None,
            })
            .collect();

        assert_eq!(effects_stmts.len(), 1, "expected 1 Effects stmt");
        let e = effects_stmts[0];
        assert_eq!(e.modifier, Some(EffectModifier::Bad));
        assert!(
            e.effects.iter().any(|id| id.name == "timeout"),
            "expected timeout in effects, got {:?}",
            e.effects
        );
    }

    #[test]
    fn parse_match_two_arm_with_otherwise() {
        // "when x equals 1 then a, when x equals 2 then b, otherwise c"
        let result = parse_match_expr("when x equals 1 then a, when x equals 2 then b, otherwise c");
        assert!(result.is_some(), "expected a match expr");
        match result.unwrap() {
            Expr::IfExpr(if_expr) => {
                // First arm: x == 1 → a
                assert!(matches!(
                    if_expr.condition.as_ref(),
                    Expr::BinaryOp(left, BinOp::Eq, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "x")
                            && matches!(&**right, Expr::Literal(Literal::Integer(1)))
                ));
                assert!(matches!(
                    if_expr.then_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "a"
                ));
                // One else-if: x == 2 → b
                assert_eq!(if_expr.else_ifs.len(), 1);
                let (elif_cond, elif_body) = &if_expr.else_ifs[0];
                assert!(matches!(
                    elif_cond,
                    Expr::BinaryOp(left, BinOp::Eq, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "x")
                            && matches!(&**right, Expr::Literal(Literal::Integer(2)))
                ));
                assert!(matches!(
                    elif_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "b"
                ));
                // Otherwise → c
                let else_body = if_expr.else_body.as_ref().expect("else_body");
                assert!(matches!(
                    else_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "c"
                ));
            }
            other => panic!("expected IfExpr, got {:?}", other),
        }
    }

    #[test]
    fn parse_match_three_arm_without_otherwise() {
        // Three arms, no otherwise clause
        let result = parse_match_expr(
            "when score is greater than 90 then grade_a, when score is greater than 70 then grade_b, when score is greater than 50 then grade_c",
        );
        assert!(result.is_some(), "expected a match expr");
        match result.unwrap() {
            Expr::IfExpr(if_expr) => {
                // First arm: score > 90 → grade_a
                assert!(matches!(
                    if_expr.condition.as_ref(),
                    Expr::BinaryOp(_, BinOp::Gt, _)
                ));
                assert!(matches!(
                    if_expr.then_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "grade_a"
                ));
                // Two else-ifs
                assert_eq!(if_expr.else_ifs.len(), 2);
                assert!(matches!(
                    &if_expr.else_ifs[0].0,
                    Expr::BinaryOp(_, BinOp::Gt, _)
                ));
                assert!(matches!(
                    if_expr.else_ifs[1].1.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "grade_c"
                ));
                // No else body
                assert!(if_expr.else_body.is_none());
            }
            other => panic!("expected IfExpr, got {:?}", other),
        }
    }

    #[test]
    fn parse_match_keyword_prefix_form() {
        // "match status when status equals 0 then ok, when status equals 1 then error, otherwise unknown"
        let result = parse_match_expr(
            "match status when status equals 0 then ok, when status equals 1 then error, otherwise unknown",
        );
        assert!(result.is_some(), "expected a match expr from 'match' prefix");
        match result.unwrap() {
            Expr::IfExpr(if_expr) => {
                // First arm: status == 0 → ok
                assert!(matches!(
                    if_expr.condition.as_ref(),
                    Expr::BinaryOp(left, BinOp::Eq, right)
                        if matches!(&**left, Expr::Ident(n) if n.name == "status")
                            && matches!(&**right, Expr::Literal(Literal::Integer(0)))
                ));
                assert!(matches!(
                    if_expr.then_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "ok"
                ));
                // else-if: status == 1 → error
                assert_eq!(if_expr.else_ifs.len(), 1);
                assert!(matches!(
                    if_expr.else_ifs[0].1.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "error"
                ));
                // otherwise → unknown
                let else_body = if_expr.else_body.as_ref().expect("else_body");
                assert!(matches!(
                    else_body.stmts.as_slice(),
                    [BlockStmt::Return(Some(Expr::Ident(n)))] if n.name == "unknown"
                ));
            }
            other => panic!("expected IfExpr, got {:?}", other),
        }
    }

    #[test]
    fn parse_match_does_not_trigger_on_single_when_then() {
        // A single "when ... then ... otherwise ..." should NOT be matched by parse_match_expr
        // (it has no ", when " separator) — it falls through to parse_if_expr instead.
        let result = parse_match_expr("when x is greater than 0 then x otherwise zero");
        assert!(
            result.is_none(),
            "single-arm 'when' should not match parse_match_expr"
        );
    }

    #[test]
    fn contracts_appear_before_fn_def_in_statement_order() {
        let pipeline = run_pipeline(
            "the function add is given left of number, right of number, returns number.\n  requires left is non-negative.\n  ensures the result is the sum.",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let decl = &source.declarations[0];

        // Find positions of Describe (contract) stmts vs FnDef stmt
        let mut first_contract_pos: Option<usize> = None;
        let mut fn_def_pos: Option<usize> = None;
        for (i, stmt) in decl.statements.iter().enumerate() {
            match stmt {
                Statement::Describe(d)
                    if d.text.starts_with("requires:") || d.text.starts_with("ensures:") =>
                {
                    if first_contract_pos.is_none() {
                        first_contract_pos = Some(i);
                    }
                }
                Statement::FnDef(_) => {
                    fn_def_pos = Some(i);
                }
                _ => {}
            }
        }

        let contract_pos = first_contract_pos.expect("expected contract Describe stmts");
        let fn_pos = fn_def_pos.expect("expected FnDef stmt");
        assert!(
            contract_pos < fn_pos,
            "contracts (pos {}) should appear before FnDef (pos {})",
            contract_pos,
            fn_pos
        );
    }

    #[test]
    fn parse_nested_call_expr() {
        let expr = parse_call_expr("outer(inner(x), y)").expect("nested call");
        match expr {
            Expr::Call(call) => {
                assert_eq!(call.callee.name, "outer");
                assert_eq!(call.args.len(), 2);
                match &call.args[0] {
                    Expr::Call(inner) => {
                        assert_eq!(inner.callee.name, "inner");
                        assert_eq!(inner.args.len(), 1);
                        assert!(matches!(&inner.args[0], Expr::Ident(n) if n.name == "x"));
                    }
                    other => panic!("expected inner Call, got {:?}", other),
                }
                assert!(matches!(&call.args[1], Expr::Ident(n) if n.name == "y"));
            }
            other => panic!("expected outer Call, got {:?}", other),
        }
    }

    #[test]
    fn parse_deeply_nested_call() {
        let expr = parse_call_expr("a(b(c(d)))").expect("deeply nested call");
        match expr {
            Expr::Call(a) => {
                assert_eq!(a.callee.name, "a");
                assert_eq!(a.args.len(), 1);
                match &a.args[0] {
                    Expr::Call(b) => {
                        assert_eq!(b.callee.name, "b");
                        assert_eq!(b.args.len(), 1);
                        match &b.args[0] {
                            Expr::Call(c) => {
                                assert_eq!(c.callee.name, "c");
                                assert_eq!(c.args.len(), 1);
                                assert!(matches!(&c.args[0], Expr::Ident(n) if n.name == "d"));
                            }
                            other => panic!("expected c Call, got {:?}", other),
                        }
                    }
                    other => panic!("expected b Call, got {:?}", other),
                }
            }
            other => panic!("expected a Call, got {:?}", other),
        }
    }

    #[test]
    fn bridges_nested_call_return_body() {
        let pipeline = run_pipeline(
            "the function foo is given x of number, returns number by returning outer(inner(x), 1).",
        )
        .expect("pipeline");
        let source = bridge_to_ast(&pipeline, None);
        let fn_def = bridged_fn(&source);

        assert_eq!(fn_def.body.stmts.len(), 1);
        match &fn_def.body.stmts[0] {
            BlockStmt::Return(Some(Expr::Call(outer))) => {
                assert_eq!(outer.callee.name, "outer");
                assert_eq!(outer.args.len(), 2);
                match &outer.args[0] {
                    Expr::Call(inner) => {
                        assert_eq!(inner.callee.name, "inner");
                        assert_eq!(inner.args.len(), 1);
                        assert!(matches!(&inner.args[0], Expr::Ident(n) if n.name == "x"));
                    }
                    other => panic!("expected inner Call in first arg, got {:?}", other),
                }
                assert!(matches!(
                    &outer.args[1],
                    Expr::Literal(Literal::Integer(1))
                ));
            }
            other => panic!("expected Return(Call), got {:?}", other),
        }
    }
}
