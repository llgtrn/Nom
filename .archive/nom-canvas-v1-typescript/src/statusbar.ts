export interface StatusBarState {
  zoom: number;
  blockCount: number;
  selectedCount: number;
  cursorX: number;
  cursorY: number;
  compileStatus: "idle" | "compiling" | "success" | "error";
  projectName: string | null;
  dirtyFiles: number;
}

export class StatusBar {
  private container: HTMLElement;
  private sections: Map<string, HTMLElement> = new Map();

  constructor() {
    this.container = document.createElement("footer");
    this.container.className = "status-bar";
    this.container.id = "status-bar";

    // Left section
    const left = document.createElement("div");
    left.className = "status-section status-left";
    this.container.appendChild(left);

    // Center section
    const center = document.createElement("div");
    center.className = "status-section status-center";
    this.container.appendChild(center);

    // Right section
    const right = document.createElement("div");
    right.className = "status-section status-right";
    this.container.appendChild(right);

    // Items
    this.addItem("project", left, "NomCanvas");
    this.addItem("dirty", left, "");
    this.addItem("compile", center, "idle");
    this.addItem("blocks", center, "0 blocks");
    this.addItem("cursor", right, "0, 0");
    this.addItem("zoom", right, "100%");

    document.getElementById("app")?.appendChild(this.container);
  }

  private addItem(id: string, parent: HTMLElement, text: string): void {
    const el = document.createElement("span");
    el.className = `status-item status-${id}`;
    el.textContent = text;
    parent.appendChild(el);
    this.sections.set(id, el);
  }

  update(state: Partial<StatusBarState>): void {
    if (state.zoom !== undefined) {
      this.setText("zoom", `${(state.zoom * 100).toFixed(0)}%`);
    }
    if (state.blockCount !== undefined) {
      const sel = state.selectedCount ?? 0;
      this.setText("blocks", sel > 0 ? `${sel}/${state.blockCount} selected` : `${state.blockCount} blocks`);
    }
    if (state.cursorX !== undefined && state.cursorY !== undefined) {
      this.setText("cursor", `${state.cursorX.toFixed(0)}, ${state.cursorY.toFixed(0)}`);
    }
    if (state.compileStatus !== undefined) {
      const el = this.sections.get("compile");
      if (el) {
        el.textContent = state.compileStatus;
        el.className = `status-item status-compile compile-${state.compileStatus}`;
      }
    }
    if (state.projectName !== undefined) {
      this.setText("project", state.projectName || "NomCanvas");
    }
    if (state.dirtyFiles !== undefined) {
      this.setText("dirty", state.dirtyFiles > 0 ? `${state.dirtyFiles} unsaved` : "");
    }
  }

  private setText(id: string, text: string): void {
    const el = this.sections.get(id);
    if (el) el.textContent = text;
  }

  getElement(): HTMLElement { return this.container; }
}
