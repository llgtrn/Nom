export interface PaletteCommand {
  id: string;
  label: string;
  category: string;
  shortcut?: string;
  action: () => void;
}

export class CommandPalette {
  private overlay: HTMLElement;
  private input: HTMLInputElement;
  private resultsList: HTMLElement;
  private commands: PaletteCommand[] = [];
  private filtered: PaletteCommand[] = [];
  private selectedIndex = 0;
  private isOpen = false;

  constructor() {
    // Create overlay
    this.overlay = document.createElement("div");
    this.overlay.className = "command-palette-overlay";
    this.overlay.style.display = "none";

    const dialog = document.createElement("div");
    dialog.className = "command-palette";

    this.input = document.createElement("input");
    this.input.className = "palette-input";
    this.input.placeholder = "Type a command...";
    this.input.type = "text";
    dialog.appendChild(this.input);

    this.resultsList = document.createElement("div");
    this.resultsList.className = "palette-results";
    dialog.appendChild(this.resultsList);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

    // Events
    this.input.addEventListener("input", () => this.filter());
    this.input.addEventListener("keydown", (e) => this.handleKey(e));
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });
  }

  registerCommand(cmd: PaletteCommand): void {
    this.commands.push(cmd);
  }

  registerCommands(cmds: PaletteCommand[]): void {
    this.commands.push(...cmds);
  }

  open(): void {
    this.isOpen = true;
    this.overlay.style.display = "flex";
    this.input.value = "";
    this.selectedIndex = 0;
    this.filter();
    this.input.focus();
  }

  close(): void {
    this.isOpen = false;
    this.overlay.style.display = "none";
  }

  toggle(): void {
    if (this.isOpen) this.close(); else this.open();
  }

  private filter(): void {
    const query = this.input.value.toLowerCase();
    this.filtered = query
      ? this.commands.filter(c =>
          c.label.toLowerCase().includes(query) ||
          c.category.toLowerCase().includes(query) ||
          c.id.toLowerCase().includes(query)
        )
      : this.commands;
    this.selectedIndex = 0;
    this.render();
  }

  private render(): void {
    this.resultsList.replaceChildren();
    const max = Math.min(this.filtered.length, 12);
    for (let i = 0; i < max; i++) {
      const cmd = this.filtered[i];
      const item = document.createElement("div");
      item.className = `palette-item${i === this.selectedIndex ? " selected" : ""}`;

      const label = document.createElement("span");
      label.className = "palette-label";
      label.textContent = cmd.label;

      const category = document.createElement("span");
      category.className = "palette-category";
      category.textContent = cmd.category;

      item.appendChild(label);
      item.appendChild(category);

      if (cmd.shortcut) {
        const shortcut = document.createElement("span");
        shortcut.className = "palette-shortcut";
        shortcut.textContent = cmd.shortcut;
        item.appendChild(shortcut);
      }

      item.addEventListener("click", () => { cmd.action(); this.close(); });
      this.resultsList.appendChild(item);
    }
  }

  private handleKey(e: KeyboardEvent): void {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        this.selectedIndex = Math.min(this.selectedIndex + 1, this.filtered.length - 1);
        this.render();
        break;
      case "ArrowUp":
        e.preventDefault();
        this.selectedIndex = Math.max(this.selectedIndex - 1, 0);
        this.render();
        break;
      case "Enter":
        e.preventDefault();
        if (this.filtered[this.selectedIndex]) {
          this.filtered[this.selectedIndex].action();
          this.close();
        }
        break;
      case "Escape":
        this.close();
        break;
    }
  }
}
