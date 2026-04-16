export interface BlockPosition {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
}

export interface CanvasState {
  blocks: Map<string, BlockPosition>;
  nextZ: number;
}

export interface Bound {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function boundsIntersect(a: Bound, b: Bound): boolean {
  return !(a.x + a.w < b.x || b.x + b.w < a.x || a.y + a.h < b.y || b.y + b.h < a.y);
}

export class Viewport {
  private _zoom = 1;
  private _panX = 0;
  private _panY = 0;
  private _width = 0;
  private _height = 0;

  get zoom() { return this._zoom; }
  get panX() { return this._panX; }
  get panY() { return this._panY; }

  /** Convert client (screen) coordinates to model (canvas) coordinates */
  clientToModel(clientX: number, clientY: number): [number, number] {
    return [
      (clientX - this._panX) / this._zoom,
      (clientY - this._panY) / this._zoom,
    ];
  }

  /** Convert model coordinates to client coordinates */
  modelToClient(modelX: number, modelY: number): [number, number] {
    return [
      modelX * this._zoom + this._panX,
      modelY * this._zoom + this._panY,
    ];
  }

  /** Get the visible bounds in model coordinates */
  get viewportBounds(): Bound {
    const [left, top] = this.clientToModel(0, 0);
    const [right, bottom] = this.clientToModel(this._width, this._height);
    return { x: left, y: top, w: right - left, h: bottom - top };
  }

  setZoom(zoom: number, centerX?: number, centerY?: number) {
    const newZoom = Math.max(0.1, Math.min(5.0, zoom));
    if (centerX !== undefined && centerY !== undefined) {
      // Zoom toward cursor position
      const [modelX, modelY] = this.clientToModel(centerX, centerY);
      this._zoom = newZoom;
      this._panX = centerX - modelX * newZoom;
      this._panY = centerY - modelY * newZoom;
    } else {
      this._zoom = newZoom;
    }
  }

  pan(dx: number, dy: number) {
    this._panX += dx;
    this._panY += dy;
  }

  setPan(x: number, y: number) {
    this._panX = x;
    this._panY = y;
  }

  setSize(width: number, height: number) {
    this._width = width;
    this._height = height;
  }

  resetView() {
    this._zoom = 1;
    this._panX = 0;
    this._panY = 0;
  }
}

export class SpatialCanvas {
  private vp: Viewport;
  private state: CanvasState;
  private container: HTMLElement;
  private viewportEl: HTMLElement;
  private isDragging = false;
  private isPanning = false;
  private dragTarget: string | null = null;
  private dragStartX = 0;
  private dragStartY = 0;
  private dragOffsetX = 0;
  private dragOffsetY = 0;

  constructor(containerId: string) {
    this.container = document.getElementById(containerId)!;

    // Create viewport layer
    this.viewportEl = document.createElement("div");
    this.viewportEl.className = "canvas-viewport";
    this.container.appendChild(this.viewportEl);

    this.vp = new Viewport();
    this.state = {
      blocks: new Map(),
      nextZ: 1,
    };

    // Sync viewport size so viewportBounds is accurate
    const rect = this.container.getBoundingClientRect();
    this.vp.setSize(rect.width || window.innerWidth, rect.height || window.innerHeight);

    this.setupEventListeners();
    this.updateTransform();
  }

  /** Add a block to the canvas at a given position */
  addBlock(id: string, x: number, y: number, width = 600, height = 200): HTMLElement {
    const pos: BlockPosition = { id, x, y, width, height, zIndex: this.state.nextZ++ };
    this.state.blocks.set(id, pos);

    const el = document.createElement("div");
    el.className = "canvas-block";
    el.dataset.blockId = id;
    el.style.cssText = `
      position: absolute;
      left: ${x}px;
      top: ${y}px;
      width: ${width}px;
      min-height: ${height}px;
      z-index: ${pos.zIndex};
    `;

    // Drag handle — constructed via DOM methods to avoid innerHTML
    const handle = document.createElement("div");
    handle.className = "block-handle";
    const label = document.createElement("span");
    label.className = "block-id";
    label.textContent = id;
    handle.appendChild(label);
    el.appendChild(handle);

    // Content area (where Prosemirror mounts)
    const content = document.createElement("div");
    content.className = "block-content";
    el.appendChild(content);

    this.viewportEl.appendChild(el);
    return content; // Return content area for editor mounting
  }

