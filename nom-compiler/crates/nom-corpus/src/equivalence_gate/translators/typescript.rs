use super::{TranslatedItem, TranslationError};
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{
    BinaryOp, Decl, Expr, Lit, ModuleItem, Pat, Stmt, TsKeywordTypeKind, TsType, TsTypeAnn,
    VarDeclKind,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

/// Translate a TypeScript source string to a list of `TranslatedItem`s, one
/// per translatable top-level function or arrow-function declaration.
/// Unsupported items (class, interface, generics, async, …) are silently
/// skipped — a file with 3 fns and 1 class returns 3 items.
///
/// Returns `Err(TranslationError::Parse(_))` only if the whole module fails
/// to parse at the SWC level.
pub fn translate(source: &str) -> Result<Vec<TranslatedItem>, TranslationError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        Lrc::new(FileName::Custom("input.ts".into())),
        source.to_string(),
    );

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax { ..Default::default() }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| TranslationError::Parse(format!("{e:?}")))?;

    let mut items: Vec<TranslatedItem> = Vec::new();

    for item in &module.body {
        // Unwrap ExportDecl wrappers transparently; skip unsupported module-level decls.
        let decl_item: Option<&Decl> = match item {
            ModuleItem::ModuleDecl(md) => match md {
                swc_ecma_ast::ModuleDecl::ExportDecl(ed) => Some(&ed.decl),
                _ => continue, // import/re-export/etc — skip at item level
            },
            ModuleItem::Stmt(stmt) => match stmt {
                Stmt::Decl(d) => Some(d),
                _ => continue, // non-declaration statement — skip at item level
            },
        };

        match decl_item {
            Some(Decl::Fn(fn_decl)) => {
                let func = &fn_decl.function;
                let name = fn_decl.ident.sym.as_str().to_string();
                if let Ok(body) = translate_function(&name, func) {
                    let sig_line = fn_signature_line(&name, func);
                    items.push(TranslatedItem { name, summary: sig_line, nom_body: body });
                }
                // else: skip at item level
            }
            Some(Decl::Var(var_decl)) => {
                // Accept `const/let foo = (x: T): R => expr` at top level.
                if var_decl.kind != VarDeclKind::Const && var_decl.kind != VarDeclKind::Let {
                    continue;
                }
                if var_decl.decls.len() != 1 {
                    continue;
                }
                let decl = &var_decl.decls[0];
                let name = match &decl.name {
                    Pat::Ident(bi) => bi.id.sym.as_str().to_string(),
                    _ => continue,
                };
                let init = match decl.init.as_deref() {
                    Some(e) => e,
                    None => continue,
                };
                if let Expr::Arrow(arrow) = init {
                    if let Ok(body) = translate_arrow(&name, arrow) {
                        let sig_line = arrow_signature_line(&name, arrow);
                        items.push(TranslatedItem { name, summary: sig_line, nom_body: body });
                    }
                    // else: skip at item level
                }
                // non-arrow const/let: skip at item level
            }
            // All other decl kinds (class, interface, type alias, enum, …): skip.
            _ => continue,
        }
    }

    Ok(items)
}

/// Build a human-readable signature line for a `function` decl.
fn fn_signature_line(name: &str, func: &swc_ecma_ast::Function) -> String {
    let params: Vec<String> = func.params.iter()
        .filter_map(|p| {
            if let Ok((pname, ty)) = extract_param(&p.pat) { Some(format!("{pname}: {ty}")) }
            else { None }
        })
        .collect();
    let ret = func.return_type.as_ref()
        .and_then(|a| ts_type_to_nom(a).ok())
        .unwrap_or_else(|| "unit".to_string());
    format!("fn {name}({}) -> {ret}", params.join(", "))
}

/// Build a human-readable signature line for an arrow-function decl.
fn arrow_signature_line(name: &str, arrow: &swc_ecma_ast::ArrowExpr) -> String {
    let params: Vec<String> = arrow.params.iter()
        .filter_map(|p| {
            if let Ok((pname, ty)) = extract_param(p) { Some(format!("{pname}: {ty}")) }
            else { None }
        })
        .collect();
    let ret = arrow.return_type.as_ref()
        .and_then(|a| ts_type_to_nom(a).ok())
        .unwrap_or_else(|| "unit".to_string());
    format!("fn {name}({}) -> {ret}", params.join(", "))
}

