import type { BlockPosition } from "./canvas";
import type { Viewport } from "./canvas";

export class Minimap {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private width: number;
  private height: number;

  constructor(containerId: string, width = 200, height = 150) {
    this.width = width;
    this.height = height;

    const container = document.getElementById(containerId);
    this.canvas = document.createElement("canvas");
    this.canvas.className = "minimap-canvas";
    this.canvas.width = width;
    this.canvas.height = height;
    this.ctx = this.canvas.getContext("2d")!;

    if (container) {
      container.appendChild(this.canvas);
    } else {
      // Create floating minimap
      const wrapper = document.createElement("div");
      wrapper.className = "minimap-wrapper";
      wrapper.id = containerId;
      wrapper.appendChild(this.canvas);
      document.getElementById("app")?.appendChild(wrapper);
    }
  }

  /** Render the minimap from block positions and viewport */
  render(blocks: Map<string, BlockPosition>, viewport: { panX: number; panY: number; zoom: number; width: number; height: number }): void {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.width, this.height);

    // Calculate bounds of all blocks
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const block of blocks.values()) {
      minX = Math.min(minX, block.x);
      minY = Math.min(minY, block.y);
      maxX = Math.max(maxX, block.x + block.width);
      maxY = Math.max(maxY, block.y + block.height);
    }

    if (!isFinite(minX)) {
      // No blocks — draw empty minimap
      ctx.fillStyle = "#0F172A";
      ctx.fillRect(0, 0, this.width, this.height);
      ctx.fillStyle = "#334155";
      ctx.font = "10px 'IBM Plex Sans'";
      ctx.textAlign = "center";
      ctx.fillText("No blocks", this.width / 2, this.height / 2);
      return;
    }

    // Add padding
    const pad = 50;
    minX -= pad; minY -= pad;
    maxX += pad; maxY += pad;

    const worldW = maxX - minX;
    const worldH = maxY - minY;
    const scaleX = this.width / worldW;
    const scaleY = this.height / worldH;
    const scale = Math.min(scaleX, scaleY);

    // Background
    ctx.fillStyle = "#0F172A";
    ctx.fillRect(0, 0, this.width, this.height);

    // Draw blocks as small rectangles
    for (const block of blocks.values()) {
      const bx = (block.x - minX) * scale;
      const by = (block.y - minY) * scale;
      const bw = block.width * scale;
      const bh = block.height * scale;

      ctx.fillStyle = "#1E293B";
      ctx.strokeStyle = "#334155";
      ctx.lineWidth = 1;
      ctx.fillRect(bx, by, bw, bh);
      ctx.strokeRect(bx, by, bw, bh);
    }

    // Draw viewport rectangle
    const vpLeft = (-viewport.panX / viewport.zoom - minX) * scale;
    const vpTop = (-viewport.panY / viewport.zoom - minY) * scale;
    const vpW = (viewport.width / viewport.zoom) * scale;
    const vpH = (viewport.height / viewport.zoom) * scale;

    ctx.strokeStyle = "#22C55E";
    ctx.lineWidth = 2;
    ctx.strokeRect(vpLeft, vpTop, vpW, vpH);
  }

  /** Get the canvas element for external mounting */
  getCanvas(): HTMLCanvasElement { return this.canvas; }
}
