export interface HistoryAction {
  type: string;
  description: string;
  undo: () => void;
  redo: () => void;
  timestamp: number;
}

export class HistoryManager {
  private undoStack: HistoryAction[] = [];
  private redoStack: HistoryAction[] = [];
  private maxSize: number;
  private listeners: (() => void)[] = [];

  constructor(maxSize = 100) {
    this.maxSize = maxSize;
  }

  /** Record an action */
  push(action: Omit<HistoryAction, "timestamp">): void {
    this.undoStack.push({ ...action, timestamp: Date.now() });
    this.redoStack = []; // clear redo on new action
    if (this.undoStack.length > this.maxSize) {
      this.undoStack.shift();
    }
    this.notify();
  }

  /** Undo the last action */
  undo(): boolean {
    const action = this.undoStack.pop();
    if (!action) return false;
    action.undo();
    this.redoStack.push(action);
    this.notify();
    return true;
  }

  /** Redo the last undone action */
  redo(): boolean {
    const action = this.redoStack.pop();
    if (!action) return false;
    action.redo();
    this.undoStack.push(action);
    this.notify();
    return true;
  }

  /** Check if undo/redo is available */
  canUndo(): boolean { return this.undoStack.length > 0; }
  canRedo(): boolean { return this.redoStack.length > 0; }

  /** Get description of next undo/redo */
  nextUndoDescription(): string | null {
    return this.undoStack[this.undoStack.length - 1]?.description ?? null;
  }
  nextRedoDescription(): string | null {
    return this.redoStack[this.redoStack.length - 1]?.description ?? null;
  }

  /** Get full history */
  getHistory(): HistoryAction[] {
    return [...this.undoStack];
  }

  /** Clear all history */
  clear(): void {
    this.undoStack = [];
    this.redoStack = [];
    this.notify();
  }

  /** Listen for history changes */
  onChange(callback: () => void): void {
    this.listeners.push(callback);
  }

  private notify(): void {
    for (const cb of this.listeners) cb();
  }

  /** Helper: create a move-block action */
  static moveBlock(
    blockId: string,
    fromX: number, fromY: number,
    toX: number, toY: number,
    applyPosition: (id: string, x: number, y: number) => void
  ): Omit<HistoryAction, "timestamp"> {
    return {
      type: "move_block",
      description: `Move block ${blockId}`,
      undo: () => applyPosition(blockId, fromX, fromY),
      redo: () => applyPosition(blockId, toX, toY),
    };
  }

  /** Helper: create an add-block action */
  static addBlock(
    blockId: string,
    addFn: () => void,
    removeFn: () => void
  ): Omit<HistoryAction, "timestamp"> {
    return {
      type: "add_block",
      description: `Add block ${blockId}`,
      undo: removeFn,
      redo: addFn,
    };
  }

  /** Helper: create a wire action */
  static addWire(
    wireId: string,
    addFn: () => void,
    removeFn: () => void
  ): Omit<HistoryAction, "timestamp"> {
    return {
      type: "add_wire",
      description: `Wire ${wireId}`,
      undo: removeFn,
      redo: addFn,
    };
  }
}
