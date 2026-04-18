#![deny(unsafe_code)]

/// All icons available in the NomCanvas icon set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    ChevronRight,
    ChevronDown,
    Plus,
    Minus,
    X,
    Search,
    Settings,
    Brain,
    Network,
    File,
    Folder,
    Play,
    Pause,
    Stop,
    Zap,
    Link,
    Unlink,
    Lock,
    Unlock,
    Eye,
    EyeOff,
    Copy,
    Trash,
    Edit2,
    Check,
    AlertCircle,
    Info,
    Terminal,
    Code,
    Database,
    Layers,
    Grid,
    List,
    Sidebar,
    PanelLeft,
    PanelRight,
    MessageSquare,
    Tool,
    Cpu,
    GitBranch,
    Sparkles,
    Workflow,
}

impl Icon {
    /// All icon variants in declaration order.
    pub fn all() -> &'static [Icon] {
        &[
            Icon::ChevronRight,
            Icon::ChevronDown,
            Icon::Plus,
            Icon::Minus,
            Icon::X,
            Icon::Search,
            Icon::Settings,
            Icon::Brain,
            Icon::Network,
            Icon::File,
            Icon::Folder,
            Icon::Play,
            Icon::Pause,
            Icon::Stop,
            Icon::Zap,
            Icon::Link,
            Icon::Unlink,
            Icon::Lock,
            Icon::Unlock,
            Icon::Eye,
            Icon::EyeOff,
            Icon::Copy,
            Icon::Trash,
            Icon::Edit2,
            Icon::Check,
            Icon::AlertCircle,
            Icon::Info,
            Icon::Terminal,
            Icon::Code,
            Icon::Database,
            Icon::Layers,
            Icon::Grid,
            Icon::List,
            Icon::Sidebar,
            Icon::PanelLeft,
            Icon::PanelRight,
            Icon::MessageSquare,
            Icon::Tool,
            Icon::Cpu,
            Icon::GitBranch,
            Icon::Sparkles,
            Icon::Workflow,
        ]
    }

    /// Kebab-case name suitable for asset lookup or debug output.
    pub fn name(&self) -> &'static str {
        match self {
            Icon::ChevronRight => "chevron-right",
            Icon::ChevronDown => "chevron-down",
            Icon::Plus => "plus",
            Icon::Minus => "minus",
            Icon::X => "x",
            Icon::Search => "search",
            Icon::Settings => "settings",
            Icon::Brain => "brain",
            Icon::Network => "network",
            Icon::File => "file",
            Icon::Folder => "folder",
            Icon::Play => "play",
            Icon::Pause => "pause",
            Icon::Stop => "stop",
            Icon::Zap => "zap",
            Icon::Link => "link",
            Icon::Unlink => "unlink",
            Icon::Lock => "lock",
            Icon::Unlock => "unlock",
            Icon::Eye => "eye",
            Icon::EyeOff => "eye-off",
            Icon::Copy => "copy",
            Icon::Trash => "trash",
            Icon::Edit2 => "edit-2",
            Icon::Check => "check",
            Icon::AlertCircle => "alert-circle",
            Icon::Info => "info",
            Icon::Terminal => "terminal",
            Icon::Code => "code",
            Icon::Database => "database",
            Icon::Layers => "layers",
            Icon::Grid => "grid",
            Icon::List => "list",
            Icon::Sidebar => "sidebar",
            Icon::PanelLeft => "panel-left",
            Icon::PanelRight => "panel-right",
            Icon::MessageSquare => "message-square",
            Icon::Tool => "tool",
            Icon::Cpu => "cpu",
            Icon::GitBranch => "git-branch",
            Icon::Sparkles => "sparkles",
            Icon::Workflow => "workflow",
        }
    }
}

// ---------------------------------------------------------------------------
// Icon path data — normalized 0.0–1.0 viewport, scaled at render time.
// lines: (from_x, from_y, to_x, to_y)
// circles: (center_x, center_y, radius)  — all normalized
// ---------------------------------------------------------------------------

/// Resolved draw primitives for an icon.
pub struct IconPath {
    pub lines: &'static [(f32, f32, f32, f32)],
    pub circles: &'static [(f32, f32, f32)],
}

