import { invoke } from "@tauri-apps/api/core";
import { CompileResult } from "./types";

export type PreviewMode = "json" | "entities" | "plan" | "quality" | "security" | "dream";

export interface PreviewState {
  mode: PreviewMode;
  content: string;
  timestamp: number;
  blockId: string | null;
}

export class PreviewPanel {
  private panel: HTMLElement;
  private modeSelector: HTMLElement;
  private contentArea: HTMLElement;
  private currentMode: PreviewMode = "json";
  private history: PreviewState[] = [];

  constructor(containerId: string) {
    this.panel = document.getElementById(containerId) || document.createElement("aside");
    if (!this.panel.id) {
      this.panel.id = containerId;
      document.getElementById("app")?.appendChild(this.panel);
    }
    this.panel.className = "preview-panel";

    // Mode tabs
    this.modeSelector = document.createElement("div");
    this.modeSelector.className = "preview-modes";
    const modes: [PreviewMode, string][] = [
      ["json", "JSON"],
      ["entities", "Entities"],
      ["plan", "Plan"],
      ["quality", "Quality"],
      ["security", "Security"],
      ["dream", "Dream"],
    ];
    for (const [mode, label] of modes) {
      const tab = document.createElement("button");
      tab.className = `preview-tab${mode === this.currentMode ? " active" : ""}`;
      tab.textContent = label;
      tab.dataset.mode = mode;
      tab.addEventListener("click", () => this.switchMode(mode));
      this.modeSelector.appendChild(tab);
    }
    this.panel.appendChild(this.modeSelector);

    // Content area
    this.contentArea = document.createElement("div");
    this.contentArea.className = "preview-content";
    this.contentArea.textContent = "// compile a block to see preview";
    this.panel.appendChild(this.contentArea);
  }

  /** Switch preview mode */
  switchMode(mode: PreviewMode): void {
    this.currentMode = mode;
    // Update tab active state
    this.modeSelector.querySelectorAll(".preview-tab").forEach(tab => {
      tab.classList.toggle("active", (tab as HTMLElement).dataset.mode === mode);
    });
    // Re-render last result in new mode
    const last = this.history[this.history.length - 1];
    if (last) this.renderContent(last);
  }

  /** Show compile result */
  showCompileResult(result: CompileResult, blockId: string): void {
    const state: PreviewState = {
      mode: this.currentMode,
      content: JSON.stringify(result, null, 2),
      timestamp: Date.now(),
      blockId,
    };
    this.history.push(state);
    if (this.history.length > 50) this.history.shift();
    this.renderContent(state);
  }

  /** Show quality scores */
  async showQuality(source: string, blockId: string): Promise<void> {
    try {
      const scores = await invoke<Record<string, number>>("score_block", { source });
      const state: PreviewState = {
        mode: "quality",
        content: JSON.stringify(scores, null, 2),
        timestamp: Date.now(),
        blockId,
      };
      this.history.push(state);
      this.renderContent(state);
    } catch (e) {
      this.contentArea.textContent = `// quality error: ${e}`;
    }
  }

  /** Show security scan */
  async showSecurity(source: string, blockId: string): Promise<void> {
    try {
      const scan = await invoke<{ findings: string[]; risk_level: string }>("security_scan", { source });
      const state: PreviewState = {
        mode: "security",
        content: JSON.stringify(scan, null, 2),
        timestamp: Date.now(),
        blockId,
      };
      this.history.push(state);
      this.renderContent(state);
    } catch (e) {
      this.contentArea.textContent = `// security error: ${e}`;
    }
  }

  /** Show plan flow */
  async showPlan(source: string, blockId: string): Promise<void> {
    try {
      const plan = await invoke<{ nodes: number; edges: number; fusion_passes: string[] }>("plan_flow", { source });
      const state: PreviewState = {
        mode: "plan",
        content: JSON.stringify(plan, null, 2),
        timestamp: Date.now(),
        blockId,
      };
      this.history.push(state);
      this.renderContent(state);
    } catch (e) {
      this.contentArea.textContent = `// plan error: ${e}`;
    }
  }

  /** Show dream report */
  async showDream(manifest: string, blockId: string): Promise<void> {
    try {
      const report = await invoke<{ score: number; proposals: string[]; dict_hints: string[] }>("dream_report", { manifest });
      const state: PreviewState = {
        mode: "dream",
        content: JSON.stringify(report, null, 2),
        timestamp: Date.now(),
        blockId,
      };
      this.history.push(state);
      this.renderContent(state);
    } catch (e) {
      this.contentArea.textContent = `// dream error: ${e}`;
    }
  }

  /** Clear preview */
  clear(): void {
    this.contentArea.textContent = "// no preview";
  }

  /** Get current mode */
  getMode(): PreviewMode { return this.currentMode; }

  private renderContent(state: PreviewState): void {
    switch (this.currentMode) {
      case "json":
        this.contentArea.textContent = state.content;
        break;
      case "entities": {
        try {
          const parsed = JSON.parse(state.content);
          const entities = parsed.entities || [];
          this.contentArea.textContent = entities.length > 0
            ? entities.map((e: string, i: number) => `${i + 1}. ${e}`).join("\n")
            : "// no entities";
        } catch {
          this.contentArea.textContent = state.content;
        }
        break;
      }
      case "quality":
      case "security":
      case "plan":
      case "dream":
        // Pretty-print JSON for these modes
        try {
          const obj = JSON.parse(state.content);
          this.contentArea.textContent = formatObject(obj);
        } catch {
          this.contentArea.textContent = state.content;
        }
        break;
    }
  }
}

function formatObject(obj: Record<string, unknown>, indent = 0): string {
  const pad = "  ".repeat(indent);
  const lines: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    if (Array.isArray(value)) {
      lines.push(`${pad}${key}:`);
      for (const item of value) {
        lines.push(`${pad}  - ${item}`);
      }
    } else if (typeof value === "object" && value !== null) {
      lines.push(`${pad}${key}:`);
      lines.push(formatObject(value as Record<string, unknown>, indent + 1));
    } else if (typeof value === "number") {
      lines.push(`${pad}${key}: ${(value as number).toFixed?.(2) ?? value}`);
    } else {
      lines.push(`${pad}${key}: ${value}`);
    }
  }
  return lines.join("\n");
}
