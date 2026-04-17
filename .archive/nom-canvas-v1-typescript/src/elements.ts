/** Base properties shared by all canvas elements (Excalidraw pattern) */
export interface ElementBase {
  id: string;
  type: ElementType;
  x: number;
  y: number;
  width: number;
  height: number;
  angle: number; // radians
  strokeColor: string;
  fillColor: string;
  strokeWidth: number;
  opacity: number;
  isDeleted: boolean;
  groupId: string | null;
  locked: boolean;
  version: number;
}

export type ElementType = "rectangle" | "ellipse" | "diamond" | "text" | "arrow" | "line" | "connector" | "image";

// Shape elements
export interface RectangleElement extends ElementBase {
  type: "rectangle";
  borderRadius: number;
}

export interface EllipseElement extends ElementBase {
  type: "ellipse";
}

export interface DiamondElement extends ElementBase {
  type: "diamond";
}

export interface TextElement extends ElementBase {
  type: "text";
  text: string;
  fontSize: number;
  fontFamily: string;
  textAlign: "left" | "center" | "right";
}

export interface ArrowElement extends ElementBase {
  type: "arrow";
  points: [number, number][]; // relative to x,y
  startBinding: string | null; // element id
  endBinding: string | null;
}

export interface LineElement extends ElementBase {
  type: "line";
  points: [number, number][];
}

export interface ConnectorElement extends ElementBase {
  type: "connector";
  sourceId: string;
  targetId: string;
  sourcePort: string;
  targetPort: string;
}

export interface ImageElement extends ElementBase {
  type: "image";
  src: string;
  mime: string;
}

export type CanvasElement =
  | RectangleElement
  | EllipseElement
  | DiamondElement
  | TextElement
  | ArrowElement
  | LineElement
  | ConnectorElement
  | ImageElement;

