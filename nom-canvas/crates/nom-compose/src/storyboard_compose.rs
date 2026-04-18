//! Storyboard composition primitives for sequencing scenes, acts, and panels.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneType {
    Intro,
    Action,
    Dialogue,
    Transition,
    Outro,
}

impl SceneType {
    /// Returns true for narrative scene types (Intro, Dialogue, Outro).
    pub fn is_narrative(&self) -> bool {
        matches!(self, SceneType::Intro | SceneType::Dialogue | SceneType::Outro)
    }

    /// Returns the nominal duration hint in seconds for this scene type.
    pub fn duration_hint_secs(&self) -> u32 {
        match self {
            SceneType::Intro => 5,
            SceneType::Action => 10,
            SceneType::Dialogue => 8,
            SceneType::Transition => 2,
            SceneType::Outro => 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoryboardPanel {
    pub index: u32,
    pub scene_type: SceneType,
    pub description: String,
}

impl StoryboardPanel {
    /// Returns true if this panel is a key scene (Intro, Outro, or Action).
    pub fn is_key_scene(&self) -> bool {
        matches!(
            self.scene_type,
            SceneType::Intro | SceneType::Outro | SceneType::Action
        )
    }

    /// Returns a formatted label: "Panel {index}: [{duration_hint}] {description}".
    pub fn panel_label(&self) -> String {
        format!(
            "Panel {}: [{}] {}",
            self.index,
            self.scene_type.duration_hint_secs(),
            self.description
        )
    }
}

#[derive(Debug, Clone)]
pub struct StoryboardAct {
    pub act_number: u32,
    pub panels: Vec<StoryboardPanel>,
}

impl StoryboardAct {
    /// Returns the sum of all panel duration hints in seconds.
    pub fn total_duration_secs(&self) -> u32 {
        self.panels
            .iter()
            .map(|p| p.scene_type.duration_hint_secs())
            .sum()
    }

    /// Returns the count of panels where `is_key_scene` is true.
    pub fn key_scene_count(&self) -> usize {
        self.panels.iter().filter(|p| p.is_key_scene()).count()
    }

    /// Returns the total number of panels in this act.
    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }
}

#[derive(Debug, Clone)]
pub struct Storyboard {
    pub title: String,
    pub acts: Vec<StoryboardAct>,
}

impl Storyboard {
    /// Creates a new empty storyboard with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            acts: Vec::new(),
        }
    }

    /// Appends an act to this storyboard.
    pub fn add_act(&mut self, act: StoryboardAct) {
        self.acts.push(act);
    }

    /// Returns the total number of panels across all acts.
    pub fn total_panels(&self) -> usize {
        self.acts.iter().map(|a| a.panel_count()).sum()
    }

