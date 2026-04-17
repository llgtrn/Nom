import type { WorkflowGraph, GraphNode, GraphEdge } from "./engine";

export interface NodePosition {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export class NodeGraphRenderer {
  private container: HTMLElement;
  private svgLayer: SVGSVGElement;
  private positions: Map<string, NodePosition> = new Map();

  constructor(containerId: string) {
    this.container = document.getElementById(containerId) || document.createElement("div");
    this.container.className = "node-graph";

    this.svgLayer = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    this.svgLayer.classList.add("node-graph-svg");
    this.svgLayer.setAttribute("width", "100%");
    this.svgLayer.setAttribute("height", "100%");
    this.container.appendChild(this.svgLayer);
  }

  render(graph: WorkflowGraph): void {
    this.clear();
    this.autoLayout(graph);
    this.renderNodes(graph.nodes);
    this.renderEdges(graph.edges);
  }

  private autoLayout(graph: WorkflowGraph): void {
    const gap = 200;
    const nodeW = 160;
    const nodeH = 80;
    // Simple left-to-right topological layout
    const levels = new Map<string, number>();
    const inDegree = new Map<string, number>();
    for (const n of graph.nodes) { inDegree.set(n.id, 0); levels.set(n.id, 0); }
    for (const e of graph.edges) { inDegree.set(e.target, (inDegree.get(e.target) || 0) + 1); }

    // BFS level assignment
    const queue = graph.nodes.filter(n => (inDegree.get(n.id) || 0) === 0).map(n => n.id);
    while (queue.length > 0) {
      const nodeId = queue.shift()!;
      const level = levels.get(nodeId) || 0;
      for (const e of graph.edges) {
        if (e.source === nodeId) {
          const newLevel = level + 1;
          if (newLevel > (levels.get(e.target) || 0)) levels.set(e.target, newLevel);
          const deg = (inDegree.get(e.target) || 1) - 1;
          inDegree.set(e.target, deg);
          if (deg === 0) queue.push(e.target);
        }
      }
    }

    // Position by level
    const levelCounts = new Map<number, number>();
    for (const node of graph.nodes) {
      const level = levels.get(node.id) || 0;
      const row = levelCounts.get(level) || 0;
      levelCounts.set(level, row + 1);
      this.positions.set(node.id, {
        id: node.id,
        x: 40 + level * gap,
        y: 40 + row * (nodeH + 40),
        width: nodeW,
        height: nodeH,
      });
    }
  }

  private renderNodes(nodes: GraphNode[]): void {
    for (const node of nodes) {
      const pos = this.positions.get(node.id);
      if (!pos) continue;
      const el = document.createElement("div");
      el.className = `graph-node node-type-${node.type}`;
      el.style.cssText = `left:${pos.x}px;top:${pos.y}px;width:${pos.width}px;height:${pos.height}px;`;

      const title = document.createElement("div");
      title.className = "graph-node-title";
      title.textContent = node.type;
      el.appendChild(title);

      const label = document.createElement("div");
      label.className = "graph-node-label";
      label.textContent = node.id;
      el.appendChild(label);

      this.container.appendChild(el);
    }
  }

  private renderEdges(edges: GraphEdge[]): void {
    for (const edge of edges) {
      const from = this.positions.get(edge.source);
      const to = this.positions.get(edge.target);
      if (!from || !to) continue;

      const x1 = from.x + from.width;
      const y1 = from.y + from.height / 2;
      const x2 = to.x;
      const y2 = to.y + to.height / 2;
      const dx = Math.abs(x2 - x1) * 0.4;

      const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.setAttribute("d", `M${x1},${y1} C${x1 + dx},${y1} ${x2 - dx},${y2} ${x2},${y2}`);
      path.classList.add("graph-edge");
      this.svgLayer.appendChild(path);
    }
  }

  clear(): void {
    this.svgLayer.replaceChildren();
    this.container.querySelectorAll(".graph-node").forEach(el => el.remove());
    this.positions.clear();
  }

  getElement(): HTMLElement { return this.container; }
}
