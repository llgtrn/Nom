/// Category of a lint skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillCategory {
    Formatting,
    Naming,
    Complexity,
    Performance,
    Style,
}

impl SkillCategory {
    /// Short lowercase label for this category.
    pub fn label(&self) -> &'static str {
        match self {
            SkillCategory::Formatting => "formatting",
            SkillCategory::Naming => "naming",
            SkillCategory::Complexity => "complexity",
            SkillCategory::Performance => "performance",
            SkillCategory::Style => "style",
        }
    }

    /// Returns `true` for categories that can be applied automatically.
    pub fn is_automated(&self) -> bool {
        matches!(
            self,
            SkillCategory::Formatting | SkillCategory::Naming | SkillCategory::Style
        )
    }
}

/// Proficiency level of a lint skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Advanced,
}

impl SkillLevel {
    /// Numeric score for this level (1–3).
    pub fn score(&self) -> u8 {
        match self {
            SkillLevel::Beginner => 1,
            SkillLevel::Intermediate => 2,
            SkillLevel::Advanced => 3,
        }
    }

    /// Returns `true` when `self` can teach `other` (self has a higher score).
    pub fn can_teach(&self, other: &SkillLevel) -> bool {
        self.score() > other.score()
    }
}

/// A single skill entry in the skill map.
pub struct SkillEntry {
    pub id: u32,
    pub name: String,
    pub category: SkillCategory,
    pub level: SkillLevel,
}

impl SkillEntry {
    /// Formatted display string: `"[<category>] <name> (<score>)"`.
    pub fn display(&self) -> String {
        format!(
            "[{}] {} ({})",
            self.category.label(),
            self.name,
            self.level.score()
        )
    }

    /// Returns `true` when the entry's level is `Advanced`.
    pub fn is_advanced(&self) -> bool {
        self.level == SkillLevel::Advanced
    }
}

/// Collection of skill entries.
pub struct SkillMap {
    pub entries: Vec<SkillEntry>,
}

impl SkillMap {
    /// Creates an empty skill map.
    pub fn new() -> Self {
        SkillMap {
            entries: Vec::new(),
        }
    }

    /// Appends an entry to the map.
    pub fn add(&mut self, entry: SkillEntry) {
        self.entries.push(entry);
    }

    /// Returns all entries whose category label matches `cat`.
    pub fn by_category(&self, cat: &SkillCategory) -> Vec<&SkillEntry> {
        self.entries
            .iter()
            .filter(|e| e.category.label() == cat.label())
            .collect()
    }

    /// Counts entries whose category is automated.
    pub fn automated_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.category.is_automated())
            .count()
    }
}

impl Default for SkillMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides recommendation queries over a `SkillMap`.
pub struct SkillRecommender;

