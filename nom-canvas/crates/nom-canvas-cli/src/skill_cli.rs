/// CLI dispatch support for `nom skill route`.

/// Arguments for the skill route command.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillCliArgs {
    pub query: String,
    pub list_all: bool,
}

impl SkillCliArgs {
    pub fn new(query: impl Into<String>, list_all: bool) -> Self {
        Self {
            query: query.into(),
            list_all,
        }
    }
}

/// Result returned after routing a skill query.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillCliResult {
    pub matched_skills: Vec<String>,
    pub query: String,
}

impl SkillCliResult {
    pub fn new(query: impl Into<String>, matched: Vec<String>) -> Self {
        Self {
            query: query.into(),
            matched_skills: matched,
        }
    }

    /// Returns true when at least one skill matched.
    pub fn found(&self) -> bool {
        !self.matched_skills.is_empty()
    }

    /// Human-readable summary of the result.
    pub fn format_output(&self) -> String {
        if self.found() {
            format!("Found: {:?}", self.matched_skills)
        } else {
            format!("No skills matched '{}'", self.query)
        }
    }
}

/// Runs skill routing queries against a catalogue of trigger-pattern → skill-name pairs.
pub struct SkillCliRunner {
    /// Each entry is (trigger_pattern, skill_name).
    pub skills: Vec<(String, String)>,
}

impl SkillCliRunner {
    /// Create an empty runner.
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// Create a runner pre-seeded with the default skill catalogue.
    pub fn seed_skills() -> Self {
        let entries: &[(&str, &str)] = &[
            ("brainstorm", "superpowers:brainstorming"),
            ("debug", "superpowers:systematic-debugging"),
            ("test", "superpowers:test-driven-development"),
            ("plan", "superpowers:writing-plans"),
            ("review", "superpowers:requesting-code-review"),
            ("verify", "superpowers:verification-before-completion"),
            ("dream", "nom:dream"),
            ("route", "nom:skill-route"),
        ];
        Self {
            skills: entries
                .iter()
                .map(|(t, s)| (t.to_string(), s.to_string()))
                .collect(),
        }
    }

    /// Find all skills whose trigger pattern is contained in the query (case-insensitive).
    pub fn run(&self, args: &SkillCliArgs) -> SkillCliResult {
        let query_lower = args.query.to_lowercase();
        let matched: Vec<String> = self
            .skills
            .iter()
            .filter(|(trigger, _)| query_lower.contains(trigger.as_str()))
            .map(|(_, skill)| skill.clone())
            .collect();
        SkillCliResult::new(args.query.clone(), matched)
    }

    /// Return all skill names in the catalogue.
    pub fn list_all(&self) -> Vec<String> {
        self.skills.iter().map(|(_, s)| s.clone()).collect()
    }
}

impl Default for SkillCliRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod skill_cli_tests {
    use super::*;

    #[test]
    fn test_args_fields() {
        let args = SkillCliArgs::new("debug my code", false);
        assert_eq!(args.query, "debug my code");
        assert!(!args.list_all);

        let args2 = SkillCliArgs::new("list", true);
        assert!(args2.list_all);
    }

    #[test]
    fn test_result_found_true() {
        let result = SkillCliResult::new("debug", vec!["superpowers:systematic-debugging".into()]);
        assert!(result.found());
    }

    #[test]
    fn test_result_found_false() {
        let result = SkillCliResult::new("unknown", vec![]);
        assert!(!result.found());
    }

    #[test]
    fn test_format_output_when_found() {
        let result = SkillCliResult::new("test", vec!["superpowers:test-driven-development".into()]);
        let output = result.format_output();
        assert!(output.starts_with("Found:"), "output was: {output}");
        assert!(output.contains("superpowers:test-driven-development"));
    }

    #[test]
    fn test_format_output_when_not_found() {
        let result = SkillCliResult::new("xyz", vec![]);
        assert_eq!(result.format_output(), "No skills matched 'xyz'");
    }

    #[test]
    fn test_seed_skills_count() {
        let runner = SkillCliRunner::seed_skills();
        assert!(runner.skills.len() >= 8, "expected >= 8 seeds, got {}", runner.skills.len());
    }

    #[test]
    fn test_run_matches_single_skill() {
        let runner = SkillCliRunner::seed_skills();
        let args = SkillCliArgs::new("brainstorm a feature", false);
        let result = runner.run(&args);
        assert!(result.found());
        assert!(result.matched_skills.contains(&"superpowers:brainstorming".to_string()));
    }

    #[test]
    fn test_run_matches_no_skill() {
        let runner = SkillCliRunner::seed_skills();
        let args = SkillCliArgs::new("something completely unrelated", false);
        let result = runner.run(&args);
        assert!(!result.found());
    }

    #[test]
    fn test_run_case_insensitive() {
        let runner = SkillCliRunner::seed_skills();
        let args = SkillCliArgs::new("DEBUG the failing test", false);
        let result = runner.run(&args);
        assert!(result.found());
        assert!(result.matched_skills.contains(&"superpowers:systematic-debugging".to_string()));
    }

    #[test]
    fn test_list_all_returns_all_names() {
        let runner = SkillCliRunner::seed_skills();
        let all = runner.list_all();
        assert_eq!(all.len(), runner.skills.len());
        assert!(all.contains(&"superpowers:brainstorming".to_string()));
        assert!(all.contains(&"nom:dream".to_string()));
        assert!(all.contains(&"nom:skill-route".to_string()));
    }
}
