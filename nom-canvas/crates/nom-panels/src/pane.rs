#![deny(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection { Horizontal, Vertical }

#[derive(Debug, Clone)]
pub struct PaneTab {
    pub id: String,
    pub title: String,
    pub is_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub id: String,
    pub tabs: Vec<PaneTab>,
    pub active_tab: Option<usize>,
}

impl Pane {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), tabs: vec![], active_tab: None }
    }

    pub fn open_tab(&mut self, id: impl Into<String>, title: impl Into<String>) {
        let id = id.into();
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.active_tab = Some(pos);
        } else {
            self.tabs.push(PaneTab { id, title: title.into(), is_dirty: false });
            self.active_tab = Some(self.tabs.len() - 1);
        }
    }

    pub fn close_tab(&mut self, id: &str) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            self.active_tab = if self.tabs.is_empty() {
                None
            } else {
                Some(pos.saturating_sub(1))
            };
            true
        } else {
            false
        }
    }

    pub fn active_tab(&self) -> Option<&PaneTab> {
        self.active_tab.and_then(|i| self.tabs.get(i))
    }
}

pub struct PaneAxis {
    pub direction: SplitDirection,
    pub members: Vec<Member>,
    pub flexes: Vec<f32>,  // proportions summing to 1.0
}

impl PaneAxis {
    pub fn new(direction: SplitDirection) -> Self {
        Self { direction, members: vec![], flexes: vec![] }
    }

    pub fn push(&mut self, member: Member) {
        let n = self.members.len() + 1;
        let even = 1.0 / n as f32;
        self.flexes = vec![even; n];
        self.members.push(member);
    }

    pub fn adjust_flex(&mut self, idx: usize, delta: f32) {
        if idx + 1 >= self.flexes.len() { return; }
        let moved = delta.clamp(-self.flexes[idx], self.flexes[idx + 1]);
        self.flexes[idx] += moved;
        self.flexes[idx + 1] -= moved;
    }
}

pub enum Member {
    Pane(Pane),
    Axis(PaneAxis),
}

impl Member {
    pub fn pane_count(&self) -> usize {
        match self {
            Member::Pane(_) => 1,
            Member::Axis(ax) => ax.members.iter().map(|m| m.pane_count()).sum(),
        }
    }
}

pub struct PaneGroup {
    pub root: Member,
}

impl PaneGroup {
    pub fn single(id: impl Into<String>) -> Self {
        Self { root: Member::Pane(Pane::new(id)) }
    }

    pub fn pane_count(&self) -> usize { self.root.pane_count() }

    pub fn split(&mut self, direction: SplitDirection, new_id: impl Into<String>) {
        let existing = std::mem::replace(&mut self.root, Member::Pane(Pane::new("")));
        let mut axis = PaneAxis::new(direction);
        axis.push(existing);
        axis.push(Member::Pane(Pane::new(new_id)));
        self.root = Member::Axis(axis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_tab_lifecycle() {
        let mut p = Pane::new("main");
        p.open_tab("file.nom", "file.nom");
        p.open_tab("other.nom", "other.nom");
        assert_eq!(p.tabs.len(), 2);
        assert_eq!(p.active_tab().unwrap().id, "other.nom");
        p.close_tab("other.nom");
        assert_eq!(p.active_tab().unwrap().id, "file.nom");
    }

    #[test]
    fn pane_group_split() {
        let mut g = PaneGroup::single("left");
        assert_eq!(g.pane_count(), 1);
        g.split(SplitDirection::Horizontal, "right");
        assert_eq!(g.pane_count(), 2);
    }

    #[test]
    fn pane_axis_flex_adjust() {
        let mut ax = PaneAxis::new(SplitDirection::Horizontal);
        ax.push(Member::Pane(Pane::new("a")));
        ax.push(Member::Pane(Pane::new("b")));
        assert!((ax.flexes[0] + ax.flexes[1] - 1.0).abs() < 0.001);
        ax.adjust_flex(0, 0.1);
        assert!((ax.flexes[0] - 0.6).abs() < 0.001);
        assert!((ax.flexes[1] - 0.4).abs() < 0.001);
    }
}
