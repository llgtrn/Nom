#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
    Function,
    Type,
    Variable,
    Module,
    Macro,
}

impl DefinitionKind {
    pub fn is_callable(&self) -> bool {
        matches!(self, DefinitionKind::Function | DefinitionKind::Macro)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DefinitionKind::Function => "function",
            DefinitionKind::Type => "type",
            DefinitionKind::Variable => "variable",
            DefinitionKind::Module => "module",
            DefinitionKind::Macro => "macro",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DefinitionLocation {
    pub file_path: String,
    pub byte_offset: usize,
    pub line: u32,
    pub column: u32,
}

impl DefinitionLocation {
    pub fn location_string(&self) -> String {
        format!("{}:{}:{}", self.file_path, self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub struct DefinitionTarget {
    pub name: String,
    pub kind: DefinitionKind,
    pub location: DefinitionLocation,
}

impl DefinitionTarget {
    pub fn is_in_file(&self, path: &str) -> bool {
        self.location.file_path == path
    }
}

#[derive(Debug, Clone)]
pub struct GoToDefRequest {
    pub symbol_name: String,
    pub cursor_byte: usize,
    pub source_file: String,
}

impl GoToDefRequest {
    pub fn matches_symbol(&self, name: &str) -> bool {
        self.symbol_name == name
    }
}

#[derive(Debug, Default)]
pub struct GoToDefResolver {
    pub definitions: Vec<DefinitionTarget>,
}

impl GoToDefResolver {
    pub fn register(&mut self, d: DefinitionTarget) {
        self.definitions.push(d);
    }

    pub fn resolve(&self, req: &GoToDefRequest) -> Option<&DefinitionTarget> {
        self.definitions.iter().find(|d| d.name == req.symbol_name)
    }

    pub fn resolve_all(&self, name: &str) -> Vec<&DefinitionTarget> {
        self.definitions.iter().filter(|d| d.name == name).collect()
    }
}

#[cfg(test)]
mod go_to_def_tests {
    use super::*;

    fn make_location(file: &str) -> DefinitionLocation {
        DefinitionLocation {
            file_path: file.to_string(),
            byte_offset: 0,
            line: 10,
            column: 4,
        }
    }

    #[test]
    fn kind_is_callable_function_true() {
        assert!(DefinitionKind::Function.is_callable());
    }

    #[test]
    fn kind_is_callable_variable_false() {
        assert!(!DefinitionKind::Variable.is_callable());
    }

    #[test]
    fn kind_display_name() {
        assert_eq!(DefinitionKind::Function.display_name(), "function");
        assert_eq!(DefinitionKind::Type.display_name(), "type");
        assert_eq!(DefinitionKind::Variable.display_name(), "variable");
        assert_eq!(DefinitionKind::Module.display_name(), "module");
        assert_eq!(DefinitionKind::Macro.display_name(), "macro");
    }

    #[test]
    fn location_string_format() {
        let loc = DefinitionLocation {
            file_path: "src/main.rs".to_string(),
            byte_offset: 42,
            line: 7,
            column: 3,
        };
        assert_eq!(loc.location_string(), "src/main.rs:7:3");
    }

    #[test]
    fn target_is_in_file_true() {
        let target = DefinitionTarget {
            name: "foo".to_string(),
            kind: DefinitionKind::Function,
            location: make_location("src/lib.rs"),
        };
        assert!(target.is_in_file("src/lib.rs"));
        assert!(!target.is_in_file("src/main.rs"));
    }

    #[test]
    fn request_matches_symbol_true() {
        let req = GoToDefRequest {
            symbol_name: "my_fn".to_string(),
            cursor_byte: 100,
            source_file: "src/lib.rs".to_string(),
        };
        assert!(req.matches_symbol("my_fn"));
    }

    #[test]
    fn request_matches_symbol_false() {
        let req = GoToDefRequest {
            symbol_name: "my_fn".to_string(),
            cursor_byte: 100,
            source_file: "src/lib.rs".to_string(),
        };
        assert!(!req.matches_symbol("other_fn"));
    }

    #[test]
    fn resolver_resolve_found() {
        let mut resolver = GoToDefResolver::default();
        resolver.register(DefinitionTarget {
            name: "foo".to_string(),
            kind: DefinitionKind::Function,
            location: make_location("src/lib.rs"),
        });
        let req = GoToDefRequest {
            symbol_name: "foo".to_string(),
            cursor_byte: 0,
            source_file: "src/lib.rs".to_string(),
        };
        let result = resolver.resolve(&req);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "foo");
    }

    #[test]
    fn resolver_resolve_not_found() {
        let resolver = GoToDefResolver::default();
        let req = GoToDefRequest {
            symbol_name: "missing".to_string(),
            cursor_byte: 0,
            source_file: "src/lib.rs".to_string(),
        };
        assert!(resolver.resolve(&req).is_none());
    }

    #[test]
    fn resolver_resolve_all_count() {
        let mut resolver = GoToDefResolver::default();
        resolver.register(DefinitionTarget {
            name: "bar".to_string(),
            kind: DefinitionKind::Function,
            location: make_location("src/a.rs"),
        });
        resolver.register(DefinitionTarget {
            name: "bar".to_string(),
            kind: DefinitionKind::Function,
            location: make_location("src/b.rs"),
        });
        resolver.register(DefinitionTarget {
            name: "baz".to_string(),
            kind: DefinitionKind::Variable,
            location: make_location("src/c.rs"),
        });
        let results = resolver.resolve_all("bar");
        assert_eq!(results.len(), 2);
    }
}
