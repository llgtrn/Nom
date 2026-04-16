import type { CanvasElement, ElementBase } from "./elements";
import { getTransformHandles } from "./elements";
import type { Viewport } from "./canvas";

export class CanvasRenderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private viewport: Viewport;
  private animFrameId: number | null = null;
  private dirty = true;

  constructor(canvas: HTMLCanvasElement, viewport: Viewport) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    this.viewport = viewport;
  }

  /** Mark as needing re-render */
  markDirty(): void {
    this.dirty = true;
  }

  /** Start render loop (requestAnimationFrame) */
  start(): void {
    const loop = () => {
      if (this.dirty) {
        this.dirty = false;
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
      }
      this.animFrameId = requestAnimationFrame(loop);
    };
    this.animFrameId = requestAnimationFrame(loop);
  }

  /** Stop render loop */
  stop(): void {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
  }

  /** Render all elements */
  renderAll(elements: CanvasElement[], selectedIds: Set<string>): void {
    const ctx = this.ctx;
    const vp = this.viewport;

    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.save();

    // Apply viewport transform
    ctx.translate(vp.panX, vp.panY);
    ctx.scale(vp.zoom, vp.zoom);

    for (const el of elements) {
      if (el.isDeleted) continue;
      this.renderElement(ctx, el);
      if (selectedIds.has(el.id)) {
        this.renderSelection(ctx, el);
      }
    }

    ctx.restore();
  }

  /** Render a single element */
  renderElement(ctx: CanvasRenderingContext2D, el: CanvasElement): void {
    ctx.save();

    // Apply element rotation
    if (el.angle !== 0) {
      const cx = el.x + el.width / 2;
      const cy = el.y + el.height / 2;
      ctx.translate(cx, cy);
      ctx.rotate(el.angle);
      ctx.translate(-cx, -cy);
    }

    ctx.globalAlpha = el.opacity;
    ctx.strokeStyle = el.strokeColor;
    ctx.fillStyle = el.fillColor;
    ctx.lineWidth = el.strokeWidth;

    switch (el.type) {
      case "rectangle":
        this.drawRectangle(ctx, el);
        break;
      case "ellipse":
        this.drawEllipse(ctx, el);
        break;
      case "diamond":
        this.drawDiamond(ctx, el);
        break;
      case "text":
        this.drawText(ctx, el);
        break;
      case "arrow":
        this.drawArrow(ctx, el);
        break;
      case "line":
        this.drawLine(ctx, el);
        break;
      case "connector":
        this.drawConnector(ctx, el);
        break;
      case "image":
        this.drawImage(ctx, el);
        break;
    }

    ctx.restore();
  }

  private drawRectangle(ctx: CanvasRenderingContext2D, el: ElementBase & { borderRadius: number }): void {
    const r = el.borderRadius;
    if (r > 0) {
      ctx.beginPath();
      ctx.roundRect(el.x, el.y, el.width, el.height, r);
      if (el.fillColor !== "transparent") ctx.fill();
      ctx.stroke();
    } else {
      if (el.fillColor !== "transparent") ctx.fillRect(el.x, el.y, el.width, el.height);
      ctx.strokeRect(el.x, el.y, el.width, el.height);
    }
  }

  private drawEllipse(ctx: CanvasRenderingContext2D, el: ElementBase): void {
    ctx.beginPath();
    ctx.ellipse(
      el.x + el.width / 2, el.y + el.height / 2,
      el.width / 2, el.height / 2,
      0, 0, Math.PI * 2
    );
    if (el.fillColor !== "transparent") ctx.fill();
    ctx.stroke();
  }

  private drawDiamond(ctx: CanvasRenderingContext2D, el: ElementBase): void {
    const cx = el.x + el.width / 2;
    const cy = el.y + el.height / 2;
    ctx.beginPath();
    ctx.moveTo(cx, el.y);
    ctx.lineTo(el.x + el.width, cy);
    ctx.lineTo(cx, el.y + el.height);
    ctx.lineTo(el.x, cy);
    ctx.closePath();
    if (el.fillColor !== "transparent") ctx.fill();
    ctx.stroke();
  }

  private drawText(ctx: CanvasRenderingContext2D, el: ElementBase & { text: string; fontSize: number; fontFamily: string; textAlign: "left" | "center" | "right" }): void {
    ctx.font = `${el.fontSize}px ${el.fontFamily}`;
    ctx.textAlign = el.textAlign as CanvasTextAlign;
    ctx.fillStyle = el.strokeColor; // text uses stroke color
    const lines = el.text.split("\n");
    const lineHeight = el.fontSize * 1.4;
    for (let i = 0; i < lines.length; i++) {
      ctx.fillText(lines[i], el.x, el.y + (i + 1) * lineHeight, el.width);
    }
  }

  private drawArrow(ctx: CanvasRenderingContext2D, el: ElementBase & { points: [number, number][] }): void {
    const pts = el.points;
    if (pts.length < 2) return;

    ctx.beginPath();
    ctx.moveTo(el.x + pts[0][0], el.y + pts[0][1]);
    for (let i = 1; i < pts.length; i++) {
      ctx.lineTo(el.x + pts[i][0], el.y + pts[i][1]);
    }
    ctx.stroke();

    // Arrowhead
    const last = pts[pts.length - 1];
    const prev = pts[pts.length - 2];
    const angle = Math.atan2(last[1] - prev[1], last[0] - prev[0]);
    const headLen = 10;
    const tipX = el.x + last[0];
    const tipY = el.y + last[1];
    ctx.beginPath();
    ctx.moveTo(tipX, tipY);
    ctx.lineTo(tipX - headLen * Math.cos(angle - 0.4), tipY - headLen * Math.sin(angle - 0.4));
    ctx.moveTo(tipX, tipY);
    ctx.lineTo(tipX - headLen * Math.cos(angle + 0.4), tipY - headLen * Math.sin(angle + 0.4));
    ctx.stroke();
  }

  private drawLine(ctx: CanvasRenderingContext2D, el: ElementBase & { points: [number, number][] }): void {
    const pts = el.points;
    if (pts.length < 2) return;
    ctx.beginPath();
    ctx.moveTo(el.x + pts[0][0], el.y + pts[0][1]);
    for (let i = 1; i < pts.length; i++) {
      ctx.lineTo(el.x + pts[i][0], el.y + pts[i][1]);
    }
    ctx.stroke();
  }

  private drawConnector(ctx: CanvasRenderingContext2D, el: ElementBase): void {
    const midX = el.x + el.width / 2;
    ctx.beginPath();
    ctx.moveTo(el.x, el.y + el.height / 2);
    ctx.bezierCurveTo(midX, el.y, midX, el.y + el.height, el.x + el.width, el.y + el.height / 2);
    ctx.stroke();
  }

  private drawImage(ctx: CanvasRenderingContext2D, el: ElementBase): void {
    ctx.strokeRect(el.x, el.y, el.width, el.height);
    ctx.fillStyle = "#334155";
    ctx.font = "12px 'JetBrains Mono'";
    ctx.textAlign = "center";
    ctx.fillText("[image]", el.x + el.width / 2, el.y + el.height / 2);
  }

  /** Render selection indicators (handles + outline) */
  private renderSelection(ctx: CanvasRenderingContext2D, el: ElementBase): void {
    ctx.save();
    ctx.strokeStyle = "#22C55E";
    ctx.lineWidth = 1.5;
    ctx.setLineDash([4, 4]);
    ctx.strokeRect(el.x - 2, el.y - 2, el.width + 4, el.height + 4);
    ctx.setLineDash([]);

    // Transform handles
    const handles = getTransformHandles(el, 8);
    for (const h of handles) {
      ctx.fillStyle = h.direction === "rotation" ? "#22C55E" : "#F8FAFC";
      ctx.strokeStyle = "#22C55E";
      ctx.lineWidth = 1;
      const size = h.direction === "rotation" ? 10 : 8;
      if (h.direction === "rotation") {
        ctx.beginPath();
        ctx.arc(h.x + size / 2, h.y + size / 2, size / 2, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
      } else {
        ctx.fillRect(h.x, h.y, size, size);
        ctx.strokeRect(h.x, h.y, size, size);
      }
    }
    ctx.restore();
  }
}