impl SkillRecommender {
    /// Returns all entries whose level score is at or below `level`'s score.
    pub fn recommend_for_level<'a>(
        map: &'a SkillMap,
        level: &SkillLevel,
    ) -> Vec<&'a SkillEntry> {
        map.entries
            .iter()
            .filter(|e| e.level.score() <= level.score())
            .collect()
    }

    /// Returns the first entry in `cat` by insertion order, if any.
    pub fn top_by_category<'a>(
        map: &'a SkillMap,
        cat: &SkillCategory,
    ) -> Option<&'a SkillEntry> {
        map.entries
            .iter()
            .find(|e| e.category.label() == cat.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. SkillCategory::label
    #[test]
    fn test_skill_category_label() {
        assert_eq!(SkillCategory::Formatting.label(), "formatting");
        assert_eq!(SkillCategory::Naming.label(), "naming");
        assert_eq!(SkillCategory::Complexity.label(), "complexity");
        assert_eq!(SkillCategory::Performance.label(), "performance");
        assert_eq!(SkillCategory::Style.label(), "style");
    }

    // 2. SkillCategory::is_automated
    #[test]
    fn test_skill_category_is_automated() {
        assert!(SkillCategory::Formatting.is_automated());
        assert!(SkillCategory::Naming.is_automated());
        assert!(SkillCategory::Style.is_automated());
        assert!(!SkillCategory::Complexity.is_automated());
        assert!(!SkillCategory::Performance.is_automated());
    }

    // 3. SkillLevel::score
    #[test]
    fn test_skill_level_score() {
        assert_eq!(SkillLevel::Beginner.score(), 1);
        assert_eq!(SkillLevel::Intermediate.score(), 2);
        assert_eq!(SkillLevel::Advanced.score(), 3);
    }

    // 4. SkillLevel::can_teach
    #[test]
    fn test_skill_level_can_teach() {
        assert!(SkillLevel::Advanced.can_teach(&SkillLevel::Beginner));
        assert!(SkillLevel::Advanced.can_teach(&SkillLevel::Intermediate));
        assert!(SkillLevel::Intermediate.can_teach(&SkillLevel::Beginner));
        assert!(!SkillLevel::Beginner.can_teach(&SkillLevel::Intermediate));
        assert!(!SkillLevel::Intermediate.can_teach(&SkillLevel::Advanced));
        assert!(!SkillLevel::Beginner.can_teach(&SkillLevel::Beginner));
    }

    // 5. SkillEntry::display format
    #[test]
    fn test_skill_entry_display() {
        let entry = SkillEntry {
            id: 1,
            name: "indent-check".to_string(),
            category: SkillCategory::Formatting,
            level: SkillLevel::Beginner,
        };
        assert_eq!(entry.display(), "[formatting] indent-check (1)");
    }

    // 6. SkillEntry::is_advanced
    #[test]
    fn test_skill_entry_is_advanced() {
        let advanced = SkillEntry {
            id: 2,
            name: "cyclomatic".to_string(),
            category: SkillCategory::Complexity,
            level: SkillLevel::Advanced,
        };
        let beginner = SkillEntry {
            id: 3,
            name: "simple".to_string(),
            category: SkillCategory::Naming,
            level: SkillLevel::Beginner,
        };
        assert!(advanced.is_advanced());
        assert!(!beginner.is_advanced());
    }

    // 7. SkillMap::by_category
    #[test]
    fn test_skill_map_by_category() {
        let mut map = SkillMap::new();
        map.add(SkillEntry {
            id: 1,
            name: "trailing-ws".to_string(),
            category: SkillCategory::Formatting,
            level: SkillLevel::Beginner,
        });
        map.add(SkillEntry {
            id: 2,
            name: "snake-case".to_string(),
            category: SkillCategory::Naming,
            level: SkillLevel::Intermediate,
        });
        map.add(SkillEntry {
            id: 3,
            name: "brace-align".to_string(),
            category: SkillCategory::Formatting,
            level: SkillLevel::Advanced,
        });

        let fmt = map.by_category(&SkillCategory::Formatting);
        assert_eq!(fmt.len(), 2);
        assert_eq!(fmt[0].name, "trailing-ws");
        assert_eq!(fmt[1].name, "brace-align");

        let naming = map.by_category(&SkillCategory::Naming);
        assert_eq!(naming.len(), 1);

        let perf = map.by_category(&SkillCategory::Performance);
        assert!(perf.is_empty());
    }

    // 8. SkillMap::automated_count
    #[test]
    fn test_skill_map_automated_count() {
        let mut map = SkillMap::new();
        map.add(SkillEntry {
            id: 1,
            name: "ws".to_string(),
            category: SkillCategory::Formatting,
            level: SkillLevel::Beginner,
        });
        map.add(SkillEntry {
            id: 2,
            name: "cyclo".to_string(),
            category: SkillCategory::Complexity,
            level: SkillLevel::Advanced,
        });
        map.add(SkillEntry {
            id: 3,
            name: "camel".to_string(),
            category: SkillCategory::Naming,
            level: SkillLevel::Intermediate,
        });
        map.add(SkillEntry {
            id: 4,
            name: "alloc".to_string(),
            category: SkillCategory::Performance,
            level: SkillLevel::Advanced,
        });
        map.add(SkillEntry {
            id: 5,
            name: "braces".to_string(),
            category: SkillCategory::Style,
            level: SkillLevel::Beginner,
        });
        // Formatting + Naming + Style = 3 automated
        assert_eq!(map.automated_count(), 3);
    }

    // 9. SkillRecommender::recommend_for_level filter
    #[test]
    fn test_recommender_for_level() {
        let mut map = SkillMap::new();
        map.add(SkillEntry {
            id: 1,
            name: "ws".to_string(),
            category: SkillCategory::Formatting,
            level: SkillLevel::Beginner,
        });
        map.add(SkillEntry {
            id: 2,
            name: "indent".to_string(),
            category: SkillCategory::Formatting,
            level: SkillLevel::Intermediate,
        });
        map.add(SkillEntry {
            id: 3,
            name: "cyclo".to_string(),
            category: SkillCategory::Complexity,
            level: SkillLevel::Advanced,
        });

        // Intermediate sees Beginner + Intermediate (score <=2), not Advanced
        let recs = SkillRecommender::recommend_for_level(&map, &SkillLevel::Intermediate);
        assert_eq!(recs.len(), 2);
        assert!(recs.iter().all(|e| e.level.score() <= 2));

        // Beginner sees only Beginner (score <=1)
        let recs_b = SkillRecommender::recommend_for_level(&map, &SkillLevel::Beginner);
        assert_eq!(recs_b.len(), 1);
        assert_eq!(recs_b[0].name, "ws");

        // Advanced sees all three
        let recs_a = SkillRecommender::recommend_for_level(&map, &SkillLevel::Advanced);
        assert_eq!(recs_a.len(), 3);
    }
}
