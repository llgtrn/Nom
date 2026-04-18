#![deny(unsafe_code)]

/// A skill entry that can be matched against a query by its trigger pattern.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub trigger_pattern: String,
}

/// Routes queries to matching skill entries by trigger-pattern containment.
pub struct SkillRouter {
    entries: Vec<SkillEntry>,
}

impl SkillRouter {
    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    /// Builder-style registration of a skill entry.
    pub fn register(mut self, entry: SkillEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Returns the first entry whose trigger_pattern is contained in `query`
    /// (case-insensitive).
    pub fn route(&self, query: &str) -> Option<&SkillEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .find(|e| q.contains(&e.trigger_pattern.to_lowercase()))
    }

    /// Returns all entries whose trigger_pattern is contained in `query`
    /// (case-insensitive).
    pub fn route_all(&self, query: &str) -> Vec<&SkillEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| q.contains(&e.trigger_pattern.to_lowercase()))
            .collect()
    }

    /// Number of registered entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for SkillRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Dispatches queries through a `SkillRouter` and returns skill names.
pub struct SkillDispatch {
    router: SkillRouter,
}

impl SkillDispatch {
    pub fn new(router: SkillRouter) -> Self {
        Self { router }
    }

    /// Returns the name of the first matching skill, or `None`.
    pub fn dispatch(&self, query: &str) -> Option<String> {
        self.router.route(query).map(|e| e.name.clone())
    }

    /// Returns names of all matching skills.
    pub fn dispatch_all(&self, query: &str) -> Vec<String> {
        self.router
            .route_all(query)
            .into_iter()
            .map(|e| e.name.clone())
            .collect()
    }
}

#[cfg(test)]
mod skill_route_tests {
    use super::*;

    fn make_entry(name: &str, description: &str, trigger: &str) -> SkillEntry {
        SkillEntry {
            name: name.to_string(),
            description: description.to_string(),
            trigger_pattern: trigger.to_string(),
        }
    }

    // Test 1: SkillEntry fields are accessible and correct.
    #[test]
    fn test_skill_entry_fields() {
        let entry = make_entry("brainstorm", "Brainstorming skill", "brainstorm");
        assert_eq!(entry.name, "brainstorm");
        assert_eq!(entry.description, "Brainstorming skill");
        assert_eq!(entry.trigger_pattern, "brainstorm");
    }

    // Test 2: SkillRouter::new() starts empty.
    #[test]
    fn test_skill_router_new_empty() {
        let router = SkillRouter::new();
        assert_eq!(router.count(), 0);
    }

    // Test 3: register() adds an entry.
    #[test]
    fn test_register_adds_entry() {
        let router = SkillRouter::new().register(make_entry("debug", "Debugger", "debug"));
        assert_eq!(router.count(), 1);
    }

    // Test 4: route() returns None when router is empty.
    #[test]
    fn test_route_returns_none_for_empty() {
        let router = SkillRouter::new();
        assert!(router.route("anything").is_none());
    }

    // Test 5: route() matches trigger_pattern case-insensitively.
    #[test]
    fn test_route_matches_case_insensitive() {
        let router = SkillRouter::new().register(make_entry("plan", "Planning skill", "PLAN"));
        let result = router.route("I want to plan something");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "plan");
    }

    // Test 6: route() returns None when no entry matches.
    #[test]
    fn test_route_returns_none_no_match() {
        let router = SkillRouter::new().register(make_entry("debug", "Debugger", "debug"));
        assert!(router.route("run the build").is_none());
    }

    // Test 7: route_all() returns multiple matches.
    #[test]
    fn test_route_all_returns_multiple() {
        let router = SkillRouter::new()
            .register(make_entry("debug-basic", "Basic debugger", "debug"))
            .register(make_entry("debug-advanced", "Advanced debugger", "debug"))
            .register(make_entry("plan", "Planner", "plan"));
        let matches = router.route_all("please debug the issue");
        assert_eq!(matches.len(), 2);
        let names: Vec<&str> = matches.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"debug-basic"));
        assert!(names.contains(&"debug-advanced"));
    }

    // Test 8: SkillDispatch::dispatch() returns the skill name.
    #[test]
    fn test_dispatch_returns_skill_name() {
        let router = SkillRouter::new().register(make_entry("tdd", "TDD skill", "test-driven"));
        let dispatch = SkillDispatch::new(router);
        assert_eq!(
            dispatch.dispatch("use test-driven development"),
            Some("tdd".to_string())
        );
    }

    // Test 9: SkillDispatch::dispatch_all() returns a vec of names.
    #[test]
    fn test_dispatch_all_returns_vec() {
        let router = SkillRouter::new()
            .register(make_entry("review-a", "Review A", "review"))
            .register(make_entry("review-b", "Review B", "review"));
        let dispatch = SkillDispatch::new(router);
        let names = dispatch.dispatch_all("please review the code");
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"review-a".to_string()));
        assert!(names.contains(&"review-b".to_string()));
    }
}