/** Create a new element with defaults */
export function createElement(type: ElementType, x: number, y: number, overrides?: Partial<ElementBase>): CanvasElement {
  const base: ElementBase = {
    id: `el-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    type,
    x, y,
    width: 100,
    height: 60,
    angle: 0,
    strokeColor: "#94A3B8",
    fillColor: "transparent",
    strokeWidth: 2,
    opacity: 1,
    isDeleted: false,
    groupId: null,
    locked: false,
    version: 1,
    ...overrides,
  };

  switch (type) {
    case "rectangle": return { ...base, type: "rectangle", borderRadius: 4 } as RectangleElement;
    case "ellipse": return { ...base, type: "ellipse" } as EllipseElement;
    case "diamond": return { ...base, type: "diamond" } as DiamondElement;
    case "text": return { ...base, type: "text", text: "", fontSize: 14, fontFamily: "IBM Plex Sans", textAlign: "left" } as TextElement;
    case "arrow": return { ...base, type: "arrow", points: [[0, 0], [100, 0]], startBinding: null, endBinding: null } as ArrowElement;
    case "line": return { ...base, type: "line", points: [[0, 0], [100, 0]] } as LineElement;
    case "connector": return { ...base, type: "connector", sourceId: "", targetId: "", sourcePort: "output", targetPort: "input" } as ConnectorElement;
    case "image": return { ...base, type: "image", src: "", mime: "" } as ImageElement;
  }
}

/** Two-phase hit testing (Excalidraw pattern) */

/** Phase 1: AABB pre-filter (accounts for rotation) */
function isPointInRotatedBounds(px: number, py: number, el: ElementBase, threshold: number): boolean {
  // Inverse-rotate point around element center
  const cx = el.x + el.width / 2;
  const cy = el.y + el.height / 2;
  const cos = Math.cos(-el.angle);
  const sin = Math.sin(-el.angle);
  const dx = px - cx;
  const dy = py - cy;
  const rx = dx * cos - dy * sin + cx;
  const ry = dx * sin + dy * cos + cy;
  // AABB test with threshold
  return (
    rx >= el.x - threshold &&
    rx <= el.x + el.width + threshold &&
    ry >= el.y - threshold &&
    ry <= el.y + el.height + threshold
  );
}

/** Phase 2: Precise shape-specific test */
function isPointOnShape(px: number, py: number, el: CanvasElement, threshold: number): boolean {
  switch (el.type) {
    case "rectangle":
    case "image":
    case "text":
      return true; // AABB already passed
    case "ellipse": {
      const cx = el.x + el.width / 2;
      const cy = el.y + el.height / 2;
      const rx = el.width / 2 + threshold;
      const ry = el.height / 2 + threshold;
      const dx = (px - cx) / rx;
      const dy = (py - cy) / ry;
      return dx * dx + dy * dy <= 1;
    }
    case "diamond": {
      const cx = el.x + el.width / 2;
      const cy = el.y + el.height / 2;
      const dx = Math.abs(px - cx) / (el.width / 2 + threshold);
      const dy = Math.abs(py - cy) / (el.height / 2 + threshold);
      return dx + dy <= 1;
    }
    default:
      return true;
  }
}

/** Full hit test: returns element at point or null */
export function hitTest(elements: CanvasElement[], px: number, py: number, threshold = 5): CanvasElement | null {
  // Reverse order: top elements first
  for (let i = elements.length - 1; i >= 0; i--) {
    const el = elements[i];
    if (el.isDeleted) continue;
    if (!isPointInRotatedBounds(px, py, el, threshold)) continue;
    if (isPointOnShape(px, py, el, threshold)) return el;
  }
  return null;
}

/** Transform handle positions */
export type HandleDirection = "n" | "s" | "e" | "w" | "nw" | "ne" | "sw" | "se" | "rotation";

export interface TransformHandle {
  direction: HandleDirection;
  x: number;
  y: number;
  cursor: string;
}

export function getTransformHandles(el: ElementBase, handleSize = 8): TransformHandle[] {
  const hs = handleSize / 2;
  const { x, y, width: w, height: h } = el;
  return [
    { direction: "nw", x: x - hs, y: y - hs, cursor: "nwse-resize" },
    { direction: "n", x: x + w / 2 - hs, y: y - hs, cursor: "ns-resize" },
    { direction: "ne", x: x + w - hs, y: y - hs, cursor: "nesw-resize" },
    { direction: "e", x: x + w - hs, y: y + h / 2 - hs, cursor: "ew-resize" },
    { direction: "se", x: x + w - hs, y: y + h - hs, cursor: "nwse-resize" },
    { direction: "s", x: x + w / 2 - hs, y: y + h - hs, cursor: "ns-resize" },
    { direction: "sw", x: x - hs, y: y + h - hs, cursor: "nesw-resize" },
    { direction: "w", x: x - hs, y: y + h / 2 - hs, cursor: "ew-resize" },
    { direction: "rotation", x: x + w / 2 - hs, y: y - 30, cursor: "grab" },
  ];
}

/** Element store */
export class ElementStore {
  private elements: CanvasElement[] = [];
  private selectedIds: Set<string> = new Set();

  add(element: CanvasElement): void {
    this.elements.push(element);
  }

  remove(id: string): void {
    const el = this.elements.find(e => e.id === id);
    if (el) el.isDeleted = true;
  }

  getAll(): CanvasElement[] {
    return this.elements.filter(e => !e.isDeleted);
  }

  getById(id: string): CanvasElement | undefined {
    return this.elements.find(e => e.id === id && !e.isDeleted);
  }

  select(id: string): void { this.selectedIds.add(id); }
  deselect(id: string): void { this.selectedIds.delete(id); }
  clearSelection(): void { this.selectedIds.clear(); }
  getSelected(): CanvasElement[] {
    return this.getAll().filter(e => this.selectedIds.has(e.id));
  }
  isSelected(id: string): boolean { return this.selectedIds.has(id); }

  hitTest(px: number, py: number, threshold?: number): CanvasElement | null {
    return hitTest(this.getAll(), px, py, threshold);
  }
}