  /** Remove a block */
  removeBlock(id: string) {
    this.state.blocks.delete(id);
    const el = this.viewportEl.querySelector(`[data-block-id="${id}"]`);
    if (el) el.remove();
  }

  /** Get block position */
  getBlockPosition(id: string): BlockPosition | undefined {
    return this.state.blocks.get(id);
  }

  /** Bring block to front */
  bringToFront(id: string) {
    const pos = this.state.blocks.get(id);
    if (!pos) return;
    pos.zIndex = this.state.nextZ++;
    const el = this.viewportEl.querySelector(`[data-block-id="${id}"]`) as HTMLElement;
    if (el) el.style.zIndex = String(pos.zIndex);
  }

  /** Set zoom level (0.1 to 5.0) */
  setZoom(zoom: number) {
    this.vp.setZoom(zoom);
    this.updateTransform();
  }

  /** Pan to a position */
  panTo(x: number, y: number) {
    this.vp.setPan(x, y);
    this.updateTransform();
  }

  /** Reset view to origin */
  resetView() {
    this.vp.resetView();
    this.updateTransform();
  }

  /** Returns all blocks whose bounding rect intersects the current viewport */
  getVisibleBlocks(): BlockPosition[] {
    const vb = this.vp.viewportBounds;
    const result: BlockPosition[] = [];
    for (const pos of this.state.blocks.values()) {
      const blockBound: Bound = { x: pos.x, y: pos.y, w: pos.width, h: pos.height };
      if (boundsIntersect(vb, blockBound)) {
        result.push(pos);
      }
    }
    return result;
  }

  private updateTransform() {
    const { panX, panY, zoom } = this.vp;
    this.viewportEl.style.transform = `translate(${panX}px, ${panY}px) scale(${zoom})`;
    this.viewportEl.style.transformOrigin = "0 0";
  }

  private setupEventListeners() {
    // Zoom with Ctrl+scroll — zoom toward cursor position
    this.container.addEventListener("wheel", (e) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        this.vp.setZoom(this.vp.zoom * delta, e.clientX, e.clientY);
        this.updateTransform();
      }
    }, { passive: false });

    // Pan with middle mouse or Ctrl+drag on background
    this.container.addEventListener("mousedown", (e) => {
      // Check if clicking on a block handle for dragging
      const handle = (e.target as HTMLElement).closest(".block-handle");
      if (handle) {
        const block = handle.closest(".canvas-block") as HTMLElement;
        const blockId = block?.dataset.blockId;
        if (blockId) {
          this.isDragging = true;
          this.dragTarget = blockId;
          this.bringToFront(blockId);
          const pos = this.state.blocks.get(blockId)!;
          const [modelX, modelY] = this.vp.clientToModel(e.clientX, e.clientY);
          this.dragOffsetX = modelX - pos.x;
          this.dragOffsetY = modelY - pos.y;
          e.preventDefault();
          return;
        }
      }

      // Pan with middle button or Ctrl+left
      if (e.button === 1 || (e.button === 0 && (e.ctrlKey || e.metaKey))) {
        this.isPanning = true;
        this.dragStartX = e.clientX - this.vp.panX;
        this.dragStartY = e.clientY - this.vp.panY;
        e.preventDefault();
      }
    });

    window.addEventListener("mousemove", (e) => {
      if (this.isDragging && this.dragTarget) {
        const pos = this.state.blocks.get(this.dragTarget);
        if (!pos) return;
        const [modelX, modelY] = this.vp.clientToModel(e.clientX, e.clientY);
        pos.x = modelX - this.dragOffsetX;
        pos.y = modelY - this.dragOffsetY;
        const el = this.viewportEl.querySelector(`[data-block-id="${this.dragTarget}"]`) as HTMLElement;
        if (el) {
          el.style.left = `${pos.x}px`;
          el.style.top = `${pos.y}px`;
        }
      }
      if (this.isPanning) {
        this.vp.setPan(e.clientX - this.dragStartX, e.clientY - this.dragStartY);
        this.updateTransform();
      }
    });

    window.addEventListener("mouseup", () => {
      this.isDragging = false;
      this.isPanning = false;
      this.dragTarget = null;
    });
  }
}
