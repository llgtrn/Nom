#![deny(unsafe_code)]

/// A skill that can be looked up and invoked by the intent layer.
///
/// Pattern derived from skill-package invocation engines: each skill carries
/// a stable ID, a human name, a description used for fuzzy lookup, and JSON
/// Schema strings that describe what the skill accepts and returns.
#[derive(Debug, Clone)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    /// JSON Schema string describing the expected input payload.
    pub input_schema: String,
    /// JSON Schema string describing the produced output payload.
    pub output_schema: String,
}

/// Registry of available skills, supporting registration and lookup.
pub struct SkillRouter {
    skills: Vec<SkillDefinition>,
}

impl SkillRouter {
    pub fn new() -> Self {
        Self { skills: vec![] }
    }

    /// Register a skill. Duplicate IDs are allowed (last write wins via
    /// `find_by_id` returning the first match).
    pub fn register(&mut self, skill: SkillDefinition) {
        self.skills.push(skill);
    }

    /// Find a skill by exact ID, returning the first registered match.
    pub fn find_by_id(&self, id: &str) -> Option<&SkillDefinition> {
        self.skills.iter().find(|s| s.id == id)
    }

    /// Find skills whose name or description contains `query` (case-insensitive).
    pub fn find_by_query(&self, query: &str) -> Vec<&SkillDefinition> {
        let q = query.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q) || s.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// True when the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(id: &str, name: &str, description: &str) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            input_schema: r#"{"type":"object"}"#.to_string(),
            output_schema: r#"{"type":"string"}"#.to_string(),
        }
    }

    #[test]
    fn skill_router_register_and_find_by_id() {
        let mut router = SkillRouter::new();
        router.register(make_skill(
            "compose.text",
            "Compose Text",
            "generates prose",
        ));
        router.register(make_skill(
            "analyze.code",
            "Analyze Code",
            "inspects source",
        ));

        let found = router.find_by_id("compose.text");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Compose Text");

        let missing = router.find_by_id("no.such.skill");
        assert!(missing.is_none());
    }

    #[test]
    fn skill_router_find_by_query_matches_name_and_description() {
        let mut router = SkillRouter::new();
        router.register(make_skill(
            "s1",
            "Build Graph",
            "constructs a dependency graph",
        ));
        router.register(make_skill("s2", "Render Canvas", "draws nodes onto canvas"));
        router.register(make_skill("s3", "Export Image", "saves canvas as an image"));

        // matches description of s1 and name of s2/s3 via "canvas" or "graph"
        let results = router.find_by_query("canvas");
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"s2"));
        assert!(ids.contains(&"s3"));
    }

    #[test]
    fn skill_router_len_tracks_registrations() {
        let mut router = SkillRouter::new();
        assert_eq!(router.len(), 0);
        assert!(router.is_empty());

        router.register(make_skill("a", "Alpha", "first"));
        router.register(make_skill("b", "Beta", "second"));
        router.register(make_skill("c", "Gamma", "third"));

        assert_eq!(router.len(), 3);
        assert!(!router.is_empty());
    }
}
