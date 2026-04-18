#[derive(Debug, Clone, PartialEq)]
pub enum ScreenKind {
    Canvas,
    Dashboard,
    Editor,
    Onboarding,
    Settings,
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub id: String,
    pub kind: ScreenKind,
    pub title: String,
}

impl Screen {
    pub fn new(id: &str, kind: ScreenKind, title: &str) -> Self {
        Self {
            id: id.to_owned(),
            kind,
            title: title.to_owned(),
        }
    }

    /// Returns true for the two primary editing surfaces.
    pub fn is_primary(&self) -> bool {
        matches!(self.kind, ScreenKind::Canvas | ScreenKind::Editor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_screen_fields() {
        let s = Screen::new("s1", ScreenKind::Dashboard, "Home");
        assert_eq!(s.id, "s1");
        assert_eq!(s.kind, ScreenKind::Dashboard);
        assert_eq!(s.title, "Home");
    }

    #[test]
    fn is_primary_canvas_and_editor() {
        assert!(Screen::new("s2", ScreenKind::Canvas, "C").is_primary());
        assert!(Screen::new("s3", ScreenKind::Editor, "E").is_primary());
        assert!(!Screen::new("s4", ScreenKind::Settings, "S").is_primary());
        assert!(!Screen::new("s5", ScreenKind::Onboarding, "O").is_primary());
        assert!(!Screen::new("s6", ScreenKind::Dashboard, "D").is_primary());
    }
}