fn translate_function(
    name: &str,
    func: &swc_ecma_ast::Function,
) -> Result<String, TranslationError> {
    if !func.type_params.is_none() {
        return Err(TranslationError::Unsupported(
            "generic function (type parameters)".into(),
        ));
    }
    if func.is_async {
        return Err(TranslationError::Unsupported("async function".into()));
    }
    if func.is_generator {
        return Err(TranslationError::Unsupported("generator function".into()));
    }

    let mut params: Vec<String> = Vec::new();
    for p in &func.params {
        let (pname, ty) = extract_param(&p.pat)?;
        params.push(format!("{pname}: {ty}"));
    }

    let ret = match &func.return_type {
        Some(ann) => ts_type_to_nom(ann)?,
        None => "unit".to_string(),
    };

    let body = func
        .body
        .as_ref()
        .ok_or_else(|| TranslationError::Unsupported("function without body".into()))?;

    let mut out = format!("fn {name}({}) -> {ret} {{\n", params.join(", "));
    out.push_str(&block_to_nom(body, 1)?);
    out.push_str("}\n\n");
    Ok(out)
}

fn translate_arrow(
    name: &str,
    arrow: &swc_ecma_ast::ArrowExpr,
) -> Result<String, TranslationError> {
    if !arrow.type_params.is_none() {
        return Err(TranslationError::Unsupported(
            "generic arrow function (type parameters)".into(),
        ));
    }
    if arrow.is_async {
        return Err(TranslationError::Unsupported("async arrow function".into()));
    }

    let mut params: Vec<String> = Vec::new();
    for p in &arrow.params {
        let (pname, ty) = extract_param(p)?;
        params.push(format!("{pname}: {ty}"));
    }

    let ret = match &arrow.return_type {
        Some(ann) => ts_type_to_nom(ann)?,
        None => "unit".to_string(),
    };

    let mut out = format!("fn {name}({}) -> {ret} {{\n", params.join(", "));
    match &*arrow.body {
        swc_ecma_ast::BlockStmtOrExpr::BlockStmt(block) => {
            out.push_str(&block_to_nom(block, 1)?);
        }
        swc_ecma_ast::BlockStmtOrExpr::Expr(expr) => {
            // Single-expression body: implicit return.
            let pad = "  ";
            out.push_str(&format!("{pad}return {}\n", expr_to_nom(expr)?));
        }
    }
    out.push_str("}\n\n");
    Ok(out)
}

fn extract_param(pat: &Pat) -> Result<(String, String), TranslationError> {
    match pat {
        Pat::Ident(bi) => {
            let pname = bi.id.sym.as_str().to_string();
            let ty = match &bi.type_ann {
                Some(ann) => ts_type_to_nom(ann)?,
                None => {
                    return Err(TranslationError::Unsupported(format!(
                        "param `{pname}` missing type annotation"
                    )))
                }
            };
            Ok((pname, ty))
        }
        _ => Err(TranslationError::Unsupported(
            "destructuring parameter".into(),
        )),
    }
}

fn ts_type_to_nom(ann: &TsTypeAnn) -> Result<String, TranslationError> {
    match &*ann.type_ann {
        TsType::TsKeywordType(kw) => match kw.kind {
            TsKeywordTypeKind::TsNumberKeyword => Ok("integer".into()),
            TsKeywordTypeKind::TsStringKeyword => Ok("text".into()),
            TsKeywordTypeKind::TsBooleanKeyword => Ok("boolean".into()),
            TsKeywordTypeKind::TsVoidKeyword => Ok("unit".into()),
            _ => Err(TranslationError::Unsupported(format!(
                "TS keyword type `{:?}`",
                kw.kind
            ))),
        },
        _ => Err(TranslationError::Unsupported("complex TS type".into())),
    }
}

