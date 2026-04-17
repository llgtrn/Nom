#![deny(unsafe_code)]

use crate::dock::RenderPrimitive;

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

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        render_member(&self.root, 0.0, 0.0, width, height)
    }
}

fn render_member(member: &Member, x: f32, y: f32, w: f32, h: f32) -> Vec<RenderPrimitive> {
    match member {
        Member::Pane(pane) => render_pane(pane, x, y, w, h),
        Member::Axis(axis) => render_axis(axis, x, y, w, h),
    }
}

fn render_pane(pane: &Pane, x: f32, y: f32, w: f32, h: f32) -> Vec<RenderPrimitive> {
    let _ = h;
    let mut out = Vec::new();

    // Tab bar background
    out.push(RenderPrimitive::Rect { x, y, w, h: 28.0, color: 0x181825 });

    // Tab labels
    let mut tab_x = x + 8.0;
    for (i, tab) in pane.tabs.iter().enumerate() {
        let is_active = pane.active_tab == Some(i);
        let label = if tab.is_dirty {
            format!("{} ●", tab.title)
        } else {
            tab.title.clone()
        };
        out.push(RenderPrimitive::Text {
            x: tab_x,
            y: y + 7.0,
            text: label,
            size: 13.0,
            color: if is_active { 0xcdd6f4 } else { 0x6c7086 },
        });
        if is_active {
            // Active tab underline
            out.push(RenderPrimitive::Rect {
                x: tab_x - 4.0,
                y: y + 26.0,
                w: 80.0,
                h: 2.0,
                color: 0x89b4fa,
            });
        }
        tab_x += 100.0;
    }

    out
}

fn render_axis(axis: &PaneAxis, x: f32, y: f32, w: f32, h: f32) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    let n = axis.members.len();
    if n == 0 { return out; }

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
        out.extend(render_member(member, mx, my, mw, mh));

        // Draw split line between members (not after the last one)
        if i + 1 < n {
            match axis.direction {
                SplitDirection::Horizontal => {
                    let line_x = x + offset + flex * w;
                    out.push(RenderPrimitive::Line {
                        x1: line_x, y1: y,
                        x2: line_x, y2: y + h,
                        color: 0x313244,
                    });
                }
                SplitDirection::Vertical => {
                    let line_y = y + offset + flex * h;
                    out.push(RenderPrimitive::Line {
                        x1: x,       y1: line_y,
                        x2: x + w,   y2: line_y,
                        color: 0x313244,
                    });
                }
            }
        }

        match axis.direction {
            SplitDirection::Horizontal => offset += flex * w,
            SplitDirection::Vertical   => offset += flex * h,
        }
    }

    out
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
    fn pane_group_render_tab_bar() {
        let mut g = PaneGroup::single("main");
        if let Member::Pane(ref mut p) = g.root {
            p.open_tab("file.nom", "file.nom");
            p.open_tab("other.nom", "other.nom");
        }
        let prims = g.render_bounds(800.0, 600.0);

        // Tab bar rect at top
        match &prims[0] {
            RenderPrimitive::Rect { x, y, w, h, color } => {
                assert!((x - 0.0).abs() < 0.01);
                assert!((y - 0.0).abs() < 0.01);
                assert!((w - 800.0).abs() < 0.01);
                assert!((h - 28.0).abs() < 0.01);
                assert_eq!(*color, 0x181825);
            }
            _ => panic!("expected tab bar Rect"),
        }

        // Active tab underline should be present
        let has_underline = prims.iter().any(|p| matches!(p,
            RenderPrimitive::Rect { h, color: 0x89b4fa, .. } if (*h - 2.0).abs() < 0.01
        ));
        assert!(has_underline, "active tab underline missing");

        // Both tab labels present as Text
        let texts: Vec<&str> = prims.iter().filter_map(|p| {
            if let RenderPrimitive::Text { text, .. } = p { Some(text.as_str()) } else { None }
        }).collect();
        assert!(texts.iter().any(|t| t.contains("file.nom")));
        assert!(texts.iter().any(|t| t.contains("other.nom")));
    }

    #[test]
    fn pane_group_render_split_line() {
        let mut g = PaneGroup::single("left");
        g.split(SplitDirection::Horizontal, "right");
        let prims = g.render_bounds(800.0, 600.0);

        let has_split_line = prims.iter().any(|p| matches!(p,
            RenderPrimitive::Line { color: 0x313244, .. }
        ));
        assert!(has_split_line, "split line missing for horizontal split");
    }
}
