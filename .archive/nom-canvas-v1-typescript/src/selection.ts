import type { CanvasElement } from "./elements";
import type { Viewport } from "./canvas";

export interface SelectionBox {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

export class SelectionManager {
  private selectedIds: Set<string> = new Set();
  private selectionBox: SelectionBox | null = null;
  private isDragging = false;
  private listeners: ((ids: Set<string>) => void)[] = [];

  /** Start rubber-band selection */
  startSelection(x: number, y: number): void {
    this.selectionBox = { startX: x, startY: y, endX: x, endY: y };
    this.isDragging = true;
  }

  /** Update rubber-band during drag */
  updateSelection(x: number, y: number): void {
    if (this.selectionBox && this.isDragging) {
      this.selectionBox.endX = x;
      this.selectionBox.endY = y;
    }
  }

  /** End rubber-band and select elements within box */
  endSelection(elements: CanvasElement[]): void {
    if (this.selectionBox && this.isDragging) {
      const box = this.normalizeBox(this.selectionBox);
      for (const el of elements) {
        if (el.isDeleted) continue;
        if (this.elementIntersectsBox(el, box)) {
          this.selectedIds.add(el.id);
        }
      }
      this.notifyChange();
    }
    this.selectionBox = null;
    this.isDragging = false;
  }

  /** Toggle selection on click (with Shift for multi-select) */
  toggleSelect(id: string, addToSelection: boolean): void {
    if (addToSelection) {
      if (this.selectedIds.has(id)) {
        this.selectedIds.delete(id);
      } else {
        this.selectedIds.add(id);
      }
    } else {
      this.selectedIds.clear();
      this.selectedIds.add(id);
    }
    this.notifyChange();
  }

  /** Select all elements */
  selectAll(elements: CanvasElement[]): void {
    this.selectedIds.clear();
    for (const el of elements) {
      if (!el.isDeleted) this.selectedIds.add(el.id);
    }
    this.notifyChange();
  }

  /** Clear selection */
  clearSelection(): void {
    this.selectedIds.clear();
    this.notifyChange();
  }

  /** Get selected IDs */
  getSelectedIds(): Set<string> {
    return new Set(this.selectedIds);
  }

  /** Get count */
  getSelectedCount(): number {
    return this.selectedIds.size;
  }

  /** Check if element is selected */
  isSelected(id: string): boolean {
    return this.selectedIds.has(id);
  }

  /** Get current selection box (for rendering) */
  getSelectionBox(): SelectionBox | null {
    return this.selectionBox;
  }

  /** Listen for selection changes */
  onChange(callback: (ids: Set<string>) => void): void {
    this.listeners.push(callback);
  }

  /** Render selection box on canvas */
  renderSelectionBox(ctx: CanvasRenderingContext2D): void {
    if (!this.selectionBox || !this.isDragging) return;
    const box = this.normalizeBox(this.selectionBox);
    ctx.save();
    ctx.strokeStyle = "#22C55E";
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    ctx.fillStyle = "rgba(34, 197, 94, 0.08)";
    ctx.fillRect(box.startX, box.startY, box.endX - box.startX, box.endY - box.startY);
    ctx.strokeRect(box.startX, box.startY, box.endX - box.startX, box.endY - box.startY);
    ctx.restore();
  }

  private normalizeBox(box: SelectionBox): SelectionBox {
    return {
      startX: Math.min(box.startX, box.endX),
      startY: Math.min(box.startY, box.endY),
      endX: Math.max(box.startX, box.endX),
      endY: Math.max(box.startY, box.endY),
    };
  }

  private elementIntersectsBox(el: CanvasElement, box: SelectionBox): boolean {
    return !(
      el.x + el.width < box.startX ||
      el.x > box.endX ||
      el.y + el.height < box.startY ||
      el.y > box.endY
    );
  }

  private notifyChange(): void {
    for (const cb of this.listeners) cb(this.getSelectedIds());
  }
}
