export interface SettingsData {
  theme: string;
  gridColumns: number;
  gridGap: number;
  autoCompile: boolean;
  compileDelay: number;
  showMinimap: boolean;
  showStatusBar: boolean;
  showLineNumbers: boolean;
  fontSize: number;
  dictPath: string;
}

const DEFAULT_SETTINGS: SettingsData = {
  theme: "dark",
  gridColumns: 3,
  gridGap: 24,
  autoCompile: true,
  compileDelay: 500,
  showMinimap: true,
  showStatusBar: true,
  showLineNumbers: true,
  fontSize: 14,
  dictPath: "",
};

export class SettingsPanel {
  private panel: HTMLElement;
  private settings: SettingsData;
  private isOpen = false;
  private listeners: ((settings: SettingsData) => void)[] = [];

  constructor() {
    this.settings = this.load();
    this.panel = document.createElement("div");
    this.panel.className = "settings-panel";
    this.panel.style.display = "none";
    this.buildUI();
    document.body.appendChild(this.panel);
  }

  private buildUI(): void {
    const header = document.createElement("div");
    header.className = "settings-header";
    const title = document.createElement("h3");
    title.textContent = "Settings";
    header.appendChild(title);
    const closeBtn = document.createElement("button");
    closeBtn.textContent = "X";
    closeBtn.className = "settings-close";
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    this.panel.appendChild(header);

    const form = document.createElement("div");
    form.className = "settings-form";

    this.addSelect(form, "Theme", "theme", ["dark", "light", "high-contrast", "solarized"]);
    this.addNumber(form, "Font Size", "fontSize", 8, 32);
    this.addNumber(form, "Grid Columns", "gridColumns", 1, 6);
    this.addNumber(form, "Grid Gap (px)", "gridGap", 0, 100);
    this.addNumber(form, "Compile Delay (ms)", "compileDelay", 100, 5000);
    this.addToggle(form, "Auto Compile", "autoCompile");
    this.addToggle(form, "Show Minimap", "showMinimap");
    this.addToggle(form, "Show Status Bar", "showStatusBar");
    this.addToggle(form, "Show Line Numbers", "showLineNumbers");
    this.addText(form, "Dict Path", "dictPath", "~/.nom");

    this.panel.appendChild(form);
  }

  private addSelect(form: HTMLElement, label: string, key: keyof SettingsData, options: string[]): void {
    const row = this.createRow(label);
    const select = document.createElement("select");
    select.className = "settings-input";
    for (const opt of options) {
      const option = document.createElement("option");
      option.value = opt;
      option.textContent = opt;
      if (this.settings[key] === opt) option.selected = true;
      select.appendChild(option);
    }
    select.addEventListener("change", () => { (this.settings as any)[key] = select.value; this.save(); this.notify(); });
    row.appendChild(select);
    form.appendChild(row);
  }

  private addNumber(form: HTMLElement, label: string, key: keyof SettingsData, min: number, max: number): void {
    const row = this.createRow(label);
    const input = document.createElement("input");
    input.type = "number";
    input.className = "settings-input";
    input.min = String(min);
    input.max = String(max);
    input.value = String(this.settings[key]);
    input.addEventListener("change", () => { (this.settings as any)[key] = Number(input.value); this.save(); this.notify(); });
    row.appendChild(input);
    form.appendChild(row);
  }

  private addToggle(form: HTMLElement, label: string, key: keyof SettingsData): void {
    const row = this.createRow(label);
    const input = document.createElement("input");
    input.type = "checkbox";
    input.className = "settings-checkbox";
    input.checked = !!this.settings[key];
    input.addEventListener("change", () => { (this.settings as any)[key] = input.checked; this.save(); this.notify(); });
    row.appendChild(input);
    form.appendChild(row);
  }

  private addText(form: HTMLElement, label: string, key: keyof SettingsData, placeholder: string): void {
    const row = this.createRow(label);
    const input = document.createElement("input");
    input.type = "text";
    input.className = "settings-input";
    input.placeholder = placeholder;
    input.value = String(this.settings[key] || "");
    input.addEventListener("change", () => { (this.settings as any)[key] = input.value; this.save(); this.notify(); });
    row.appendChild(input);
    form.appendChild(row);
  }

  private createRow(label: string): HTMLElement {
    const row = document.createElement("div");
    row.className = "settings-row";
    const lbl = document.createElement("label");
    lbl.className = "settings-label";
    lbl.textContent = label;
    row.appendChild(lbl);
    return row;
  }

  open(): void { this.isOpen = true; this.panel.style.display = "block"; }
  close(): void { this.isOpen = false; this.panel.style.display = "none"; }
  toggle(): void { if (this.isOpen) this.close(); else this.open(); }

  getSettings(): SettingsData { return { ...this.settings }; }
  onChange(cb: (s: SettingsData) => void): void { this.listeners.push(cb); }

  private save(): void {
    try { localStorage.setItem("nomcanvas-settings", JSON.stringify(this.settings)); } catch {}
  }

  private load(): SettingsData {
    try {
      const stored = localStorage.getItem("nomcanvas-settings");
      if (stored) return { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
    } catch {}
    return { ...DEFAULT_SETTINGS };
  }

  private notify(): void { for (const cb of this.listeners) cb(this.getSettings()); }
}
