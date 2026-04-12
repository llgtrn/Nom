use super::{TranslatedItem, TranslationError};
use syn::{
    parse_file, Block, BinOp, Expr, FnArg, Item, ItemFn, Lit, Pat, PathSegment, ReturnType, Stmt,
    Type, UnOp,
};

/// Translate a Rust source string to a list of `TranslatedItem`s, one per
/// translatable top-level fn.  Items that cannot be translated (generics,
/// async, unsupported types, …) are silently skipped — a file with 5 fns
/// where 4 translate cleanly and 1 has generics returns 4 items.
///
/// Returns `Err(TranslationError::Parse(_))` only if the whole file fails
/// to parse at the syn level.
pub fn translate(source: &str) -> Result<Vec<TranslatedItem>, TranslationError> {
    let file = parse_file(source).map_err(|e| TranslationError::Parse(e.to_string()))?;
    let mut items: Vec<TranslatedItem> = Vec::new();
    for item in file.items {
        if let Item::Fn(ItemFn { sig, block, .. }) = item {
            // Item-level reject: skip (don't Err) unsupported fns.
            if !sig.generics.params.is_empty() { continue; }
            if sig.asyncness.is_some() { continue; }

            let name = sig.ident.to_string();

            // Collect params; skip fn if any param type is unsupported.
            let mut params: Vec<String> = Vec::new();
            let mut ok = true;
            for arg in &sig.inputs {
                match arg {
                    FnArg::Receiver(_) => { ok = false; break; }
                    FnArg::Typed(pt) => {
                        let arg_name = match &*pt.pat {
                            Pat::Ident(pi) => pi.ident.to_string(),
                            _ => { ok = false; break; }
                        };
                        match type_to_nom(&pt.ty) {
                            Ok(ty) => params.push(format!("{arg_name}: {ty}")),
                            Err(_) => { ok = false; break; }
                        }
                    }
                }
            }
            if !ok { continue; }

            let ret = match &sig.output {
                ReturnType::Default => "unit".to_string(),
                ReturnType::Type(_, ty) => match type_to_nom(ty) {
                    Ok(t) => t,
                    Err(_) => continue,
                },
            };

            let nom_body_stmts = match block_to_nom(&block, 1) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let sig_line = format!("fn {name}({}) -> {ret}", params.join(", "));
            let nom_body = format!("{sig_line} {{\n{nom_body_stmts}}}\n\n");

            items.push(TranslatedItem {
                name,
                summary: sig_line,
                nom_body,
            });
        }
        // Non-fn items (struct, enum, impl, …) are silently skipped at item level.
    }
    Ok(items)
}

fn type_to_nom(ty: &Type) -> Result<String, TranslationError> {
    match ty {
        Type::Path(tp) => {
            let seg: &PathSegment = tp
                .path
                .segments
                .last()
                .ok_or_else(|| TranslationError::Unsupported("empty type path".into()))?;
            let n = seg.ident.to_string();
            Ok(match n.as_str() {
                "i64" | "i32" | "isize" | "u64" | "u32" | "usize" => "integer",
                "f64" | "f32" => "number",
                "bool" => "boolean",
                "String" | "str" => "text",
                _ => return Err(TranslationError::Unsupported(format!("type `{n}`"))),
            }
            .to_string())
        }
        Type::Reference(r) => {
            // &str -> text. &T generally unsupported.
            if let Type::Path(tp) = &*r.elem {
                if let Some(seg) = tp.path.segments.last() {
                    if seg.ident == "str" {
                        return Ok("text".into());
                    }
                }
            }
            Err(TranslationError::Unsupported("reference type".into()))
        }
        _ => Err(TranslationError::Unsupported("complex type".into())),
    }
}

fn block_to_nom(block: &Block, indent: usize) -> Result<String, TranslationError> {
    let pad = "  ".repeat(indent);
    let mut out = String::new();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Local(local) => {
                let name = match &local.pat {
                    Pat::Ident(pi) => pi.ident.to_string(),
                    _ => {
                        return Err(TranslationError::Unsupported(
                            "let with non-ident pattern".into(),
                        ))
                    }
                };
                let rhs = match &local.init {
                    Some(init) => expr_to_nom(&init.expr)?,
                    None => {
                        return Err(TranslationError::Unsupported(
                            "uninitialized let".into(),
                        ))
                    }
                };
                out.push_str(&format!("{pad}let {name} = {rhs}\n"));
            }
            Stmt::Expr(e, semi) => {
                let s = expr_to_nom(e)?;
                if semi.is_some() {
                    out.push_str(&format!("{pad}{s}\n"));
                } else {
                    // Tail expression — implicit return.
                    out.push_str(&format!("{pad}return {s}\n"));
                }
            }
            _ => return Err(TranslationError::Unsupported("stmt kind".into())),
        }
    }
    Ok(out)
}

