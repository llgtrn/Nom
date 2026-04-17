#![deny(unsafe_code)]

use crate::dock::fill_quad;
use nom_gpui::scene::Scene;
use nom_theme::tokens;

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

    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        paint_member(&self.root, 0.0, 0.0, width, height, scene);
    }
}

fn paint_member(member: &Member, x: f32, y: f32, w: f32, h: f32, scene: &mut Scene) {
    match member {
        Member::Pane(pane) => paint_pane(pane, x, y, w, h, scene),
        Member::Axis(axis) => paint_axis(axis, x, y, w, h, scene),
    }
}

fn paint_pane(pane: &Pane, x: f32, y: f32, w: f32, _h: f32, scene: &mut Scene) {
    // Tab bar background strip.
    scene.push_quad(fill_quad(x, y, w, 28.0, tokens::BG2));

    // Active-tab underline.
    let mut tab_x = x + 8.0;
    for (i, _tab) in pane.tabs.iter().enumerate() {
        let is_active = pane.active_tab == Some(i);
        if is_active {
            scene.push_quad(fill_quad(tab_x - 4.0, y + 26.0, 80.0, 2.0, tokens::CTA));
        }
        tab_x += 100.0;
    }
}

fn paint_axis(axis: &PaneAxis, x: f32, y: f32, w: f32, h: f32, scene: &mut Scene) {
    let n = axis.members.len();
    if n == 0 { return; }

    let mut offset = 0.0;
    for (i, (member, &flex)) in axis.members.iter().zip(axis.flexes.iter()).enumerate() {
        let (mx, my, mw, mh) = match axis.direction {
            SplitDirection::Horizontal => {
                let member_w = flex * w;
                (x + offset, y, member_w, h)
            }
            SplitDirection::Vertical => {
                let member_h = flex * h;
                (x, y + offset, w, member_h)
            }
        };
        paint_member(member, mx, my, mw, mh, scene);

        // Split-divider quad between members (1 px on the splitting axis).
        if i + 1 < n {
            match axis.direction {
                SplitDirection::Horizontal => {
                    let line_x = x + offset + flex * w;
                    scene.push_quad(fill_quad(line_x, y, 1.0, h, tokens::BORDER));
                }
                SplitDirection::Vertical => {
                    let line_y = y + offset + flex * h;
                    scene.push_quad(fill_quad(x, line_y, w, 1.0, tokens::BORDER));
                }
            }
        }

        match axis.direction {
            SplitDirection::Horizontal => offset += flex * w,
            SplitDirection::Vertical   => offset += flex * h,
        }
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

    #[test]
    fn pane_group_paint_tab_bar() {
        let mut g = PaneGroup::single("main");
        if let Member::Pane(ref mut p) = g.root {
            p.open_tab("file.nom", "file.nom");
            p.open_tab("other.nom", "other.nom");
        }
        let mut scene = Scene::new();
        g.paint_scene(800.0, 600.0, &mut scene);
        // Tab bar + active-tab underline.
        assert!(scene.quads.len() >= 2);
        let bar = &scene.quads[0];
        assert_eq!(bar.bounds.size.height, nom_gpui::types::Pixels(28.0));
    }

    #[test]
    fn pane_group_paint_split_line() {
        let mut g = PaneGroup::single("left");
        g.split(SplitDirection::Horizontal, "right");
        let mut scene = Scene::new();
        g.paint_scene(800.0, 600.0, &mut scene);
        // >=2 pane tab bars + 1 split divider.
        assert!(scene.quads.len() >= 3, "expected >=3 quads, got {}", scene.quads.len());
    }
}
