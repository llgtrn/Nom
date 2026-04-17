export interface WorkspaceData {
  version: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  blocks: WorkspaceBlock[];
  wires: WorkspaceWire[];
  graph: string | null; // serialized WorkflowGraph JSON
  settings: Record<string, unknown>;
}

export interface WorkspaceBlock {
  id: string;
  type: string;
  x: number;
  y: number;
  width: number;
  height: number;
  content: string;
}

export interface WorkspaceWire {
  id: string;
  fromBlockId: string;
  toBlockId: string;
  status: string;
}

const WORKSPACE_VERSION = "1.0.0";

export class WorkspaceManager {
  /** Export workspace to JSON string */
  static exportWorkspace(
    blocks: WorkspaceBlock[],
    wires: WorkspaceWire[],
    graphJson: string | null,
    settings: Record<string, unknown>,
    name: string = "Untitled"
  ): string {
    const data: WorkspaceData = {
      version: WORKSPACE_VERSION,
      name,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      blocks,
      wires,
      graph: graphJson,
      settings,
    };
    return JSON.stringify(data, null, 2);
  }

  /** Import workspace from JSON string */
  static importWorkspace(json: string): WorkspaceData | null {
    try {
      const data = JSON.parse(json);
      // Schema validation
      if (typeof data !== "object" || data === null) return null;
      if (typeof data.version !== "string") return null;
      if (!Array.isArray(data.blocks)) return null;
      if (!Array.isArray(data.wires)) return null;

      // Validate blocks have required fields
      for (const block of data.blocks) {
        if (typeof block.id !== "string") return null;
        if (typeof block.type !== "string") return null;
        if (typeof block.x !== "number") return null;
        if (typeof block.y !== "number") return null;
      }

      // Validate wires have required fields
      for (const wire of data.wires) {
        if (typeof wire.id !== "string") return null;
        if (typeof wire.fromBlockId !== "string") return null;
        if (typeof wire.toBlockId !== "string") return null;
      }

      return data as WorkspaceData;
    } catch {
      return null;
    }
  }

  /** Trigger download of workspace file */
  static downloadWorkspace(data: string, filename: string = "workspace.nomcanvas"): void {
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  /** Open file picker and load workspace */
  static async uploadWorkspace(): Promise<WorkspaceData | null> {
    return new Promise((resolve) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".nomcanvas,.json";
      input.addEventListener("change", () => {
        const file = input.files?.[0];
        if (!file) { resolve(null); return; }
        const reader = new FileReader();
        reader.onload = () => {
          const json = reader.result as string;
          resolve(WorkspaceManager.importWorkspace(json));
        };
        reader.onerror = () => resolve(null);
        reader.readAsText(file);
      });
      input.click();
    });
  }

  /** Auto-save to localStorage */
  static autoSave(data: string): void {
    try { localStorage.setItem("nomcanvas-workspace", data); } catch {}
  }

  /** Load auto-saved workspace */
  static autoLoad(): WorkspaceData | null {
    try {
      const stored = localStorage.getItem("nomcanvas-workspace");
      if (stored) return WorkspaceManager.importWorkspace(stored);
    } catch {}
    return null;
  }
}