fn expr_to_nom(expr: &Expr) -> Result<String, TranslationError> {
    match expr {
        Expr::Lit(el) => match &el.lit {
            Lit::Int(i) => Ok(i.base10_digits().to_string()),
            Lit::Float(f) => Ok(f.base10_digits().to_string()),
            Lit::Bool(b) => Ok(b.value().to_string()),
            Lit::Str(s) => Ok(format!("\"{}\"", s.value())),
            _ => Err(TranslationError::Unsupported("literal kind".into())),
        },
        Expr::Path(p) => {
            let name = p
                .path
                .segments
                .last()
                .ok_or_else(|| TranslationError::Unsupported("empty path".into()))?
                .ident
                .to_string();
            Ok(name)
        }
        Expr::Binary(b) => {
            let l = expr_to_nom(&b.left)?;
            let r = expr_to_nom(&b.right)?;
            let op = match b.op {
                BinOp::Add(_) => "+",
                BinOp::Sub(_) => "-",
                BinOp::Mul(_) => "*",
                BinOp::Div(_) => "/",
                _ => return Err(TranslationError::Unsupported("binary op".into())),
            };
            Ok(format!("({l} {op} {r})"))
        }
        Expr::Return(r) => {
            let e = r
                .expr
                .as_ref()
                .ok_or_else(|| TranslationError::Unsupported("bare return".into()))?;
            Ok(format!("return {}", expr_to_nom(e)?))
        }
        Expr::Paren(p) => expr_to_nom(&p.expr),
        Expr::Unary(u) => {
            let e = expr_to_nom(&u.expr)?;
            match u.op {
                UnOp::Neg(_) => Ok(format!("-{e}")),
                _ => Err(TranslationError::Unsupported("unary op".into())),
            }
        }
        _ => Err(TranslationError::Unsupported("expr kind".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_add_fn_translates() {
        let src = "fn add(a: i64, b: i64) -> i64 { a + b }";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "add");
        assert!(items[0].summary.contains("fn add(a: integer, b: integer) -> integer"));
        assert!(items[0].nom_body.contains("return (a + b)"));
    }

    #[test]
    fn fn_with_let_translates() {
        let src = "fn double(x: i64) -> i64 { let y = x * 2; return y; }";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].nom_body.contains("let y = (x * 2)"));
        assert!(items[0].nom_body.contains("return y"));
    }

    #[test]
    fn struct_top_level_skipped_not_error() {
        // struct at top level is now silently skipped (item-level reject).
        let src = "struct Foo { a: i64 }";
        let items = translate(src).unwrap();
        assert!(items.is_empty(), "struct should be skipped, got {items:?}");
    }

    #[test]
    fn generic_fn_skipped_not_error() {
        // Generic fn is now silently skipped (item-level reject).
        let src = "fn foo<T>(x: T) -> T { x }";
        let items = translate(src).unwrap();
        assert!(items.is_empty(), "generic fn should be skipped");
    }

    #[test]
    fn no_params_fn_translates() {
        let src = "fn answer() -> i64 { 42 }";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].summary.contains("fn answer() -> integer"));
        assert!(items[0].nom_body.contains("return 42"));
    }

    #[test]
    fn bool_param_translates() {
        let src = "fn identity(v: bool) -> bool { v }";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].summary.contains("v: boolean") && items[0].summary.contains("-> boolean"));
    }

    #[test]
    fn str_ref_param_translates() {
        let src = "fn len_hint(s: &str) -> i64 { 0 }";
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].summary.contains("s: text"));
    }

    #[test]
    fn async_fn_skipped_not_error() {
        // Async fns are now silently skipped (item-level reject).
        let src = "async fn foo() -> i64 { 1 }";
        let items = translate(src).unwrap();
        assert!(items.is_empty(), "async fn should be skipped");
    }

    #[test]
    fn unknown_type_skipped_not_error() {
        // Fns with unsupported types are silently skipped.
        let src = "fn foo(x: Vec<i64>) -> Vec<i64> { x }";
        let items = translate(src).unwrap();
        assert!(items.is_empty(), "fn with Vec param should be skipped");
    }

    #[test]
    fn multi_item_translate() {
        // File with 2 translatable fns + 1 struct (item-level skipped) → 2 items.
        let src = r#"
            fn add(a: i64, b: i64) -> i64 { a + b }
            struct Point { x: f64 }
            fn sub(a: i64, b: i64) -> i64 { a - b }
        "#;
        let items = translate(src).unwrap();
        assert_eq!(items.len(), 2, "expected 2 items (struct skipped), got {items:?}");
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"add"), "missing 'add'");
        assert!(names.contains(&"sub"), "missing 'sub'");
    }
}
