#[derive(Debug, Clone, PartialEq)]
pub enum RouteKind {
    Static,
    Dynamic,
    Fallback,
    Redirect,
}

impl RouteKind {
    pub fn is_terminal(&self) -> bool {
        matches!(self, RouteKind::Static | RouteKind::Fallback)
    }

    pub fn priority(&self) -> u8 {
        match self {
            RouteKind::Static => 3,
            RouteKind::Dynamic => 2,
            RouteKind::Redirect => 1,
            RouteKind::Fallback => 0,
        }
    }
}

pub struct RouteKey {
    pub path: String,
    pub method: String,
}

impl RouteKey {
    pub fn matches(&self, path: &str, method: &str) -> bool {
        self.path == path && self.method.to_uppercase() == method.to_uppercase()
    }

    pub fn key_string(&self) -> String {
        format!("{}:{}", self.method.to_uppercase(), self.path)
    }
}

pub struct RouteEntry {
    pub key: RouteKey,
    pub kind: RouteKind,
    pub handler_id: u64,
}

impl RouteEntry {
    pub fn is_higher_priority_than(&self, other: &RouteEntry) -> bool {
        self.kind.priority() > other.kind.priority()
    }
}

pub struct RouteTable {
    pub entries: Vec<RouteEntry>,
}

impl RouteTable {
    pub fn new() -> Self {
        RouteTable { entries: Vec::new() }
    }

    pub fn register(&mut self, e: RouteEntry) {
        self.entries.push(e);
    }

    pub fn resolve(&self, path: &str, method: &str) -> Option<&RouteEntry> {
        let mut best: Option<&RouteEntry> = None;
        for entry in &self.entries {
            if entry.key.matches(path, method) {
                match best {
                    None => best = Some(entry),
                    Some(b) => {
                        if entry.kind.priority() > b.kind.priority() {
                            best = Some(entry);
                        }
                    }
                }
            }
        }
        best
    }

    pub fn fallbacks(&self) -> Vec<&RouteEntry> {
        self.entries
            .iter()
            .filter(|e| e.kind == RouteKind::Fallback)
            .collect()
    }
}

impl Default for RouteTable {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RouteResolver {
    pub tables: Vec<RouteTable>,
}

impl RouteResolver {
    pub fn new() -> Self {
        RouteResolver { tables: Vec::new() }
    }

    pub fn add_table(&mut self, t: RouteTable) {
        self.tables.push(t);
    }

    pub fn resolve_all<'a>(&'a self, path: &str, method: &str) -> Vec<&'a RouteEntry> {
        self.tables
            .iter()
            .filter_map(|t| t.resolve(path, method))
            .collect()
    }
}

impl Default for RouteResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_is_terminal() {
        assert!(RouteKind::Static.is_terminal());
        assert!(RouteKind::Fallback.is_terminal());
        assert!(!RouteKind::Dynamic.is_terminal());
        assert!(!RouteKind::Redirect.is_terminal());
    }

    #[test]
    fn kind_priority_static_is_3() {
        assert_eq!(RouteKind::Static.priority(), 3);
    }

    #[test]
    fn key_matches_case_insensitive_method() {
        let key = RouteKey { path: "/home".to_string(), method: "GET".to_string() };
        assert!(key.matches("/home", "get"));
        assert!(key.matches("/home", "GET"));
        assert!(key.matches("/home", "Get"));
        assert!(!key.matches("/home", "POST"));
    }

    #[test]
    fn key_key_string_uppercase() {
        let key = RouteKey { path: "/api/v1".to_string(), method: "post".to_string() };
        assert_eq!(key.key_string(), "POST:/api/v1");
    }

    #[test]
    fn entry_is_higher_priority_than() {
        let a = RouteEntry {
            key: RouteKey { path: "/".to_string(), method: "GET".to_string() },
            kind: RouteKind::Static,
            handler_id: 1,
        };
        let b = RouteEntry {
            key: RouteKey { path: "/".to_string(), method: "GET".to_string() },
            kind: RouteKind::Dynamic,
            handler_id: 2,
        };
        assert!(a.is_higher_priority_than(&b));
        assert!(!b.is_higher_priority_than(&a));
    }

    #[test]
    fn table_register_and_resolve_found() {
        let mut table = RouteTable::new();
        table.register(RouteEntry {
            key: RouteKey { path: "/users".to_string(), method: "GET".to_string() },
            kind: RouteKind::Static,
            handler_id: 42,
        });
        let result = table.resolve("/users", "GET");
        assert!(result.is_some());
        assert_eq!(result.unwrap().handler_id, 42);
    }

    #[test]
    fn table_fallbacks_count() {
        let mut table = RouteTable::new();
        table.register(RouteEntry {
            key: RouteKey { path: "/a".to_string(), method: "GET".to_string() },
            kind: RouteKind::Fallback,
            handler_id: 1,
        });
        table.register(RouteEntry {
            key: RouteKey { path: "/b".to_string(), method: "GET".to_string() },
            kind: RouteKind::Static,
            handler_id: 2,
        });
        table.register(RouteEntry {
            key: RouteKey { path: "/c".to_string(), method: "GET".to_string() },
            kind: RouteKind::Fallback,
            handler_id: 3,
        });
        assert_eq!(table.fallbacks().len(), 2);
    }

    #[test]
    fn resolver_resolve_all_count() {
        let mut t1 = RouteTable::new();
        t1.register(RouteEntry {
            key: RouteKey { path: "/ping".to_string(), method: "GET".to_string() },
            kind: RouteKind::Static,
            handler_id: 10,
        });
        let mut t2 = RouteTable::new();
        t2.register(RouteEntry {
            key: RouteKey { path: "/ping".to_string(), method: "GET".to_string() },
            kind: RouteKind::Dynamic,
            handler_id: 20,
        });
        let mut resolver = RouteResolver::new();
        resolver.add_table(t1);
        resolver.add_table(t2);
        let results = resolver.resolve_all("/ping", "GET");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn key_no_match_returns_none() {
        let mut table = RouteTable::new();
        table.register(RouteEntry {
            key: RouteKey { path: "/only".to_string(), method: "GET".to_string() },
            kind: RouteKind::Static,
            handler_id: 99,
        });
        assert!(table.resolve("/missing", "GET").is_none());
        assert!(table.resolve("/only", "DELETE").is_none());
    }
}
