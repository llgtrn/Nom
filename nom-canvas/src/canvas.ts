export interface BlockPosition {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
}

export interface CanvasState {
  panX: number;
  panY: number;
  zoom: number;
  blocks: Map<string, BlockPosition>;
  nextZ: number;
}

export class SpatialCanvas {
  private state: CanvasState;
  private container: HTMLElement;
  private viewport: HTMLElement;
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
    this.viewport = document.createElement("div");
    this.viewport.className = "canvas-viewport";
    this.container.appendChild(this.viewport);

    this.state = {
      panX: 0,
      panY: 0,
      zoom: 1,
      blocks: new Map(),
      nextZ: 1,
    };

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

    this.viewport.appendChild(el);
    return content; // Return content area for editor mounting
  }

  /** Remove a block */
  removeBlock(id: string) {
    this.state.blocks.delete(id);
    const el = this.viewport.querySelector(`[data-block-id="${id}"]`);
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
    const el = this.viewport.querySelector(`[data-block-id="${id}"]`) as HTMLElement;
    if (el) el.style.zIndex = String(pos.zIndex);
  }

  /** Set zoom level (0.25 to 4.0) */
  setZoom(zoom: number) {
    this.state.zoom = Math.max(0.25, Math.min(4.0, zoom));
    this.updateTransform();
  }

  /** Pan to a position */
  panTo(x: number, y: number) {
    this.state.panX = x;
    this.state.panY = y;
    this.updateTransform();
  }

  /** Reset view to origin */
  resetView() {
    this.state.panX = 0;
    this.state.panY = 0;
    this.state.zoom = 1;
    this.updateTransform();
  }

  private updateTransform() {
    const { panX, panY, zoom } = this.state;
    this.viewport.style.transform = `translate(${panX}px, ${panY}px) scale(${zoom})`;
    this.viewport.style.transformOrigin = "0 0";
  }

  private setupEventListeners() {
    // Zoom with Ctrl+scroll
    this.container.addEventListener("wheel", (e) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        this.setZoom(this.state.zoom * delta);
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
          this.dragOffsetX = e.clientX / this.state.zoom - pos.x;
          this.dragOffsetY = e.clientY / this.state.zoom - pos.y;
          e.preventDefault();
          return;
        }
      }

      // Pan with middle button or Ctrl+left
      if (e.button === 1 || (e.button === 0 && (e.ctrlKey || e.metaKey))) {
        this.isPanning = true;
        this.dragStartX = e.clientX - this.state.panX;
        this.dragStartY = e.clientY - this.state.panY;
        e.preventDefault();
      }
    });

    window.addEventListener("mousemove", (e) => {
      if (this.isDragging && this.dragTarget) {
        const pos = this.state.blocks.get(this.dragTarget);
        if (!pos) return;
        pos.x = e.clientX / this.state.zoom - this.dragOffsetX;
        pos.y = e.clientY / this.state.zoom - this.dragOffsetY;
        const el = this.viewport.querySelector(`[data-block-id="${this.dragTarget}"]`) as HTMLElement;
        if (el) {
          el.style.left = `${pos.x}px`;
          el.style.top = `${pos.y}px`;
        }
      }
      if (this.isPanning) {
        this.state.panX = e.clientX - this.dragStartX;
        this.state.panY = e.clientY - this.dragStartY;
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
