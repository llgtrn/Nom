//! AST-level sandbox for user-supplied `.nom` scripts executed inside graph nodes.
//!
//! This is a DATA-STRUCTURE transform layer — the actual wasm/v8 isolation
//! lives in a separate runtime crate.  What lives here:
//!   - `SandboxConfig`: limits (memory, timeout, denylist)
//!   - `AstNode` placeholder + `visit` helper
//!   - 4 sanitizers: this_replace, prototype_block, dollar_validate, allowlist
//!   - `SanitizeError` union
#![deny(unsafe_code)]

#[derive(Clone, Debug, PartialEq)]
pub struct SandboxConfig {
    pub memory_bytes_limit: usize,
    pub timeout_ms: u64,
    pub allowed_globals: Vec<&'static str>,
    pub blocked_globals: Vec<&'static str>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_bytes_limit: 128 * 1024 * 1024,
            timeout_ms: 5000,
            allowed_globals: vec!["DateTime", "Duration", "Interval"],
            blocked_globals: vec!["process", "require", "module", "Buffer", "global", "globalThis"],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AstNode {
    Identifier(String),
    MemberAccess { receiver: Box<AstNode>, field: String },
    Call { callee: Box<AstNode>, args: Vec<AstNode> },
    This,
    Literal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SanitizeError {
    #[error("bare identifier `$` must be a call or property access")]
    BareDollarIdentifier,
    #[error("blocked global `{0}`")]
    BlockedGlobal(String),
    #[error("prototype-pollution method `{0}` is not allowed")]
    BlockedPrototypeMethod(String),
    #[error("user-facing scripts may not access `Error.prepareStackTrace`")]
    PrepareStackTraceWrite,
}

/// 1. Replace `this` with the `EMPTY_CONTEXT` sentinel.
pub fn this_replace(node: AstNode) -> AstNode {
    match node {
        AstNode::This => AstNode::Identifier("EMPTY_CONTEXT".to_string()),
        AstNode::MemberAccess { receiver, field } => AstNode::MemberAccess {
            receiver: Box::new(this_replace(*receiver)),
            field,
        },
        AstNode::Call { callee, args } => AstNode::Call {
            callee: Box::new(this_replace(*callee)),
            args: args.into_iter().map(this_replace).collect(),
        },
        other => other,
    }
}

/// 2. Reject prototype-pollution method names.
pub fn prototype_block(node: &AstNode) -> Result<(), SanitizeError> {
    let blocked = [
        "defineProperty", "defineProperties", "setPrototypeOf", "getPrototypeOf",
        "getOwnPropertyDescriptor", "getOwnPropertyDescriptors",
        "__defineGetter__", "__defineSetter__",
        "__lookupGetter__", "__lookupSetter__",
    ];
    fn walk(n: &AstNode, blocked: &[&str]) -> Result<(), SanitizeError> {
        if let AstNode::MemberAccess { field, receiver } = n {
            if blocked.contains(&field.as_str()) {
                return Err(SanitizeError::BlockedPrototypeMethod(field.clone()));
            }
            // The V8 RCE vector.
            if matches!(receiver.as_ref(), AstNode::Identifier(id) if id == "Error") && field == "prepareStackTrace" {
                return Err(SanitizeError::PrepareStackTraceWrite);
            }
            walk(receiver, blocked)?;
        }
        if let AstNode::Call { callee, args } = n {
            walk(callee, blocked)?;
            for a in args { walk(a, blocked)?; }
        }
        Ok(())
    }
    walk(node, &blocked)
}

/// 3. `$` must be used as a function call or property access, never bare.
pub fn dollar_validate(node: &AstNode) -> Result<(), SanitizeError> {
    // A bare `$` appears as `AstNode::Identifier("$")` at a position that is
    // NOT the callee of a Call and NOT the receiver of a MemberAccess.
    // We mark the context when recursing and flag any identifier "$" that
    // doesn't satisfy either shape.
    fn walk(n: &AstNode, context_is_callee_or_receiver: bool) -> Result<(), SanitizeError> {
        match n {
            AstNode::Identifier(id) if id == "$" && !context_is_callee_or_receiver => {
                Err(SanitizeError::BareDollarIdentifier)
            }
            AstNode::MemberAccess { receiver, .. } => walk(receiver, true),
            AstNode::Call { callee, args } => {
                walk(callee, true)?;
                for a in args { walk(a, false)?; }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    walk(node, false)
}

/// 4. Block identifiers listed in the denylist.
pub fn allowlist(node: &AstNode, config: &SandboxConfig) -> Result<(), SanitizeError> {
    fn walk(n: &AstNode, config: &SandboxConfig) -> Result<(), SanitizeError> {
        if let AstNode::Identifier(id) = n {
            if config.blocked_globals.iter().any(|b| *b == id) {
                return Err(SanitizeError::BlockedGlobal(id.clone()));
            }
        }
        if let AstNode::MemberAccess { receiver, .. } = n { walk(receiver, config)?; }
        if let AstNode::Call { callee, args } = n {
            walk(callee, config)?;
            for a in args { walk(a, config)?; }
        }
        Ok(())
    }
    walk(node, config)
}

/// Run all 4 sanitizers in order.  Returns the transformed node (from
/// `this_replace`) on success, or the first error encountered.
pub fn sanitize(node: AstNode, config: &SandboxConfig) -> Result<AstNode, SanitizeError> {
    let transformed = this_replace(node);
    prototype_block(&transformed)?;
    dollar_validate(&transformed)?;
    allowlist(&transformed, config)?;
    Ok(transformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SandboxConfig {
        SandboxConfig::default()
    }

    // --- SandboxConfig defaults ---

    #[test]
    fn default_memory_limit_is_128mb() {
        assert_eq!(cfg().memory_bytes_limit, 128 * 1024 * 1024);
    }

    #[test]
    fn default_timeout_is_5000ms() {
        assert_eq!(cfg().timeout_ms, 5000);
    }

    #[test]
    fn default_blocked_globals_contains_process_and_require() {
        let c = cfg();
        assert!(c.blocked_globals.contains(&"process"));
        assert!(c.blocked_globals.contains(&"require"));
        assert!(c.blocked_globals.contains(&"module"));
        assert!(c.blocked_globals.contains(&"Buffer"));
        assert!(c.blocked_globals.contains(&"global"));
        assert!(c.blocked_globals.contains(&"globalThis"));
    }

    // --- this_replace ---

    #[test]
    fn this_replace_bare_this_becomes_empty_context() {
        let result = this_replace(AstNode::This);
        assert_eq!(result, AstNode::Identifier("EMPTY_CONTEXT".to_string()));
    }

    #[test]
    fn this_replace_nested_recurses_through_member_and_call() {
        // Call(callee=MemberAccess(receiver=This, field="foo"), args=[])
        let node = AstNode::Call {
            callee: Box::new(AstNode::MemberAccess {
                receiver: Box::new(AstNode::This),
                field: "foo".to_string(),
            }),
            args: vec![],
        };
        let result = this_replace(node);
        let expected = AstNode::Call {
            callee: Box::new(AstNode::MemberAccess {
                receiver: Box::new(AstNode::Identifier("EMPTY_CONTEXT".to_string())),
                field: "foo".to_string(),
            }),
            args: vec![],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn this_replace_leaves_non_this_nodes_intact() {
        let node = AstNode::Identifier("x".to_string());
        assert_eq!(this_replace(node.clone()), node);
    }

    // --- prototype_block ---

    #[test]
    fn prototype_block_flags_define_property() {
        let node = AstNode::MemberAccess {
            receiver: Box::new(AstNode::Identifier("obj".to_string())),
            field: "defineProperty".to_string(),
        };
        let err = prototype_block(&node).unwrap_err();
        assert!(matches!(err, SanitizeError::BlockedPrototypeMethod(m) if m == "defineProperty"));
    }

    #[test]
    fn prototype_block_flags_error_prepare_stack_trace() {
        let node = AstNode::MemberAccess {
            receiver: Box::new(AstNode::Identifier("Error".to_string())),
            field: "prepareStackTrace".to_string(),
        };
        let err = prototype_block(&node).unwrap_err();
        assert!(matches!(err, SanitizeError::PrepareStackTraceWrite));
    }

    #[test]
    fn prototype_block_passes_allowed_method() {
        let node = AstNode::MemberAccess {
            receiver: Box::new(AstNode::Identifier("obj".to_string())),
            field: "save".to_string(),
        };
        assert!(prototype_block(&node).is_ok());
    }

    // --- dollar_validate ---

    #[test]
    fn dollar_validate_passes_dollar_as_member_receiver() {
        // $.foo  →  MemberAccess(receiver=Identifier("$"), field="foo")
        let node = AstNode::MemberAccess {
            receiver: Box::new(AstNode::Identifier("$".to_string())),
            field: "foo".to_string(),
        };
        assert!(dollar_validate(&node).is_ok());
    }

    #[test]
    fn dollar_validate_passes_dollar_as_call_callee() {
        // $()  →  Call(callee=Identifier("$"), args=[])
        let node = AstNode::Call {
            callee: Box::new(AstNode::Identifier("$".to_string())),
            args: vec![],
        };
        assert!(dollar_validate(&node).is_ok());
    }

    #[test]
    fn dollar_validate_rejects_bare_dollar_in_args() {
        // foo($)  →  Call(callee=Identifier("foo"), args=[Identifier("$")])
        let node = AstNode::Call {
            callee: Box::new(AstNode::Identifier("foo".to_string())),
            args: vec![AstNode::Identifier("$".to_string())],
        };
        let err = dollar_validate(&node).unwrap_err();
        assert!(matches!(err, SanitizeError::BareDollarIdentifier));
    }

    // --- allowlist ---

    #[test]
    fn allowlist_rejects_process() {
        let node = AstNode::Identifier("process".to_string());
        let err = allowlist(&node, &cfg()).unwrap_err();
        assert!(matches!(err, SanitizeError::BlockedGlobal(g) if g == "process"));
    }

    #[test]
    fn allowlist_passes_datetime() {
        let node = AstNode::Identifier("DateTime".to_string());
        assert!(allowlist(&node, &cfg()).is_ok());
    }

    // --- sanitize integration ---

    #[test]
    fn sanitize_prototype_block_wins_over_allowlist_when_both_trigger() {
        // Node has both a blocked prototype method and a blocked global identifier.
        // prototype_block runs before allowlist, so its error is returned first.
        let node = AstNode::Call {
            callee: Box::new(AstNode::MemberAccess {
                receiver: Box::new(AstNode::Identifier("process".to_string())),
                field: "defineProperty".to_string(),
            }),
            args: vec![],
        };
        let err = sanitize(node, &cfg()).unwrap_err();
        assert!(matches!(err, SanitizeError::BlockedPrototypeMethod(_)));
    }

    #[test]
    fn sanitize_replaces_this_and_returns_transformed_on_success() {
        let node = AstNode::MemberAccess {
            receiver: Box::new(AstNode::This),
            field: "value".to_string(),
        };
        let result = sanitize(node, &cfg()).unwrap();
        let expected = AstNode::MemberAccess {
            receiver: Box::new(AstNode::Identifier("EMPTY_CONTEXT".to_string())),
            field: "value".to_string(),
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn sanitize_clean_literal_passes_through() {
        let node = AstNode::Literal("hello".to_string());
        let result = sanitize(node.clone(), &cfg()).unwrap();
        assert_eq!(result, node);
    }
}
