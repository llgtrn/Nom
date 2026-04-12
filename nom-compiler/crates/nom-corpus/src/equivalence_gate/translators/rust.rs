use super::TranslationError;
use syn::{
    parse_file, Block, BinOp, Expr, FnArg, Item, ItemFn, Lit, Pat, PathSegment, ReturnType, Stmt,
    Type, UnOp,
};

/// Translate a Rust source string to a Nom source string.
/// Narrow subset only — returns Unsupported for anything outside the
/// supported set of free-standing fns with scalar params and simple bodies.
pub fn translate(source: &str) -> Result<String, TranslationError> {
    let file = parse_file(source).map_err(|e| TranslationError::Parse(e.to_string()))?;
    let mut out = String::new();
    for item in file.items {
        match item {
            Item::Fn(ItemFn {
                sig,
                block,
                vis: _,
                attrs: _,
                ..
            }) => {
                if !sig.generics.params.is_empty() {
                    return Err(TranslationError::Unsupported("generics".into()));
                }
                if sig.asyncness.is_some() {
                    return Err(TranslationError::Unsupported("async fn".into()));
                }
                let name = sig.ident.to_string();
                let mut params: Vec<String> = Vec::new();
                for arg in &sig.inputs {
                    match arg {
                        FnArg::Receiver(_) => {
                            return Err(TranslationError::Unsupported(
                                "method (self arg)".into(),
                            ))
                        }
                        FnArg::Typed(pt) => {
                            let arg_name = match &*pt.pat {
                                Pat::Ident(pi) => pi.ident.to_string(),
                                _ => {
                                    return Err(TranslationError::Unsupported(
                                        "non-ident pattern in fn arg".into(),
                                    ))
                                }
                            };
                            let ty = type_to_nom(&pt.ty)?;
                            params.push(format!("{arg_name}: {ty}"));
                        }
                    }
                }
                let ret = match &sig.output {
                    ReturnType::Default => "unit".to_string(),
                    ReturnType::Type(_, ty) => type_to_nom(ty)?,
                };
                out.push_str(&format!(
                    "fn {name}({}) -> {ret} {{\n",
                    params.join(", ")
                ));
                out.push_str(&block_to_nom(&block, 1)?);
                out.push_str("}\n\n");
            }
            other => {
                return Err(TranslationError::Unsupported(format!(
                    "top-level item: {}",
                    match other {
                        Item::Struct(_) => "struct",
                        Item::Enum(_) => "enum",
                        Item::Impl(_) => "impl block",
                        Item::Trait(_) => "trait",
                        Item::Use(_) => "use statement",
                        Item::Mod(_) => "mod",
                        Item::Static(_) => "static",
                        Item::Const(_) => "const",
                        Item::Type(_) => "type alias",
                        _ => "other",
                    }
                )));
            }
        }
    }
    Ok(out)
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
        let out = translate(src).unwrap();
        assert!(out.contains("fn add(a: integer, b: integer) -> integer"));
        assert!(out.contains("return (a + b)"));
    }

    #[test]
    fn fn_with_let_translates() {
        let src = "fn double(x: i64) -> i64 { let y = x * 2; return y; }";
        let out = translate(src).unwrap();
        assert!(out.contains("let y = (x * 2)"));
        assert!(out.contains("return y"));
    }

    #[test]
    fn struct_top_level_rejected() {
        let src = "struct Foo { a: i64 }";
        let err = translate(src).unwrap_err();
        match err {
            TranslationError::Unsupported(r) => assert!(r.contains("struct")),
            _ => panic!("expected Unsupported"),
        }
    }

    #[test]
    fn generic_fn_rejected() {
        let src = "fn foo<T>(x: T) -> T { x }";
        let err = translate(src).unwrap_err();
        match err {
            TranslationError::Unsupported(r) => assert!(r.contains("generics")),
            _ => panic!("expected Unsupported"),
        }
    }

    #[test]
    fn no_params_fn_translates() {
        let src = "fn answer() -> i64 { 42 }";
        let out = translate(src).unwrap();
        assert!(out.contains("fn answer() -> integer"));
        assert!(out.contains("return 42"));
    }

    #[test]
    fn bool_param_translates() {
        let src = "fn identity(v: bool) -> bool { v }";
        let out = translate(src).unwrap();
        assert!(out.contains("v: boolean") && out.contains("-> boolean"));
    }

    #[test]
    fn str_ref_param_translates() {
        let src = "fn len_hint(s: &str) -> i64 { 0 }";
        let out = translate(src).unwrap();
        assert!(out.contains("s: text"));
    }

    #[test]
    fn async_fn_rejected() {
        let src = "async fn foo() -> i64 { 1 }";
        let err = translate(src).unwrap_err();
        match err {
            TranslationError::Unsupported(r) => assert!(r.contains("async fn")),
            _ => panic!("expected Unsupported"),
        }
    }

    #[test]
    fn unknown_type_rejected() {
        let src = "fn foo(x: Vec<i64>) -> Vec<i64> { x }";
        let err = translate(src).unwrap_err();
        match err {
            TranslationError::Unsupported(_) => {}
            _ => panic!("expected Unsupported"),
        }
    }
}
