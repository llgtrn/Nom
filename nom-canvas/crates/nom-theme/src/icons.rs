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
            lines: &[(0.25, 0.15, 0.25, 0.85), (0.25, 0.15, 0.80, 0.50), (0.80, 0.50, 0.25, 0.85)],
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
            lines: &[(0.60, 0.05, 0.30, 0.50), (0.30, 0.50, 0.55, 0.50), (0.55, 0.50, 0.25, 0.95)],
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
            circles: &[(0.30, 0.15, 0.07), (0.30, 0.85, 0.07), (0.70, 0.15, 0.07), (0.70, 0.50, 0.07)],
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
