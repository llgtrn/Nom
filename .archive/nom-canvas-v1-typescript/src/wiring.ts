import { invoke } from "@tauri-apps/api/core";

export interface WireCheck {
  status: string;  // "compatible" | "needs_adapter" | "incompatible"
  reason: string | null;
}

export interface Wire {
  id: string;
  fromBlockId: string;
  toBlockId: string;
  fromPort: "output";
  toPort: "input";
  status: "pending" | "compatible" | "needs_adapter" | "incompatible";
}

export class WiringEngine {
  private wires: Map<string, Wire> = new Map();
  private svgLayer: SVGSVGElement;
  private dragState: {
    fromBlockId: string;
    startX: number;
    startY: number;
  } | null = null;

  private nextId = 0;

  constructor(containerId: string) {
    // Create SVG overlay for wire rendering
    const container = document.getElementById(containerId)!;
    this.svgLayer = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    this.svgLayer.classList.add("wiring-layer");
    this.svgLayer.setAttribute("width", "100%");
    this.svgLayer.setAttribute("height", "100%");
    container.appendChild(this.svgLayer);
  }

  /** Start dragging a wire from a block's output port */
  startWire(fromBlockId: string, startX: number, startY: number): void {
    this.dragState = { fromBlockId, startX, startY };
  }

  /** Update the in-progress wire endpoint during drag */
  updateDrag(currentX: number, currentY: number): void {
    if (!this.dragState) return;
    // Remove any existing drag preview
    const preview = this.svgLayer.querySelector(".wire-preview");
    if (preview) preview.remove();

    // Draw preview wire
    const path = this.createWirePath(
      this.dragState.startX, this.dragState.startY,
      currentX, currentY
    );
    path.classList.add("wire-preview");
    this.svgLayer.appendChild(path);
  }

  /** Complete the wire connection to a target block */
  async endWire(toBlockId: string, endX: number, endY: number): Promise<Wire | null> {
    if (!this.dragState) return null;
    if (this.dragState.fromBlockId === toBlockId) {
      this.cancelDrag();
      return null; // Can't wire to self
    }

    const wireId = `wire-${this.nextId++}`;
    const wire: Wire = {
      id: wireId,
      fromBlockId: this.dragState.fromBlockId,
      toBlockId,
      fromPort: "output",
      toPort: "input",
      status: "pending",
    };

    // Check compatibility via Tauri command
    try {
      const check = await invoke<WireCheck>("wire_check", {
        fromHash: this.dragState.fromBlockId,
        toHash: toBlockId,
      });
      wire.status = check.status as Wire["status"];
    } catch {
      wire.status = "incompatible";
    }

    this.wires.set(wireId, wire);

    // Remove preview, draw permanent wire
    const savedDragState = this.dragState;
    this.cancelDrag();
    this.renderWire(wire, savedDragState.startX, savedDragState.startY, endX, endY);

    return wire;
  }

  /** Cancel an in-progress wire drag */
  cancelDrag(): void {
    this.dragState = null;
    const preview = this.svgLayer.querySelector(".wire-preview");
    if (preview) preview.remove();
  }

  /** Remove a wire */
  removeWire(wireId: string): void {
    this.wires.delete(wireId);
    const el = this.svgLayer.querySelector(`[data-wire-id="${wireId}"]`);
    if (el) el.remove();
  }

  /** Get all wires */
  getWires(): Wire[] {
    return Array.from(this.wires.values());
  }

  /** Get wires connected to a specific block */
  getBlockWires(blockId: string): Wire[] {
    return this.getWires().filter(
      w => w.fromBlockId === blockId || w.toBlockId === blockId
    );
  }

  private createWirePath(x1: number, y1: number, x2: number, y2: number): SVGPathElement {
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    // Bezier curve for smooth wire
    const dx = Math.abs(x2 - x1) * 0.5;
    const d = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
    path.setAttribute("d", d);
    path.setAttribute("fill", "none");
    return path;
  }

  private renderWire(wire: Wire, x1: number, y1: number, x2: number, y2: number): void {
    const path = this.createWirePath(x1, y1, x2, y2);
    path.setAttribute("data-wire-id", wire.id);
    path.classList.add("wire", `wire-${wire.status}`);
    this.svgLayer.appendChild(path);
  }
}
