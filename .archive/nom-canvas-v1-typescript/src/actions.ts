import { invoke } from "@tauri-apps/api/core";
import { CompileResult } from "./types";

export type ActionType = "compile" | "build" | "test" | "score" | "extract" | "scan" | "custom";

export interface Action {
  id: string;
  type: ActionType;
  blockId: string;
  params: Record<string, string>;
  status: "pending" | "running" | "success" | "error";
  output: string;
  startedAt: number | null;
  completedAt: number | null;
}

export type ActionCallback = (action: Action) => void;

export class ActionRunner {
  private queue: Action[] = [];
  private running: Action | null = null;
  private listeners: ActionCallback[] = [];

  onAction(callback: ActionCallback): void {
    this.listeners.push(callback);
  }

  private emit(action: Action): void {
    for (const cb of this.listeners) cb(action);
  }

  /** Queue an action for execution */
  enqueue(type: ActionType, blockId: string, params: Record<string, string> = {}): Action {
    const action: Action = {
      id: `action-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      type,
      blockId,
      params,
      status: "pending",
      output: "",
      startedAt: null,
      completedAt: null,
    };
    this.queue.push(action);
    this.emit(action);
    this.processNext();
    return action;
  }

  /** Cancel all pending actions */
  cancelAll(): void {
    this.queue = this.queue.filter(a => {
      if (a.status === "pending") {
        a.status = "error";
        a.output = "cancelled";
        this.emit(a);
        return false;
      }
      return true;
    });
  }

  /** Get action by ID */
  getAction(id: string): Action | undefined {
    if (this.running?.id === id) return this.running;
    return this.queue.find(a => a.id === id);
  }

  /** Get all actions for a block */
  getBlockActions(blockId: string): Action[] {
    const all = this.running ? [this.running, ...this.queue] : [...this.queue];
    return all.filter(a => a.blockId === blockId);
  }

  private async processNext(): Promise<void> {
    if (this.running) return;
    const next = this.queue.find(a => a.status === "pending");
    if (!next) return;

    this.running = next;
    next.status = "running";
    next.startedAt = Date.now();
    this.emit(next);

    try {
      next.output = await this.executeAction(next);
      next.status = "success";
    } catch (e) {
      next.output = String(e);
      next.status = "error";
    }

    next.completedAt = Date.now();
    this.running = null;
    this.emit(next);

    // Process next in queue
    this.processNext();
  }

  private async executeAction(action: Action): Promise<string> {
    switch (action.type) {
      case "compile": {
        const result = await invoke<CompileResult>("compile_block", { source: action.params.source || "" });
        return result.success
          ? `Compiled: ${result.entities.join(", ")}`
          : `Error: ${result.diagnostics.join("; ")}`;
      }
      case "score": {
        const scores = await invoke<Record<string, number>>("score_block", { source: action.params.source || "" });
        return `Quality: ${Object.entries(scores).map(([k, v]) => `${k}=${(v * 100).toFixed(0)}%`).join(", ")}`;
      }
      case "scan": {
        const scan = await invoke<{ findings: string[]; risk_level: string }>(
          "security_scan", { source: action.params.source || "" }
        );
        return `Risk: ${scan.risk_level}, Findings: ${scan.findings.length}`;
      }
      case "extract": {
        const result = await invoke<{ entities_found: number; languages: string[] }>(
          "extract_atoms", { path: action.params.path || "" }
        );
        return `Found ${result.entities_found} entities in ${result.languages.join(", ")}`;
      }
      case "build": {
        const result = await invoke<{ success: boolean; artifact_path: string | null; error: string | null }>(
          "build_artifact", { manifestHash: action.params.manifestHash || "" }
        );
        return result.success ? `Built: ${result.artifact_path}` : `Error: ${result.error}`;
      }
      case "test": {
        return "Test action not yet implemented";
      }
      case "custom": {
        return `Custom action: ${JSON.stringify(action.params)}`;
      }
    }
  }
}
