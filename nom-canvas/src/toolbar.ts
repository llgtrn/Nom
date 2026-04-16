export interface ToolbarButton {
  id: string;
  label: string;
  icon: string; // text icon (no emojis per project rules — use text abbreviations)
  tooltip: string;
  group: string;
  action: () => void;
}

export class Toolbar {
  private container: HTMLElement;
  private buttons: Map<string, HTMLButtonElement> = new Map();

  constructor(containerId: string) {
    this.container = document.getElementById(containerId)!;
    this.container.className = "nom-toolbar";
  }

  addButton(btn: ToolbarButton): void {
    // Find or create group
    let group = this.container.querySelector(`[data-group="${btn.group}"]`) as HTMLElement;
    if (!group) {
      group = document.createElement("div");
      group.className = "toolbar-group";
      group.dataset.group = btn.group;
      this.container.appendChild(group);
    }

    const el = document.createElement("button");
    el.className = "toolbar-btn";
    el.id = `tb-${btn.id}`;
    el.title = btn.tooltip;
    el.textContent = btn.icon;
    el.addEventListener("click", btn.action);
    group.appendChild(el);
    this.buttons.set(btn.id, el);
  }

  addSeparator(): void {
    const sep = document.createElement("div");
    sep.className = "toolbar-separator";
    this.container.appendChild(sep);
  }

  /** Add a group of buttons */
  addGroup(groupName: string, buttons: ToolbarButton[]): void {
    for (const btn of buttons) {
      this.addButton({ ...btn, group: groupName });
    }
  }

  /** Set button active state */
  setActive(id: string, active: boolean): void {
    const btn = this.buttons.get(id);
    if (btn) btn.classList.toggle("active", active);
  }

  /** Set button disabled state */
  setDisabled(id: string, disabled: boolean): void {
    const btn = this.buttons.get(id);
    if (btn) btn.disabled = disabled;
  }

  /** Add a label/text element */
  addLabel(text: string): void {
    const label = document.createElement("span");
    label.className = "toolbar-label";
    label.textContent = text;
    this.container.appendChild(label);
  }

  /** Add a spacer that pushes subsequent items to the right */
  addSpacer(): void {
    const spacer = document.createElement("div");
    spacer.className = "toolbar-spacer";
    this.container.appendChild(spacer);
  }

  /** Get default NomCanvas toolbar buttons */
  static defaultButtons(handlers: Record<string, () => void>): ToolbarButton[] {
    return [
      // Block creation
      { id: "new-prose", label: "Prose", icon: "P", tooltip: "New Prose Block", group: "create", action: handlers.newProse || (() => {}) },
      { id: "new-nomx", label: "NomX", icon: "N", tooltip: "New .nomx Block", group: "create", action: handlers.newNomx || (() => {}) },
      { id: "new-code", label: "Code", icon: "<>", tooltip: "New Code Block", group: "create", action: handlers.newCode || (() => {}) },
      { id: "new-graph", label: "Graph", icon: "G", tooltip: "New Graph Block", group: "create", action: handlers.newGraph || (() => {}) },

      // Execution
      { id: "compile", label: "Compile", icon: "[>", tooltip: "Compile Block (Ctrl+Enter)", group: "execute", action: handlers.compile || (() => {}) },
      { id: "run-graph", label: "Run", icon: ">>", tooltip: "Execute Graph (Ctrl+Shift+Enter)", group: "execute", action: handlers.runGraph || (() => {}) },
      { id: "score", label: "Score", icon: "Q", tooltip: "Quality Score", group: "execute", action: handlers.score || (() => {}) },
      { id: "scan", label: "Scan", icon: "S!", tooltip: "Security Scan", group: "execute", action: handlers.scan || (() => {}) },

      // View
      { id: "zoom-in", label: "Zoom In", icon: "+", tooltip: "Zoom In (Ctrl++)", group: "view", action: handlers.zoomIn || (() => {}) },
      { id: "zoom-out", label: "Zoom Out", icon: "-", tooltip: "Zoom Out (Ctrl+-)", group: "view", action: handlers.zoomOut || (() => {}) },
      { id: "fit", label: "Fit", icon: "[]", tooltip: "Fit to View (Ctrl+0)", group: "view", action: handlers.fit || (() => {}) },

      // Panels
      { id: "library", label: "Library", icon: "LB", tooltip: "Toggle Library (Ctrl+L)", group: "panels", action: handlers.library || (() => {}) },
      { id: "palette", label: "Palette", icon: "Cmd", tooltip: "Command Palette (Ctrl+K)", group: "panels", action: handlers.palette || (() => {}) },
      { id: "minimap", label: "Map", icon: "M", tooltip: "Toggle Minimap", group: "panels", action: handlers.minimap || (() => {}) },
    ];
  }
}