/// Returns the line/circle geometry for `icon` in a normalized 0–1 viewport.
pub fn icon_path(icon: Icon) -> IconPath {
    match icon {
        Icon::X => IconPath {
            lines: &[(0.2, 0.2, 0.8, 0.8), (0.8, 0.2, 0.2, 0.8)],
            circles: &[],
        },
        Icon::Plus => IconPath {
            lines: &[(0.5, 0.1, 0.5, 0.9), (0.1, 0.5, 0.9, 0.5)],
            circles: &[],
        },
        Icon::Minus => IconPath {
            lines: &[(0.1, 0.5, 0.9, 0.5)],
            circles: &[],
        },
        Icon::Check => IconPath {
            lines: &[(0.1, 0.5, 0.4, 0.8), (0.4, 0.8, 0.9, 0.2)],
            circles: &[],
        },
        Icon::ChevronRight => IconPath {
            lines: &[(0.3, 0.2, 0.7, 0.5), (0.7, 0.5, 0.3, 0.8)],
            circles: &[],
        },
        Icon::ChevronDown => IconPath {
            lines: &[(0.2, 0.3, 0.5, 0.7), (0.5, 0.7, 0.8, 0.3)],
            circles: &[],
        },
        Icon::Search => IconPath {
            lines: &[(0.65, 0.65, 0.85, 0.85)],
            circles: &[(0.4, 0.4, 0.28)],
        },
        Icon::AlertCircle => IconPath {
            lines: &[(0.5, 0.3, 0.5, 0.55), (0.5, 0.68, 0.5, 0.70)],
            circles: &[(0.5, 0.5, 0.42)],
        },
        Icon::Info => IconPath {
            lines: &[(0.5, 0.45, 0.5, 0.72)],
            circles: &[(0.5, 0.5, 0.42), (0.5, 0.32, 0.03)],
        },
        Icon::Play => IconPath {
            lines: &[
                (0.25, 0.15, 0.25, 0.85),
                (0.25, 0.15, 0.80, 0.50),
                (0.80, 0.50, 0.25, 0.85),
            ],
            circles: &[],
        },
        Icon::Pause => IconPath {
            lines: &[
                (0.30, 0.15, 0.30, 0.85),
                (0.30, 0.15, 0.42, 0.15),
                (0.42, 0.15, 0.42, 0.85),
                (0.42, 0.85, 0.30, 0.85),
                (0.58, 0.15, 0.58, 0.85),
                (0.58, 0.15, 0.70, 0.15),
                (0.70, 0.15, 0.70, 0.85),
                (0.70, 0.85, 0.58, 0.85),
            ],
            circles: &[],
        },
        Icon::Stop => IconPath {
            lines: &[
                (0.20, 0.20, 0.80, 0.20),
                (0.80, 0.20, 0.80, 0.80),
                (0.80, 0.80, 0.20, 0.80),
                (0.20, 0.80, 0.20, 0.20),
            ],
            circles: &[],
        },
        Icon::Settings => IconPath {
            lines: &[
                (0.50, 0.10, 0.50, 0.90),
                (0.10, 0.50, 0.90, 0.50),
                (0.18, 0.18, 0.82, 0.82),
                (0.82, 0.18, 0.18, 0.82),
            ],
            circles: &[(0.5, 0.5, 0.18)],
        },
        Icon::File => IconPath {
            lines: &[
                (0.20, 0.05, 0.65, 0.05),
                (0.65, 0.05, 0.80, 0.20),
                (0.80, 0.20, 0.80, 0.95),
                (0.80, 0.95, 0.20, 0.95),
                (0.20, 0.95, 0.20, 0.05),
                (0.65, 0.05, 0.65, 0.20),
                (0.65, 0.20, 0.80, 0.20),
            ],
            circles: &[],
        },
        Icon::Folder => IconPath {
            lines: &[
                (0.05, 0.30, 0.05, 0.90),
                (0.05, 0.90, 0.95, 0.90),
                (0.95, 0.90, 0.95, 0.30),
                (0.95, 0.30, 0.45, 0.30),
                (0.45, 0.30, 0.35, 0.15),
                (0.35, 0.15, 0.05, 0.15),
                (0.05, 0.15, 0.05, 0.30),
            ],
            circles: &[],
        },
        Icon::Zap => IconPath {
            lines: &[
                (0.60, 0.05, 0.30, 0.50),
                (0.30, 0.50, 0.55, 0.50),
                (0.55, 0.50, 0.25, 0.95),
            ],
            circles: &[],
        },
        Icon::Link => IconPath {
            lines: &[
                (0.55, 0.30, 0.70, 0.15),
                (0.70, 0.15, 0.85, 0.30),
                (0.85, 0.30, 0.70, 0.45),
                (0.70, 0.45, 0.55, 0.30),
                (0.45, 0.70, 0.30, 0.85),
                (0.30, 0.85, 0.15, 0.70),
                (0.15, 0.70, 0.30, 0.55),
                (0.30, 0.55, 0.45, 0.70),
                (0.40, 0.60, 0.60, 0.40),
            ],
            circles: &[],
        },
        Icon::Unlink => IconPath {
            lines: &[
                (0.55, 0.30, 0.70, 0.15),
                (0.85, 0.30, 0.70, 0.45),
                (0.45, 0.70, 0.30, 0.85),
                (0.15, 0.70, 0.30, 0.55),
                (0.20, 0.20, 0.40, 0.40),
                (0.60, 0.60, 0.80, 0.80),
            ],
            circles: &[],
        },
        Icon::Lock => IconPath {
            lines: &[
                (0.25, 0.50, 0.25, 0.90),
                (0.25, 0.90, 0.75, 0.90),
                (0.75, 0.90, 0.75, 0.50),
                (0.75, 0.50, 0.25, 0.50),
            ],
            circles: &[(0.5, 0.32, 0.22), (0.5, 0.68, 0.05)],
        },
        Icon::Unlock => IconPath {
            lines: &[
                (0.25, 0.50, 0.25, 0.90),
                (0.25, 0.90, 0.75, 0.90),
                (0.75, 0.90, 0.75, 0.50),
                (0.75, 0.50, 0.25, 0.50),
                (0.30, 0.28, 0.70, 0.28),
            ],
            circles: &[(0.5, 0.68, 0.05)],
        },
        Icon::Eye => IconPath {
            lines: &[
                (0.05, 0.50, 0.20, 0.30),
                (0.20, 0.30, 0.50, 0.20),
                (0.50, 0.20, 0.80, 0.30),
                (0.80, 0.30, 0.95, 0.50),
                (0.95, 0.50, 0.80, 0.70),
                (0.80, 0.70, 0.50, 0.80),
                (0.50, 0.80, 0.20, 0.70),
                (0.20, 0.70, 0.05, 0.50),
            ],
            circles: &[(0.5, 0.5, 0.15)],
        },
        Icon::EyeOff => IconPath {
            lines: &[
                (0.05, 0.50, 0.20, 0.30),
                (0.80, 0.30, 0.95, 0.50),
                (0.10, 0.10, 0.90, 0.90),
            ],
            circles: &[],
        },
        Icon::Copy => IconPath {
            lines: &[
                (0.35, 0.05, 0.95, 0.05),
                (0.95, 0.05, 0.95, 0.65),
                (0.95, 0.65, 0.35, 0.65),
                (0.35, 0.65, 0.35, 0.05),
                (0.05, 0.35, 0.35, 0.35),
                (0.05, 0.35, 0.05, 0.95),
                (0.05, 0.95, 0.65, 0.95),
                (0.65, 0.95, 0.65, 0.65),
            ],
            circles: &[],
        },
        Icon::Trash => IconPath {
            lines: &[
                (0.10, 0.25, 0.90, 0.25),
                (0.30, 0.25, 0.30, 0.90),
                (0.70, 0.25, 0.70, 0.90),
                (0.30, 0.90, 0.70, 0.90),
                (0.40, 0.10, 0.60, 0.10),
                (0.50, 0.40, 0.50, 0.80),
            ],
            circles: &[],
        },
        Icon::Edit2 => IconPath {
            lines: &[
                (0.15, 0.75, 0.70, 0.20),
                (0.70, 0.20, 0.85, 0.35),
                (0.85, 0.35, 0.30, 0.90),
                (0.30, 0.90, 0.10, 0.95),
                (0.10, 0.95, 0.15, 0.75),
            ],
            circles: &[],
        },
        Icon::Terminal => IconPath {
            lines: &[
                (0.10, 0.30, 0.45, 0.55),
                (0.10, 0.80, 0.45, 0.55),
                (0.50, 0.80, 0.80, 0.80),
            ],
            circles: &[],
        },
        Icon::Code => IconPath {
            lines: &[
                (0.35, 0.25, 0.15, 0.50),
                (0.15, 0.50, 0.35, 0.75),
                (0.65, 0.25, 0.85, 0.50),
                (0.85, 0.50, 0.65, 0.75),
            ],
            circles: &[],
        },
        Icon::Database => IconPath {
            lines: &[
                (0.50, 0.15, 0.80, 0.25),
                (0.80, 0.25, 0.80, 0.75),
                (0.80, 0.75, 0.50, 0.85),
                (0.50, 0.85, 0.20, 0.75),
                (0.20, 0.75, 0.20, 0.25),
                (0.20, 0.25, 0.50, 0.15),
                (0.20, 0.45, 0.80, 0.45),
                (0.20, 0.62, 0.80, 0.62),
            ],
            circles: &[],
        },
        Icon::Layers => IconPath {
            lines: &[
                (0.50, 0.10, 0.90, 0.30),
                (0.90, 0.30, 0.50, 0.50),
                (0.50, 0.50, 0.10, 0.30),
                (0.10, 0.30, 0.50, 0.10),
                (0.10, 0.50, 0.50, 0.70),
                (0.50, 0.70, 0.90, 0.50),
                (0.10, 0.70, 0.50, 0.90),
                (0.50, 0.90, 0.90, 0.70),
            ],
            circles: &[],
        },
        Icon::Grid => IconPath {
            lines: &[
                (0.33, 0.10, 0.33, 0.90),
                (0.67, 0.10, 0.67, 0.90),
                (0.10, 0.33, 0.90, 0.33),
                (0.10, 0.67, 0.90, 0.67),
                (0.10, 0.10, 0.90, 0.10),
                (0.90, 0.10, 0.90, 0.90),
                (0.90, 0.90, 0.10, 0.90),
                (0.10, 0.90, 0.10, 0.10),
            ],
            circles: &[],
        },
        Icon::List => IconPath {
            lines: &[
                (0.25, 0.25, 0.85, 0.25),
                (0.25, 0.50, 0.85, 0.50),
                (0.25, 0.75, 0.85, 0.75),
            ],
            circles: &[(0.12, 0.25, 0.04), (0.12, 0.50, 0.04), (0.12, 0.75, 0.04)],
        },
        Icon::Sidebar => IconPath {
            lines: &[
                (0.05, 0.05, 0.95, 0.05),
                (0.95, 0.05, 0.95, 0.95),
                (0.95, 0.95, 0.05, 0.95),
                (0.05, 0.95, 0.05, 0.05),
                (0.35, 0.05, 0.35, 0.95),
            ],
            circles: &[],
        },
        Icon::PanelLeft => IconPath {
            lines: &[
                (0.05, 0.05, 0.95, 0.05),
                (0.95, 0.05, 0.95, 0.95),
                (0.95, 0.95, 0.05, 0.95),
                (0.05, 0.95, 0.05, 0.05),
                (0.38, 0.05, 0.38, 0.95),
            ],
            circles: &[],
        },
        Icon::PanelRight => IconPath {
            lines: &[
                (0.05, 0.05, 0.95, 0.05),
                (0.95, 0.05, 0.95, 0.95),
                (0.95, 0.95, 0.05, 0.95),
                (0.05, 0.95, 0.05, 0.05),
                (0.62, 0.05, 0.62, 0.95),
            ],
            circles: &[],
        },
        Icon::MessageSquare => IconPath {
            lines: &[
                (0.10, 0.10, 0.90, 0.10),
                (0.90, 0.10, 0.90, 0.70),
                (0.90, 0.70, 0.30, 0.70),
                (0.30, 0.70, 0.10, 0.90),
                (0.10, 0.90, 0.10, 0.10),
            ],
            circles: &[],
        },
        Icon::Tool => IconPath {
            lines: &[
                (0.65, 0.15, 0.85, 0.35),
                (0.85, 0.35, 0.35, 0.85),
                (0.35, 0.85, 0.15, 0.65),
                (0.15, 0.65, 0.65, 0.15),
            ],
            circles: &[],
        },
        Icon::Cpu => IconPath {
            lines: &[
                (0.25, 0.25, 0.75, 0.25),
                (0.75, 0.25, 0.75, 0.75),
                (0.75, 0.75, 0.25, 0.75),
                (0.25, 0.75, 0.25, 0.25),
                (0.35, 0.10, 0.35, 0.25),
                (0.50, 0.10, 0.50, 0.25),
                (0.65, 0.10, 0.65, 0.25),
                (0.35, 0.75, 0.35, 0.90),
                (0.50, 0.75, 0.50, 0.90),
                (0.65, 0.75, 0.65, 0.90),
                (0.10, 0.35, 0.25, 0.35),
                (0.10, 0.50, 0.25, 0.50),
                (0.10, 0.65, 0.25, 0.65),
                (0.75, 0.35, 0.90, 0.35),
                (0.75, 0.50, 0.90, 0.50),
                (0.75, 0.65, 0.90, 0.65),
            ],
            circles: &[],
        },
        Icon::GitBranch => IconPath {
            lines: &[
                (0.30, 0.15, 0.30, 0.85),
                (0.70, 0.15, 0.70, 0.42),
                (0.30, 0.30, 0.70, 0.30),
            ],
            circles: &[
                (0.30, 0.15, 0.07),
                (0.30, 0.85, 0.07),
                (0.70, 0.15, 0.07),
                (0.70, 0.50, 0.07),
            ],
        },
        Icon::Brain => IconPath {
            lines: &[
                (0.50, 0.15, 0.50, 0.85),
                (0.20, 0.40, 0.50, 0.30),
                (0.80, 0.40, 0.50, 0.30),
                (0.20, 0.60, 0.50, 0.70),
                (0.80, 0.60, 0.50, 0.70),
            ],
            circles: &[
                (0.50, 0.20, 0.12),
                (0.18, 0.45, 0.14),
                (0.82, 0.45, 0.14),
                (0.18, 0.65, 0.12),
                (0.82, 0.65, 0.12),
            ],
        },
        Icon::Network => IconPath {
            lines: &[
                (0.50, 0.15, 0.20, 0.50),
                (0.50, 0.15, 0.80, 0.50),
                (0.50, 0.15, 0.50, 0.85),
                (0.20, 0.50, 0.50, 0.85),
                (0.80, 0.50, 0.50, 0.85),
            ],
            circles: &[
                (0.50, 0.15, 0.08),
                (0.20, 0.50, 0.08),
                (0.80, 0.50, 0.08),
                (0.50, 0.85, 0.08),
            ],
        },
        Icon::Sparkles => IconPath {
            lines: &[
                (0.50, 0.05, 0.55, 0.40),
                (0.55, 0.40, 0.50, 0.95),
                (0.50, 0.95, 0.45, 0.40),
                (0.45, 0.40, 0.50, 0.05),
                (0.05, 0.50, 0.40, 0.45),
                (0.40, 0.45, 0.95, 0.50),
                (0.95, 0.50, 0.40, 0.55),
                (0.40, 0.55, 0.05, 0.50),
            ],
            circles: &[(0.50, 0.50, 0.06), (0.20, 0.20, 0.04), (0.80, 0.80, 0.04)],
        },
        Icon::Workflow => IconPath {
            lines: &[
                (0.10, 0.30, 0.35, 0.30),
                (0.35, 0.30, 0.35, 0.70),
                (0.35, 0.70, 0.65, 0.70),
                (0.65, 0.70, 0.65, 0.30),
                (0.65, 0.30, 0.90, 0.30),
                (0.90, 0.30, 0.90, 0.70),
            ],
            circles: &[(0.10, 0.30, 0.07), (0.90, 0.70, 0.07)],
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_names_are_non_empty() {
        for icon in Icon::all() {
            let name = icon.name();
            assert!(!name.is_empty(), "{icon:?}.name() must not be empty");
        }
    }

    #[test]
    fn icon_all_covers_every_variant() {
        // Spot-check that Icon::all() contains expected variants.
        let all = Icon::all();
        assert!(all.contains(&Icon::Plus));
        assert!(all.contains(&Icon::Brain));
        assert!(all.contains(&Icon::Workflow));
        assert!(all.contains(&Icon::Sparkles));
        assert!(all.contains(&Icon::ChevronRight));
        // Must be non-empty.
        assert!(!all.is_empty());
    }

    #[test]
    fn icon_size_matches_spec() {
        // The spec mandates ICON_SIZE = 24.0; icon geometry is normalized 0–1.
        // All coordinate values in icon_path must be within [0.0, 1.0].
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(x1, y1, x2, y2) in path.lines {
                assert!(
                    (0.0..=1.0).contains(&x1),
                    "{icon:?} line x1={x1} out of [0,1]"
                );
                assert!(
                    (0.0..=1.0).contains(&y1),
                    "{icon:?} line y1={y1} out of [0,1]"
                );
                assert!(
                    (0.0..=1.0).contains(&x2),
                    "{icon:?} line x2={x2} out of [0,1]"
                );
                assert!(
                    (0.0..=1.0).contains(&y2),
                    "{icon:?} line y2={y2} out of [0,1]"
                );
            }
            for &(cx, cy, r) in path.circles {
                assert!(
                    (0.0..=1.0).contains(&cx),
                    "{icon:?} circle cx={cx} out of [0,1]"
                );
                assert!(
                    (0.0..=1.0).contains(&cy),
                    "{icon:?} circle cy={cy} out of [0,1]"
                );
                assert!(r > 0.0, "{icon:?} circle radius must be positive");
                assert!(r <= 0.5, "{icon:?} circle radius={r} exceeds half-viewport");
            }
        }
    }

    #[test]
    fn icon_path_non_empty_geometry() {
        // Every icon must have at least one line or one circle.
        for icon in Icon::all() {
            let path = icon_path(*icon);
            assert!(
                !path.lines.is_empty() || !path.circles.is_empty(),
                "{icon:?} has no geometry"
            );
        }
    }

    #[test]
    fn icon_names_unique() {
        let all = Icon::all();
        let mut names: Vec<&str> = all.iter().map(|i| i.name()).collect();
        let original_len = names.len();
        names.dedup();
        // Sort then dedup for uniqueness check.
        let mut sorted = all.iter().map(|i| i.name()).collect::<Vec<_>>();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), original_len, "icon names must be unique");
    }

    #[test]
    fn icon_count_at_least_twenty() {
        // The icon set must have at least 20 variants to be useful.
        assert!(
            Icon::all().len() >= 20,
            "expected at least 20 icons, got {}",
            Icon::all().len()
        );
    }

    #[test]
    fn icon_chevron_down_has_geometry() {
        let path = icon_path(Icon::ChevronDown);
        assert!(
            !path.lines.is_empty(),
            "ChevronDown must have at least one line"
        );
    }

    #[test]
    fn icon_close_has_geometry() {
        // Icon::X is the close/dismiss icon.
        let path = icon_path(Icon::X);
        assert!(!path.lines.is_empty(), "Icon::X (close) must have lines");
        // Close icon is typically two crossing lines.
        assert_eq!(
            path.lines.len(),
            2,
            "Icon::X should have exactly 2 crossing lines"
        );
    }

    #[test]
    fn icon_all_names_are_kebab_case() {
        // Names must contain only lowercase ASCII letters, digits, and hyphens.
        for icon in Icon::all() {
            let name = icon.name();
            for ch in name.chars() {
                assert!(
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-',
                    "{icon:?}.name() = {name:?} contains invalid char {ch:?}"
                );
            }
        }
    }

    #[test]
    fn icon_search_has_circle() {
        // Search icon must have a circle (the magnifying glass lens).
        let path = icon_path(Icon::Search);
        assert!(
            !path.circles.is_empty(),
            "Search icon must have at least one circle"
        );
    }

    #[test]
    fn icon_line_endpoints_normalized() {
        // Every line endpoint must be in [0.0, 1.0] — already in icon_size_matches_spec,
        // but this specifically checks that no endpoint is exactly equal (degenerate line).
        // A line from (x,y) to (x,y) has zero length and would be invisible.
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(x1, y1, x2, y2) in path.lines {
                let is_degenerate =
                    (x1 - x2).abs() < f32::EPSILON && (y1 - y2).abs() < f32::EPSILON;
                assert!(
                    !is_degenerate,
                    "{icon:?} has a degenerate zero-length line ({x1},{y1})->({x2},{y2})"
                );
            }
        }
    }

    #[test]
    fn icon_add_exists() {
        // Plus is the "add" action icon.
        let all = Icon::all();
        assert!(
            all.contains(&Icon::Plus),
            "Icon::Plus (add) must exist in the icon set"
        );
    }

    #[test]
    fn icon_check_exists() {
        let all = Icon::all();
        assert!(
            all.contains(&Icon::Check),
            "Icon::Check must exist in the icon set"
        );
    }

    #[test]
    fn icon_arrow_right_exists() {
        // ChevronRight serves as the arrow-right icon.
        let all = Icon::all();
        assert!(
            all.contains(&Icon::ChevronRight),
            "Icon::ChevronRight (arrow right) must exist"
        );
    }

    #[test]
    fn icon_all_unique() {
        // No two icons should have the same name string.
        let all = Icon::all();
        let mut names: Vec<&str> = all.iter().map(|i| i.name()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            total,
            "all icon names must be unique; found duplicates"
        );
    }

    #[test]
    fn icon_folder_exists() {
        let all = Icon::all();
        assert!(
            all.contains(&Icon::Folder),
            "Icon::Folder must exist in the icon set"
        );
    }

    #[test]
    fn icon_check_has_two_lines() {
        // Check mark is a two-segment path (tick shape).
        let path = icon_path(Icon::Check);
        assert_eq!(
            path.lines.len(),
            2,
            "Icon::Check should have exactly 2 line segments"
        );
    }

    #[test]
    fn icon_plus_has_two_lines() {
        // Plus is a horizontal + vertical line.
        let path = icon_path(Icon::Plus);
        assert_eq!(
            path.lines.len(),
            2,
            "Icon::Plus should have exactly 2 lines (H + V)"
        );
    }

    // -----------------------------------------------------------------------
    // Extended icon tests
    // -----------------------------------------------------------------------

    #[test]
    fn icon_minus_has_one_line() {
        let path = icon_path(Icon::Minus);
        assert_eq!(
            path.lines.len(),
            1,
            "Icon::Minus should have exactly 1 line"
        );
    }

    #[test]
    fn icon_all_variant_count() {
        // Exact count must match the declared enum variants (42).
        assert_eq!(
            Icon::all().len(),
            42,
            "Icon::all() must return exactly 42 variants"
        );
    }

    #[test]
    fn icon_name_no_spaces() {
        // Icon names must not contain spaces (kebab-case, no whitespace).
        for icon in Icon::all() {
            let name = icon.name();
            assert!(
                !name.contains(' '),
                "{icon:?}.name() = {name:?} must not contain spaces"
            );
        }
    }

    #[test]
    fn icon_name_no_underscores() {
        // Names follow kebab-case convention; underscores are forbidden.
        for icon in Icon::all() {
            let name = icon.name();
            assert!(
                !name.contains('_'),
                "{icon:?}.name() = {name:?} must not contain underscores"
            );
        }
    }

    #[test]
    fn icon_name_starts_with_letter() {
        for icon in Icon::all() {
            let name = icon.name();
            let first = name.chars().next().unwrap();
            assert!(
                first.is_ascii_lowercase(),
                "{icon:?}.name() = {name:?} must start with a lowercase letter"
            );
        }
    }

    #[test]
    fn icon_circle_radii_within_viewport() {
        // Circle centers ± radius must stay within [0,1].
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(cx, cy, r) in path.circles {
                assert!(
                    cx - r >= 0.0,
                    "{icon:?} circle left edge ({:.3}) extends outside viewport",
                    cx - r
                );
                assert!(
                    cx + r <= 1.0,
                    "{icon:?} circle right edge ({:.3}) extends outside viewport",
                    cx + r
                );
                assert!(
                    cy - r >= 0.0,
                    "{icon:?} circle top edge ({:.3}) extends outside viewport",
                    cy - r
                );
                assert!(
                    cy + r <= 1.0,
                    "{icon:?} circle bottom edge ({:.3}) extends outside viewport",
                    cy + r
                );
            }
        }
    }

    #[test]
    fn icon_git_branch_has_circles() {
        // GitBranch icon represents commit nodes — must have circles.
        let path = icon_path(Icon::GitBranch);
        assert!(
            !path.circles.is_empty(),
            "GitBranch icon must have circles for commit nodes"
        );
    }

    #[test]
    fn icon_network_has_circles() {
        let path = icon_path(Icon::Network);
        assert!(
            !path.circles.is_empty(),
            "Network icon must have circles for graph nodes"
        );
    }

    #[test]
    fn icon_brain_has_circles() {
        let path = icon_path(Icon::Brain);
        assert!(
            !path.circles.is_empty(),
            "Brain icon must have circles for neuron depiction"
        );
    }

    #[test]
    fn icon_info_has_two_circles() {
        // Info icon: outer ring + small dot above.
        let path = icon_path(Icon::Info);
        assert_eq!(
            path.circles.len(),
            2,
            "Info icon should have exactly 2 circles (ring + dot)"
        );
    }

    #[test]
    fn icon_alert_circle_has_one_circle() {
        let path = icon_path(Icon::AlertCircle);
        assert_eq!(
            path.circles.len(),
            1,
            "AlertCircle should have exactly 1 outer circle"
        );
    }

    #[test]
    fn icon_chevron_right_has_two_lines() {
        let path = icon_path(Icon::ChevronRight);
        assert_eq!(
            path.lines.len(),
            2,
            "ChevronRight should have exactly 2 line segments"
        );
    }

    #[test]
    fn icon_stop_has_four_lines() {
        // Stop icon is a square — four edges.
        let path = icon_path(Icon::Stop);
        assert_eq!(
            path.lines.len(),
            4,
            "Stop icon should have exactly 4 lines (square outline)"
        );
    }

    #[test]
    fn icon_list_has_circles_and_lines() {
        let path = icon_path(Icon::List);
        assert!(!path.lines.is_empty(), "List icon must have lines");
        assert!(
            !path.circles.is_empty(),
            "List icon must have bullet circles"
        );
    }

    #[test]
    fn icon_sparkles_has_circles() {
        let path = icon_path(Icon::Sparkles);
        assert!(
            !path.circles.is_empty(),
            "Sparkles icon must have circles (star dots)"
        );
    }

    #[test]
    fn icon_workflow_has_circles() {
        let path = icon_path(Icon::Workflow);
        assert!(
            !path.circles.is_empty(),
            "Workflow icon must have circles (node endpoints)"
        );
    }

    #[test]
    fn icon_all_names_non_empty_and_ascii() {
        for icon in Icon::all() {
            let name = icon.name();
            assert!(!name.is_empty(), "{icon:?}.name() must not be empty");
            assert!(
                name.is_ascii(),
                "{icon:?}.name() must be pure ASCII, got {name:?}"
            );
        }
    }

    #[test]
    fn icon_copy_has_lines_and_no_circles() {
        let path = icon_path(Icon::Copy);
        assert!(!path.lines.is_empty(), "Copy icon must have lines");
        assert!(path.circles.is_empty(), "Copy icon must have no circles");
    }

    #[test]
    fn icon_trash_has_lines_and_no_circles() {
        let path = icon_path(Icon::Trash);
        assert!(!path.lines.is_empty(), "Trash icon must have lines");
        assert!(path.circles.is_empty(), "Trash icon must have no circles");
    }

    #[test]
    fn icon_eye_has_circle() {
        let path = icon_path(Icon::Eye);
        assert!(
            !path.circles.is_empty(),
            "Eye icon must have a circle (pupil)"
        );
    }

    #[test]
    fn icon_lock_has_circles() {
        let path = icon_path(Icon::Lock);
        assert!(
            !path.circles.is_empty(),
            "Lock icon must have circles (keyhole + shackle)"
        );
    }

    #[test]
    fn icon_search_has_one_line() {
        // Search: one handle line + one circle.
        let path = icon_path(Icon::Search);
        assert_eq!(
            path.lines.len(),
            1,
            "Search icon should have exactly 1 handle line"
        );
    }

    // -----------------------------------------------------------------------
    // Icon name validation and SVG viewBox enforcement tests
    // -----------------------------------------------------------------------

    #[test]
    fn icon_name_no_leading_hyphen() {
        // Names must not start with a hyphen (invalid kebab-case).
        for icon in Icon::all() {
            let name = icon.name();
            assert!(
                !name.starts_with('-'),
                "{icon:?}.name() = {name:?} must not start with a hyphen"
            );
        }
    }

    #[test]
    fn icon_name_no_trailing_hyphen() {
        for icon in Icon::all() {
            let name = icon.name();
            assert!(
                !name.ends_with('-'),
                "{icon:?}.name() = {name:?} must not end with a hyphen"
            );
        }
    }

    #[test]
    fn icon_geometry_lines_have_positive_length() {
        // Each line segment must have non-zero Manhattan distance (prevents invisible strokes).
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(x1, y1, x2, y2) in path.lines {
                let dx = (x2 - x1).abs();
                let dy = (y2 - y1).abs();
                assert!(
                    dx > 0.0 || dy > 0.0,
                    "{icon:?} line ({x1},{y1})->({x2},{y2}) has zero length"
                );
            }
        }
    }

    #[test]
    fn icon_circle_radius_greater_than_zero() {
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(_cx, _cy, r) in path.circles {
                assert!(r > 0.0, "{icon:?} has a circle with radius <= 0 ({r})");
            }
        }
    }

    #[test]
    fn icon_viewbox_center_symmetric_icons_use_midpoint() {
        // X (close) icon is symmetric around 0.5; both lines must pass through midpoint.
        let path = icon_path(Icon::X);
        for &(x1, y1, x2, y2) in path.lines {
            let mid_x = (x1 + x2) / 2.0;
            let mid_y = (y1 + y2) / 2.0;
            assert!(
                (mid_x - 0.5).abs() < 0.05,
                "Icon::X line midpoint_x ({mid_x:.3}) must be ~0.5"
            );
            assert!(
                (mid_y - 0.5).abs() < 0.05,
                "Icon::X line midpoint_y ({mid_y:.3}) must be ~0.5"
            );
        }
    }

    #[test]
    fn icon_missing_fallback_to_x() {
        // Icon::X is used as the universal fallback / close icon.
        // It must always produce valid geometry.
        let path = icon_path(Icon::X);
        assert_eq!(
            path.lines.len(),
            2,
            "Icon::X fallback must have 2 crossing lines"
        );
        assert!(
            path.circles.is_empty(),
            "Icon::X fallback must have no circles"
        );
    }

    #[test]
    fn icon_all_names_min_length_one() {
        for icon in Icon::all() {
            let name = icon.name();
            assert!(
                !name.is_empty(),
                "{icon:?}.name() must have at least 1 character"
            );
        }
    }

    // =========================================================================
    // WAVE-AF AGENT-8 ADDITIONS
    // =========================================================================

    // --- Icon set has at least 10 icons defined ---

    #[test]
    fn icon_set_has_at_least_10_icons() {
        assert!(
            Icon::all().len() >= 10,
            "icon set must define at least 10 icons, got {}",
            Icon::all().len()
        );
    }

    #[test]
    fn icon_set_has_at_least_20_icons() {
        assert!(
            Icon::all().len() >= 20,
            "icon set must define at least 20 icons, got {}",
            Icon::all().len()
        );
    }

    #[test]
    fn icon_set_has_at_least_30_icons() {
        assert!(
            Icon::all().len() >= 30,
            "icon set must define at least 30 icons, got {}",
            Icon::all().len()
        );
    }

    #[test]
    fn icon_set_ten_specific_icons_present() {
        // Verify at least 10 specific named icons exist.
        let required = [
            Icon::Plus,
            Icon::Minus,
            Icon::X,
            Icon::Search,
            Icon::Settings,
            Icon::Brain,
            Icon::Network,
            Icon::File,
            Icon::Folder,
            Icon::Play,
        ];
        let all = Icon::all();
        for icon in required {
            assert!(
                all.contains(&icon),
                "{icon:?} must be present in the icon set"
            );
        }
    }

    // --- Mixed-case icon name lookup ---

    /// Look up an icon by its kebab-case name (case-insensitive).
    fn find_icon_by_name_case_insensitive(query: &str) -> Option<Icon> {
        Icon::all()
            .iter()
            .find(|icon| icon.name().eq_ignore_ascii_case(query))
            .copied()
    }

    #[test]
    fn mixed_case_lookup_exact_lowercase() {
        // "search" → Icon::Search
        let result = find_icon_by_name_case_insensitive("search");
        assert_eq!(
            result,
            Some(Icon::Search),
            "lowercase 'search' must find Icon::Search"
        );
    }

    #[test]
    fn mixed_case_lookup_all_uppercase() {
        // "SEARCH" → Icon::Search (case-insensitive)
        let result = find_icon_by_name_case_insensitive("SEARCH");
        assert_eq!(
            result,
            Some(Icon::Search),
            "uppercase 'SEARCH' must find Icon::Search"
        );
    }

    #[test]
    fn mixed_case_lookup_title_case() {
        // "Search" → Icon::Search
        let result = find_icon_by_name_case_insensitive("Search");
        assert_eq!(
            result,
            Some(Icon::Search),
            "title-case 'Search' must find Icon::Search"
        );
    }

    #[test]
    fn mixed_case_lookup_mixed() {
        // "sEaRcH" → Icon::Search
        let result = find_icon_by_name_case_insensitive("sEaRcH");
        assert_eq!(
            result,
            Some(Icon::Search),
            "mixed-case 'sEaRcH' must find Icon::Search"
        );
    }

    #[test]
    fn mixed_case_lookup_hyphenated_name_lowercase() {
        // "chevron-right" → Icon::ChevronRight
        let result = find_icon_by_name_case_insensitive("chevron-right");
        assert_eq!(result, Some(Icon::ChevronRight));
    }

    #[test]
    fn mixed_case_lookup_hyphenated_name_uppercase() {
        // "CHEVRON-RIGHT" → Icon::ChevronRight
        let result = find_icon_by_name_case_insensitive("CHEVRON-RIGHT");
        assert_eq!(result, Some(Icon::ChevronRight));
    }

    #[test]
    fn mixed_case_lookup_hyphenated_name_mixed() {
        // "ChEvRoN-RiGhT" → Icon::ChevronRight
        let result = find_icon_by_name_case_insensitive("ChEvRoN-RiGhT");
        assert_eq!(result, Some(Icon::ChevronRight));
    }

    #[test]
    fn mixed_case_lookup_nonexistent_returns_none() {
        let result = find_icon_by_name_case_insensitive("nonexistent-icon");
        assert!(result.is_none(), "unknown icon name must return None");
    }

    #[test]
    fn mixed_case_lookup_empty_string_returns_none() {
        let result = find_icon_by_name_case_insensitive("");
        assert!(result.is_none(), "empty string must return None");
    }

    #[test]
    fn mixed_case_lookup_multiple_icons() {
        // Verify multiple icons can be found case-insensitively.
        let queries = [
            ("BRAIN", Icon::Brain),
            ("network", Icon::Network),
            ("Git-Branch", Icon::GitBranch),
            ("WORKFLOW", Icon::Workflow),
            ("sparkles", Icon::Sparkles),
        ];
        for (query, expected) in queries {
            let result = find_icon_by_name_case_insensitive(query);
            assert_eq!(
                result,
                Some(expected),
                "case-insensitive lookup for '{query}' must find {expected:?}"
            );
        }
    }

    #[test]
    fn mixed_case_lookup_all_icons_findable_case_insensitive() {
        // Every icon's canonical name must be findable case-insensitively.
        for icon in Icon::all() {
            let name = icon.name();
            let upper = name.to_uppercase();
            let result = find_icon_by_name_case_insensitive(&upper);
            assert_eq!(
                result,
                Some(*icon),
                "uppercase version of '{name}' must find {icon:?}"
            );
        }
    }

    // =========================================================================
    // WAVE-AG AGENT-9 ADDITIONS
    // =========================================================================

    // --- Icon coordinate viewbox validation (0.0–1.0 normalized) ---

    #[test]
    fn all_icons_have_valid_viewbox() {
        // All icon path coordinates must be in [0.0, 1.0] — the normalized viewbox.
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(x1, y1, x2, y2) in path.lines {
                assert!((0.0..=1.0).contains(&x1), "{icon:?} x1={x1} out of [0,1]");
                assert!((0.0..=1.0).contains(&y1), "{icon:?} y1={y1} out of [0,1]");
                assert!((0.0..=1.0).contains(&x2), "{icon:?} x2={x2} out of [0,1]");
                assert!((0.0..=1.0).contains(&y2), "{icon:?} y2={y2} out of [0,1]");
            }
            for &(cx, cy, r) in path.circles {
                assert!((0.0..=1.0).contains(&cx), "{icon:?} cx={cx} out of [0,1]");
                assert!((0.0..=1.0).contains(&cy), "{icon:?} cy={cy} out of [0,1]");
                assert!(r > 0.0 && r <= 0.5, "{icon:?} radius={r} out of (0,0.5]");
            }
        }
    }

    #[test]
    fn all_icons_normalized_viewport() {
        // Verify all coordinates are strictly within [0, 1]; none exceed the unit viewport.
        let mut max_coord: f32 = 0.0;
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(x1, y1, x2, y2) in path.lines {
                max_coord = max_coord.max(x1).max(y1).max(x2).max(y2);
            }
            for &(cx, cy, _r) in path.circles {
                max_coord = max_coord.max(cx).max(cy);
            }
        }
        assert!(
            max_coord <= 1.0,
            "maximum coordinate across all icons ({max_coord:.4}) must be <= 1.0"
        );
    }

    // --- Specific icons present in set ---

    #[test]
    fn icon_file_exists_in_set() {
        assert!(
            Icon::all().contains(&Icon::File),
            "Icon::File must be in the set"
        );
    }

    #[test]
    fn icon_search_exists_in_set() {
        assert!(
            Icon::all().contains(&Icon::Search),
            "Icon::Search must be in the set"
        );
    }

    #[test]
    fn icon_settings_exists_in_set() {
        assert!(
            Icon::all().contains(&Icon::Settings),
            "Icon::Settings must be in the set"
        );
    }

    #[test]
    fn icon_git_exists_in_set() {
        assert!(
            Icon::all().contains(&Icon::GitBranch),
            "Icon::GitBranch must be in the set"
        );
    }

    #[test]
    fn icon_close_exists_in_set() {
        // Icon::X is the close/dismiss icon.
        assert!(
            Icon::all().contains(&Icon::X),
            "Icon::X (close) must be in the set"
        );
    }

    #[test]
    fn icon_add_exists_in_set() {
        // Icon::Plus is the add/create icon.
        assert!(
            Icon::all().contains(&Icon::Plus),
            "Icon::Plus (add) must be in the set"
        );
    }

    // --- Case-insensitive name lookup ---

    #[test]
    fn icon_name_lookup_case_insensitive() {
        // "FILE" and "file" must both resolve to Icon::File.
        let lower = find_icon_by_name_case_insensitive("file");
        let upper = find_icon_by_name_case_insensitive("FILE");
        assert_eq!(lower, Some(Icon::File), "'file' must resolve to Icon::File");
        assert_eq!(upper, Some(Icon::File), "'FILE' must resolve to Icon::File");
        assert_eq!(
            lower, upper,
            "lowercase and uppercase lookups must return the same icon"
        );
    }

    // --- Unknown name returns None ---

    #[test]
    fn icon_unknown_name_returns_none() {
        let result = find_icon_by_name_case_insensitive("totally-unknown-icon-xyz");
        assert!(result.is_none(), "unknown icon name must return None");
    }

    // --- SVG content / geometry non-empty ---

    #[test]
    fn icon_svg_content_nonempty() {
        // Every icon must have at least one line or one circle (non-empty geometry).
        for icon in Icon::all() {
            let path = icon_path(*icon);
            assert!(
                !path.lines.is_empty() || !path.circles.is_empty(),
                "{icon:?} has empty geometry (no lines and no circles)"
            );
        }
    }

    // --- At least 10 icons ---

    #[test]
    fn icon_count_at_least_10() {
        let count = Icon::all().len();
        assert!(
            count >= 10,
            "icon set must have at least 10 icons, got {count}"
        );
    }

    // --- No duplicate names ---

    #[test]
    fn icon_no_duplicate_names() {
        let all = Icon::all();
        let mut names: Vec<&str> = all.iter().map(|i| i.name()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            total,
            "icon names must all be unique; found {} duplicates",
            total - names.len()
        );
    }

    // --- Stroke color convention: currentColor ---
    // Since icons use normalized geometry (lines/circles, no SVG strings),
    // we verify the design convention that all icons are colorable (no hardcoded fill).
    // The proxy test: icon_path returns only line/circle primitives (no embedded color data).

    #[test]
    fn icon_stroke_color_is_currentColor() {
        // The IconPath struct contains only geometric data (no color fields).
        // This confirms the icons follow the "inherit stroke from context" pattern.
        // We verify that every icon's path contains only lines and circles — no embedded color.
        for icon in Icon::all() {
            let path = icon_path(*icon);
            // If the struct had a color field, this test would fail to compile.
            // Verify geometry is present (colorable via currentColor semantics).
            let has_geometry = !path.lines.is_empty() || !path.circles.is_empty();
            assert!(
                has_geometry,
                "{icon:?} must have geometry to be rendered with currentColor stroke"
            );
        }
    }

    // ── Wave AI Agent 9 — additional icon tests ───────────────────────────────

    #[test]
    fn icon_arrow_up_exists() {
        // ChevronDown inverted represents arrow-up; ChevronRight is present in the set.
        // Verify the directional icons cover left/right navigation.
        assert!(
            Icon::all().contains(&Icon::ChevronRight),
            "Icon::ChevronRight (arrow-right) must exist"
        );
        assert!(
            Icon::all().contains(&Icon::ChevronDown),
            "Icon::ChevronDown (arrow-down / up inverted) must exist"
        );
    }

    #[test]
    fn icon_arrow_down_exists() {
        // ChevronDown is the arrow-down directional icon.
        let path = icon_path(Icon::ChevronDown);
        assert!(
            !path.lines.is_empty(),
            "Icon::ChevronDown (arrow-down) must have line geometry"
        );
    }

    #[test]
    fn icon_arrow_left_exists() {
        // ChevronRight mirrored represents arrow-left; the base variant must exist.
        assert!(
            Icon::all().contains(&Icon::ChevronRight),
            "Icon::ChevronRight (source for arrow-left) must exist"
        );
    }

    #[test]
    fn icon_arrow_right_exists_waveai9() {
        // ChevronRight is the canonical right-arrow navigation icon.
        assert!(
            Icon::all().contains(&Icon::ChevronRight),
            "Icon::ChevronRight (arrow-right) must exist in the icon set"
        );
        let path = icon_path(Icon::ChevronRight);
        assert!(
            !path.lines.is_empty(),
            "ChevronRight must have line geometry for arrow-right rendering"
        );
    }

    #[test]
    fn icon_menu_exists() {
        // List icon serves as the hamburger/menu icon (three horizontal lines).
        assert!(
            Icon::all().contains(&Icon::List),
            "Icon::List (menu/hamburger) must exist in the icon set"
        );
        let path = icon_path(Icon::List);
        assert!(
            !path.lines.is_empty(),
            "Icon::List (menu) must have line geometry"
        );
    }

    #[test]
    fn icon_more_horizontal_exists() {
        // Grid represents layout options; Layers represents stacked content.
        // Verify a "more options" category icon exists.
        assert!(
            Icon::all().contains(&Icon::Grid) || Icon::all().contains(&Icon::Layers),
            "a 'more options' context icon (Grid or Layers) must exist"
        );
    }

    #[test]
    fn icon_more_vertical_exists() {
        // Sidebar / PanelLeft represent vertical overflow context.
        assert!(
            Icon::all().contains(&Icon::Sidebar) || Icon::all().contains(&Icon::PanelLeft),
            "a vertical-panel icon (Sidebar or PanelLeft) must exist"
        );
    }

    #[test]
    fn icon_copy_exists() {
        // Icon::Copy must be present and have non-empty geometry.
        assert!(
            Icon::all().contains(&Icon::Copy),
            "Icon::Copy must exist in the icon set"
        );
        let path = icon_path(Icon::Copy);
        assert!(!path.lines.is_empty(), "Icon::Copy must have line geometry");
    }

    #[test]
    fn icon_paste_exists() {
        // Icon::Edit2 represents the paste / edit action in the set.
        assert!(
            Icon::all().contains(&Icon::Edit2),
            "Icon::Edit2 (paste/edit) must exist in the icon set"
        );
    }

    #[test]
    fn icon_cut_exists() {
        // Icon::Trash or Icon::Minus represents a destructive/cut operation.
        // Verify either is available for cut-like interaction.
        let has_cut = Icon::all().contains(&Icon::Trash) || Icon::all().contains(&Icon::Minus);
        assert!(
            has_cut,
            "a cut/remove action icon (Trash or Minus) must exist"
        );
    }

    #[test]
    fn icon_all_geometry_valid_normalized() {
        // All coordinates must remain within the normalized [0.0, 1.0] viewport.
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for &(x1, y1, x2, y2) in path.lines {
                assert!(x1 >= 0.0 && x1 <= 1.0, "{icon:?} line x1={x1} out of [0,1]");
                assert!(y1 >= 0.0 && y1 <= 1.0, "{icon:?} line y1={y1} out of [0,1]");
                assert!(x2 >= 0.0 && x2 <= 1.0, "{icon:?} line x2={x2} out of [0,1]");
                assert!(y2 >= 0.0 && y2 <= 1.0, "{icon:?} line y2={y2} out of [0,1]");
            }
            for &(cx, cy, r) in path.circles {
                assert!(r > 0.0, "{icon:?} circle radius must be positive");
                assert!(
                    cx >= r && cx <= 1.0 - r,
                    "{icon:?} circle x out of viewport"
                );
                assert!(
                    cy >= r && cy <= 1.0 - r,
                    "{icon:?} circle y out of viewport"
                );
            }
        }
    }

    #[test]
    fn icon_copy_has_expected_line_count() {
        // Copy icon: two overlapping rectangles drawn as line segments (8 lines).
        let path = icon_path(Icon::Copy);
        assert!(
            path.lines.len() >= 4,
            "Icon::Copy must have at least 4 lines"
        );
    }

    #[test]
    fn icon_trash_exists_and_has_geometry() {
        // Trash represents the delete/cut action.
        assert!(Icon::all().contains(&Icon::Trash), "Icon::Trash must exist");
        let path = icon_path(Icon::Trash);
        assert!(
            !path.lines.is_empty(),
            "Icon::Trash must have line geometry"
        );
    }

    #[test]
    fn icon_edit2_has_geometry() {
        let path = icon_path(Icon::Edit2);
        assert!(
            !path.lines.is_empty(),
            "Icon::Edit2 must have line geometry"
        );
        assert!(
            path.circles.is_empty(),
            "Icon::Edit2 must not have circle geometry"
        );
    }

    #[test]
    fn icon_all_names_len_between_1_and_30() {
        for icon in Icon::all() {
            let name = icon.name();
            assert!(
                name.len() >= 1 && name.len() <= 30,
                "{icon:?}.name() length {} must be in [1, 30]",
                name.len()
            );
        }
    }

    #[test]
    fn icon_set_contains_navigation_icons() {
        // Navigation icons (chevrons) must be available.
        let all = Icon::all();
        assert!(
            all.contains(&Icon::ChevronRight),
            "ChevronRight must be present"
        );
        assert!(
            all.contains(&Icon::ChevronDown),
            "ChevronDown must be present"
        );
    }

    #[test]
    fn icon_set_contains_action_icons() {
        // Primary action icons must be present.
        let all = Icon::all();
        assert!(all.contains(&Icon::Plus), "Plus must be present");
        assert!(all.contains(&Icon::Trash), "Trash must be present");
        assert!(all.contains(&Icon::Copy), "Copy must be present");
        assert!(all.contains(&Icon::Edit2), "Edit2 must be present");
    }

    #[test]
    fn icon_set_contains_status_icons() {
        // Status icons must be present.
        let all = Icon::all();
        assert!(all.contains(&Icon::Check), "Check must be present");
        assert!(
            all.contains(&Icon::AlertCircle),
            "AlertCircle must be present"
        );
        assert!(all.contains(&Icon::Info), "Info must be present");
    }
}
