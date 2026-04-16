export interface Shortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: string;
  label: string;
}

export type ShortcutHandler = (action: string) => void;

const DEFAULT_SHORTCUTS: Shortcut[] = [
  // Canvas navigation
  { key: "0", ctrl: true, action: "reset_zoom", label: "Reset Zoom" },
  { key: "+", ctrl: true, action: "zoom_in", label: "Zoom In" },
  { key: "-", ctrl: true, action: "zoom_out", label: "Zoom Out" },
  { key: "f", ctrl: true, action: "fit_view", label: "Fit to View" },

  // Block operations
  { key: "n", ctrl: true, action: "new_block", label: "New Block" },
  { key: "d", ctrl: true, action: "duplicate_block", label: "Duplicate Block" },
  { key: "Delete", action: "delete_block", label: "Delete Block" },
  { key: "Backspace", action: "delete_block", label: "Delete Block" },

  // Selection
  { key: "a", ctrl: true, action: "select_all", label: "Select All" },
  { key: "Escape", action: "deselect_all", label: "Deselect" },

  // Execution
  { key: "Enter", ctrl: true, action: "compile_block", label: "Compile Block" },
  { key: "Enter", ctrl: true, shift: true, action: "execute_graph", label: "Execute Graph" },
  { key: "r", ctrl: true, action: "run_action", label: "Run Action" },

  // Panels
  { key: "l", ctrl: true, action: "toggle_library", label: "Toggle Library" },
  { key: "q", ctrl: true, action: "toggle_quality", label: "Toggle Quality" },
  { key: "e", ctrl: true, action: "toggle_engine", label: "Toggle Engine Panel" },

  // File
  { key: "s", ctrl: true, action: "save_project", label: "Save Project" },
  { key: "o", ctrl: true, action: "open_project", label: "Open Project" },

  // Alignment
  { key: "ArrowLeft", ctrl: true, shift: true, action: "align_left", label: "Align Left" },
  { key: "ArrowRight", ctrl: true, shift: true, action: "align_right", label: "Align Right" },
  { key: "ArrowUp", ctrl: true, shift: true, action: "align_top", label: "Align Top" },
  { key: "ArrowDown", ctrl: true, shift: true, action: "align_bottom", label: "Align Bottom" },
];

export class ShortcutManager {
  private shortcuts: Shortcut[];
  private handlers: Map<string, ShortcutHandler[]> = new Map();
  private enabled = true;

  constructor(customShortcuts?: Shortcut[]) {
    this.shortcuts = customShortcuts || [...DEFAULT_SHORTCUTS];
    this.setupListener();
  }

  on(action: string, handler: ShortcutHandler): void {
    const existing = this.handlers.get(action) || [];
    existing.push(handler);
    this.handlers.set(action, existing);
  }

  off(action: string, handler: ShortcutHandler): void {
    const existing = this.handlers.get(action) || [];
    this.handlers.set(action, existing.filter(h => h !== handler));
  }

  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
  }

  getShortcuts(): Shortcut[] {
    return this.shortcuts;
  }

  /** Format shortcut for display (e.g., "Ctrl+Shift+Enter") */
  formatShortcut(shortcut: Shortcut): string {
    const parts: string[] = [];
    if (shortcut.ctrl) parts.push("Ctrl");
    if (shortcut.shift) parts.push("Shift");
    if (shortcut.alt) parts.push("Alt");
    parts.push(shortcut.key.length === 1 ? shortcut.key.toUpperCase() : shortcut.key);
    return parts.join("+");
  }

  private setupListener(): void {
    document.addEventListener("keydown", (e) => {
      if (!this.enabled) return;
      // Don't capture when typing in input/textarea
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      // Allow Prosemirror to handle its own shortcuts
      if (target.closest(".ProseMirror") && !e.ctrlKey && !e.metaKey) return;

      for (const shortcut of this.shortcuts) {
        if (this.matches(e, shortcut)) {
          e.preventDefault();
          e.stopPropagation();
          const handlers = this.handlers.get(shortcut.action) || [];
          for (const handler of handlers) handler(shortcut.action);
          break;
        }
      }
    });
  }

  private matches(e: KeyboardEvent, s: Shortcut): boolean {
    return (
      e.key === s.key &&
      !!(e.ctrlKey || e.metaKey) === !!s.ctrl &&
      !!e.shiftKey === !!s.shift &&
      !!e.altKey === !!s.alt
    );
  }
}

export function getDefaultShortcuts(): Shortcut[] {
  return [...DEFAULT_SHORTCUTS];
}
