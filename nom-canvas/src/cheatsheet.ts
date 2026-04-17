import type { Shortcut } from "./shortcuts";
import { getDefaultShortcuts } from "./shortcuts";

export class CheatSheet {
  private overlay: HTMLElement;
  private isOpen = false;

  constructor() {
    this.overlay = document.createElement("div");
    this.overlay.className = "cheatsheet-overlay";
    this.overlay.style.display = "none";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "cheatsheet-dialog";

    const title = document.createElement("h2");
    title.className = "cheatsheet-title";
    title.textContent = "Keyboard Shortcuts";
    dialog.appendChild(title);

    const shortcuts = getDefaultShortcuts();
    const groups = new Map<string, Shortcut[]>();

    // Group by category (infer from shortcut properties)
    for (const s of shortcuts) {
      const category = this.categorize(s);
      const group = groups.get(category) || [];
      group.push(s);
      groups.set(category, group);
    }

    for (const [category, items] of groups) {
      const section = document.createElement("div");
      section.className = "cheatsheet-section";

      const heading = document.createElement("h3");
      heading.textContent = category;
      section.appendChild(heading);

      for (const shortcut of items) {
        const row = document.createElement("div");
        row.className = "cheatsheet-row";

        const label = document.createElement("span");
        label.className = "cheatsheet-label";
        label.textContent = shortcut.action.replace(/_/g, " ");
        row.appendChild(label);

        const keys = document.createElement("kbd");
        keys.className = "cheatsheet-keys";
        keys.textContent = this.formatKeys(shortcut);
        row.appendChild(keys);

        section.appendChild(row);
      }
      dialog.appendChild(section);
    }

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

    // Listen for ? key
    document.addEventListener("keydown", (e) => {
      if (e.key === "?" && !e.ctrlKey && !e.metaKey) {
        const target = e.target as HTMLElement;
        if (target.tagName !== "INPUT" && target.tagName !== "TEXTAREA" && !target.closest(".ProseMirror")) {
          this.toggle();
        }
      }
      if (e.key === "Escape" && this.isOpen) this.close();
    });
  }

  open(): void { this.isOpen = true; this.overlay.style.display = "flex"; }
  close(): void { this.isOpen = false; this.overlay.style.display = "none"; }
  toggle(): void { if (this.isOpen) this.close(); else this.open(); }

  private formatKeys(s: Shortcut): string {
    const parts: string[] = [];
    if (s.ctrl) parts.push("Ctrl");
    if (s.shift) parts.push("Shift");
    if (s.alt) parts.push("Alt");
    parts.push(s.key.length === 1 ? s.key.toUpperCase() : s.key);
    return parts.join(" + ");
  }

  private categorize(s: Shortcut): string {
    if (s.action.includes("zoom") || s.action.includes("fit") || s.action.includes("reset")) return "Navigation";
    if (s.action.includes("block") || s.action.includes("delete") || s.action.includes("duplicate")) return "Blocks";
    if (s.action.includes("compile") || s.action.includes("execute") || s.action.includes("run")) return "Execution";
    if (s.action.includes("toggle") || s.action.includes("library") || s.action.includes("palette")) return "Panels";
    if (s.action.includes("select") || s.action.includes("deselect")) return "Selection";
    if (s.action.includes("save") || s.action.includes("open")) return "File";
    if (s.action.includes("align")) return "Alignment";
    return "Other";
  }
}