fn block_to_nom(
    block: &swc_ecma_ast::BlockStmt,
    indent: usize,
) -> Result<String, TranslationError> {
    let pad = "  ".repeat(indent);
    let mut out = String::new();
    let stmts = &block.stmts;
    for (i, stmt) in stmts.iter().enumerate() {
        let is_last = i == stmts.len() - 1;
        match stmt {
            Stmt::Decl(Decl::Var(var)) => {
                if var.decls.len() != 1 {
                    return Err(TranslationError::Unsupported(
                        "multiple declarators in local var".into(),
                    ));
                }
                let d = &var.decls[0];
                let vname = match &d.name {
                    Pat::Ident(bi) => bi.id.sym.as_str().to_string(),
                    _ => {
                        return Err(TranslationError::Unsupported(
                            "destructuring in local let/const".into(),
                        ))
                    }
                };
                let rhs = d.init.as_deref().ok_or_else(|| {
                    TranslationError::Unsupported("uninitialized local let/const".into())
                })?;
                out.push_str(&format!("{pad}let {vname} = {}\n", expr_to_nom(rhs)?));
            }
            Stmt::Return(ret_stmt) => {
                let e = ret_stmt
                    .arg
                    .as_deref()
                    .ok_or_else(|| TranslationError::Unsupported("bare return".into()))?;
                out.push_str(&format!("{pad}return {}\n", expr_to_nom(e)?));
            }
            Stmt::Expr(expr_stmt) => {
                let s = expr_to_nom(&expr_stmt.expr)?;
                if is_last {
                    // Tail expression — treat as implicit return.
                    out.push_str(&format!("{pad}return {s}\n"));
                } else {
                    out.push_str(&format!("{pad}{s}\n"));
                }
            }
            _ => {
                return Err(TranslationError::Unsupported(
                    "statement kind not supported".into(),
                ))
            }
        }
    }
    Ok(out)
}

fn expr_to_nom(expr: &Expr) -> Result<String, TranslationError> {
    match expr {
        Expr::Lit(lit) => match lit {
            Lit::Num(n) => Ok(n.value.to_string()),
            Lit::Str(s) => {
                let raw = s.value.to_atom_lossy();
                Ok(format!("\"{}\"", raw.as_str()))
            }
            Lit::Bool(b) => Ok(b.value.to_string()),
            _ => Err(TranslationError::Unsupported("literal kind".into())),
        },
        Expr::Ident(id) => Ok(id.sym.as_str().to_string()),
        Expr::Bin(bin) => {
            let l = expr_to_nom(&bin.left)?;
            let r = expr_to_nom(&bin.right)?;
            let op = match bin.op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                _ => return Err(TranslationError::Unsupported("binary operator".into())),
            };
            Ok(format!("({l} {op} {r})"))
        }
        Expr::Paren(p) => expr_to_nom(&p.expr),
        Expr::Unary(u) => {
            use swc_ecma_ast::UnaryOp;
            let e = expr_to_nom(&u.arg)?;
            match u.op {
                UnaryOp::Minus => Ok(format!("-{e}")),
                _ => Err(TranslationError::Unsupported("unary operator".into())),
            }
        }
        _ => Err(TranslationError::Unsupported("expression kind".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_add_fn_translates() {
        let src = "function add(a: number, b: number): number { return a + b; }";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "add");
        assert!(items[0].summary.contains("fn add(a: integer, b: integer) -> integer"));
        assert!(items[0].nom_body.contains("return (a + b)"));
    }

    #[test]
    fn arrow_fn_translates() {
        let src = "const double = (x: number): number => x * 2;";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "double");
        assert!(items[0].summary.contains("fn double(x: integer) -> integer"));
    }

    #[test]
    fn class_skipped_not_error() {
        // Class declarations are now silently skipped (item-level reject).
        let src = "class Foo { x: number; }";
        let items = translate(src).unwrap();
        assert!(items.is_empty(), "class should be skipped, got {items:?}");
    }

    #[test]
    fn generic_fn_skipped_not_error() {
        // Generic fns are silently skipped.
        let src = "function foo<T>(x: T): T { return x; }";
        let items = translate(src).unwrap();
        assert!(items.is_empty(), "generic fn should be skipped");
    }

    #[test]
    fn multi_item_ts_translate() {
        // Two translatable fns + one class (skipped) → 2 items.
        let src = r#"
            function add(a: number, b: number): number { return a + b; }
            class Ignored {}
            function sub(a: number, b: number): number { return a - b; }
        "#;
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 2, "expected 2 items (class skipped), got {items:?}");
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"add"), "missing 'add'");
        assert!(names.contains(&"sub"), "missing 'sub'");
    }
}