    /// Returns the total duration in seconds across all acts.
    pub fn total_duration_secs(&self) -> u32 {
        self.acts.iter().map(|a| a.total_duration_secs()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct StoryboardComposer {
    pub storyboard: Storyboard,
}

impl StoryboardComposer {
    /// Creates a new composer wrapping a fresh storyboard with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            storyboard: Storyboard::new(title),
        }
    }

    /// Adds a panel to the act identified by `act_number`. If no such act exists,
    /// a new act is created and appended before inserting the panel.
    pub fn add_panel(&mut self, act: u32, panel: StoryboardPanel) {
        if let Some(existing) = self
            .storyboard
            .acts
            .iter_mut()
            .find(|a| a.act_number == act)
        {
            existing.panels.push(panel);
        } else {
            let mut new_act = StoryboardAct {
                act_number: act,
                panels: Vec::new(),
            };
            new_act.panels.push(panel);
            self.storyboard.acts.push(new_act);
        }
    }

    /// Returns a one-line summary: "{title}: {n} acts, {m} panels, {s}s".
    pub fn summary(&self) -> String {
        format!(
            "{}: {} acts, {} panels, {}s",
            self.storyboard.title,
            self.storyboard.acts.len(),
            self.storyboard.total_panels(),
            self.storyboard.total_duration_secs(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: SceneType::is_narrative — narrative variants return true, non-narrative false.
    #[test]
    fn scene_type_is_narrative() {
        assert!(SceneType::Intro.is_narrative());
        assert!(SceneType::Dialogue.is_narrative());
        assert!(SceneType::Outro.is_narrative());
        assert!(!SceneType::Action.is_narrative());
        assert!(!SceneType::Transition.is_narrative());
    }

    // Test 2: SceneType::duration_hint_secs — correct values per variant.
    #[test]
    fn scene_type_duration_hint_secs() {
        assert_eq!(SceneType::Intro.duration_hint_secs(), 5);
        assert_eq!(SceneType::Action.duration_hint_secs(), 10);
        assert_eq!(SceneType::Dialogue.duration_hint_secs(), 8);
        assert_eq!(SceneType::Transition.duration_hint_secs(), 2);
        assert_eq!(SceneType::Outro.duration_hint_secs(), 5);
    }

    // Test 3: StoryboardPanel::is_key_scene — Intro/Outro/Action are key; Dialogue/Transition are not.
    #[test]
    fn storyboard_panel_is_key_scene() {
        let make = |st: SceneType| StoryboardPanel {
            index: 0,
            scene_type: st,
            description: String::new(),
        };
        assert!(make(SceneType::Intro).is_key_scene());
        assert!(make(SceneType::Outro).is_key_scene());
        assert!(make(SceneType::Action).is_key_scene());
        assert!(!make(SceneType::Dialogue).is_key_scene());
        assert!(!make(SceneType::Transition).is_key_scene());
    }

    // Test 4: StoryboardPanel::panel_label — format matches expected pattern.
    #[test]
    fn storyboard_panel_label_format() {
        let panel = StoryboardPanel {
            index: 3,
            scene_type: SceneType::Action,
            description: "Hero leaps".to_string(),
        };
        assert_eq!(panel.panel_label(), "Panel 3: [10] Hero leaps");

        let panel2 = StoryboardPanel {
            index: 1,
            scene_type: SceneType::Dialogue,
            description: "Opening monologue".to_string(),
        };
        assert_eq!(panel2.panel_label(), "Panel 1: [8] Opening monologue");
    }

    // Test 5: StoryboardAct::total_duration_secs — sums all panel durations correctly.
    #[test]
    fn storyboard_act_total_duration_secs() {
        let act = StoryboardAct {
            act_number: 1,
            panels: vec![
                StoryboardPanel {
                    index: 0,
                    scene_type: SceneType::Intro,       // 5
                    description: String::new(),
                },
                StoryboardPanel {
                    index: 1,
                    scene_type: SceneType::Action,      // 10
                    description: String::new(),
                },
                StoryboardPanel {
                    index: 2,
                    scene_type: SceneType::Transition,  // 2
                    description: String::new(),
                },
            ],
        };
        assert_eq!(act.total_duration_secs(), 17);
    }

    // Test 6: StoryboardAct::key_scene_count — counts only key-scene panels.
    #[test]
    fn storyboard_act_key_scene_count() {
        let act = StoryboardAct {
            act_number: 1,
            panels: vec![
                StoryboardPanel {
                    index: 0,
                    scene_type: SceneType::Intro,
                    description: String::new(),
                },
                StoryboardPanel {
                    index: 1,
                    scene_type: SceneType::Dialogue,
                    description: String::new(),
                },
                StoryboardPanel {
                    index: 2,
                    scene_type: SceneType::Action,
                    description: String::new(),
                },
                StoryboardPanel {
                    index: 3,
                    scene_type: SceneType::Transition,
                    description: String::new(),
                },
            ],
        };
        // Intro + Action = 2 key scenes
        assert_eq!(act.key_scene_count(), 2);
    }

    // Test 7: Storyboard::total_panels — sums panel_count across all acts.
    #[test]
    fn storyboard_total_panels() {
        let mut sb = Storyboard::new("My Film");
        sb.add_act(StoryboardAct {
            act_number: 1,
            panels: vec![
                StoryboardPanel { index: 0, scene_type: SceneType::Intro, description: String::new() },
                StoryboardPanel { index: 1, scene_type: SceneType::Action, description: String::new() },
            ],
        });
        sb.add_act(StoryboardAct {
            act_number: 2,
            panels: vec![
                StoryboardPanel { index: 2, scene_type: SceneType::Outro, description: String::new() },
            ],
        });
        assert_eq!(sb.total_panels(), 3);
    }

    // Test 8: Storyboard::total_duration_secs — sums durations across all acts.
    #[test]
    fn storyboard_total_duration_secs() {
        let mut sb = Storyboard::new("Epic");
        sb.add_act(StoryboardAct {
            act_number: 1,
            panels: vec![
                StoryboardPanel { index: 0, scene_type: SceneType::Intro, description: String::new() },      // 5
                StoryboardPanel { index: 1, scene_type: SceneType::Dialogue, description: String::new() },   // 8
            ],
        });
        sb.add_act(StoryboardAct {
            act_number: 2,
            panels: vec![
                StoryboardPanel { index: 2, scene_type: SceneType::Outro, description: String::new() },      // 5
            ],
        });
        // 5 + 8 + 5 = 18
        assert_eq!(sb.total_duration_secs(), 18);
    }

    // Test 9: StoryboardComposer::summary — format matches "{title}: {acts} acts, {panels} panels, {secs}s".
    #[test]
    fn storyboard_composer_summary_format() {
        let mut composer = StoryboardComposer::new("Short Film");
        composer.add_panel(
            1,
            StoryboardPanel { index: 0, scene_type: SceneType::Intro, description: "Opening".to_string() },
        );
        composer.add_panel(
            1,
            StoryboardPanel { index: 1, scene_type: SceneType::Action, description: "Chase".to_string() },
        );
        composer.add_panel(
            2,
            StoryboardPanel { index: 2, scene_type: SceneType::Outro, description: "End".to_string() },
        );
        // 2 acts, 3 panels, 5+10+5 = 20s
        assert_eq!(
            composer.summary(),
            "Short Film: 2 acts, 3 panels, 20s"
        );
    }
}
