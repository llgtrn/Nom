export interface SnapResult {
  x: number;
  y: number;
  guides: SnapGuide[];
}

export interface SnapGuide {
  type: "vertical" | "horizontal";
  position: number; // x for vertical, y for horizontal
  from: number;
  to: number;
}

interface Rect { x: number; y: number; width: number; height: number; }

const SNAP_THRESHOLD = 8; // pixels
const GRID_SIZE = 24; // matches dot grid

/** Snap a moving rect to grid and nearby rects */
export function snapToGuides(moving: Rect, others: Rect[], gridSnap = true): SnapResult {
  let { x, y } = moving;
  const guides: SnapGuide[] = [];

  // Grid snap
  if (gridSnap) {
    const gx = Math.round(x / GRID_SIZE) * GRID_SIZE;
    const gy = Math.round(y / GRID_SIZE) * GRID_SIZE;
    if (Math.abs(x - gx) < SNAP_THRESHOLD) x = gx;
    if (Math.abs(y - gy) < SNAP_THRESHOLD) y = gy;
  }

  // Edge + center alignment with other rects
  const movingCx = x + moving.width / 2;
  const movingCy = y + moving.height / 2;
  const movingRight = x + moving.width;
  const movingBottom = y + moving.height;

  for (const other of others) {
    const oCx = other.x + other.width / 2;
    const oCy = other.y + other.height / 2;
    const oRight = other.x + other.width;
    const oBottom = other.y + other.height;

    // Vertical alignment (x-axis snaps)
    for (const [movVal, otherVal] of [
      [x, other.x], [x, oRight], [movingRight, other.x], [movingRight, oRight],
      [movingCx, oCx],
    ] as [number, number][]) {
      if (Math.abs(movVal - otherVal) < SNAP_THRESHOLD) {
        x += otherVal - movVal;
        guides.push({
          type: "vertical",
          position: otherVal,
          from: Math.min(y, other.y),
          to: Math.max(movingBottom, oBottom),
        });
        break;
      }
    }

    // Horizontal alignment (y-axis snaps)
    for (const [movVal, otherVal] of [
      [y, other.y], [y, oBottom], [movingBottom, other.y], [movingBottom, oBottom],
      [movingCy, oCy],
    ] as [number, number][]) {
      if (Math.abs(movVal - otherVal) < SNAP_THRESHOLD) {
        y += otherVal - movVal;
        guides.push({
          type: "horizontal",
          position: otherVal,
          from: Math.min(x, other.x),
          to: Math.max(movingRight, oRight),
        });
        break;
      }
    }
  }

  return { x, y, guides };
}

/** Align selected elements to each other */
export type AlignAction = "left" | "center-h" | "right" | "top" | "center-v" | "bottom" | "distribute-h" | "distribute-v";

export function alignElements(rects: Rect[], action: AlignAction): Rect[] {
  if (rects.length < 2) return rects;
  const result = rects.map(r => ({ ...r }));

  switch (action) {
    case "left": {
      const minX = Math.min(...result.map(r => r.x));
      result.forEach(r => r.x = minX);
      break;
    }
    case "right": {
      const maxRight = Math.max(...result.map(r => r.x + r.width));
      result.forEach(r => r.x = maxRight - r.width);
      break;
    }
    case "center-h": {
      const avgCx = result.reduce((s, r) => s + r.x + r.width / 2, 0) / result.length;
      result.forEach(r => r.x = avgCx - r.width / 2);
      break;
    }
    case "top": {
      const minY = Math.min(...result.map(r => r.y));
      result.forEach(r => r.y = minY);
      break;
    }
    case "bottom": {
      const maxBottom = Math.max(...result.map(r => r.y + r.height));
      result.forEach(r => r.y = maxBottom - r.height);
      break;
    }
    case "center-v": {
      const avgCy = result.reduce((s, r) => s + r.y + r.height / 2, 0) / result.length;
      result.forEach(r => r.y = avgCy - r.height / 2);
      break;
    }
    case "distribute-h": {
      result.sort((a, b) => a.x - b.x);
      const totalWidth = result.reduce((s, r) => s + r.width, 0);
      const span = result[result.length - 1].x + result[result.length - 1].width - result[0].x;
      const gap = (span - totalWidth) / (result.length - 1);
      let cx = result[0].x + result[0].width;
      for (let i = 1; i < result.length - 1; i++) {
        result[i].x = cx + gap;
        cx = result[i].x + result[i].width;
      }
      break;
    }
    case "distribute-v": {
      result.sort((a, b) => a.y - b.y);
      const totalHeight = result.reduce((s, r) => s + r.height, 0);
      const spanV = result[result.length - 1].y + result[result.length - 1].height - result[0].y;
      const gapV = (spanV - totalHeight) / (result.length - 1);
      let cy = result[0].y + result[0].height;
      for (let i = 1; i < result.length - 1; i++) {
        result[i].y = cy + gapV;
        cy = result[i].y + result[i].height;
      }
      break;
    }
  }
  return result;
}

/** Render snap guides as temporary SVG lines */
export function renderSnapGuides(container: SVGSVGElement, guides: SnapGuide[]): void {
  // Remove existing guides
  container.querySelectorAll(".snap-guide").forEach(el => el.remove());
  for (const guide of guides) {
    const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
    line.classList.add("snap-guide");
    if (guide.type === "vertical") {
      line.setAttribute("x1", String(guide.position));
      line.setAttribute("y1", String(guide.from));
      line.setAttribute("x2", String(guide.position));
      line.setAttribute("y2", String(guide.to));
    } else {
      line.setAttribute("x1", String(guide.from));
      line.setAttribute("y1", String(guide.position));
      line.setAttribute("x2", String(guide.to));
      line.setAttribute("y2", String(guide.position));
    }
    container.appendChild(line);
  }
}
