import { SpatialCanvas, type BlockPosition } from "./canvas";
import { WiringEngine, type Wire } from "./wiring";
import {
  GraphEngine,
  type WorkflowGraph,
  type GraphNode,
  type GraphEdge,
  type ExecutionEvent,
  serializeGraph,
  deserializeGraph,
} from "./engine";
import { scoreAndRender } from "./quality";

/** Maps canvas blocks to graph nodes */
export interface BlockNodeBinding {
  blockId: string;
  nodeId: string;
  nodeType: string;
  source: string; // block content
}

export class GraphUI {
  private canvas: SpatialCanvas;
  private wiring: WiringEngine;
  private engine: GraphEngine;
  private bindings: Map<string, BlockNodeBinding> = new Map();
  private statusEl: HTMLElement | null;

  constructor(canvasContainerId: string, statusElementId?: string) {
    this.canvas = new SpatialCanvas(canvasContainerId);
    this.wiring = new WiringEngine(canvasContainerId);
    this.engine = new GraphEngine();
    this.statusEl = statusElementId
      ? document.getElementById(statusElementId)
      : null;

    // Wire execution events to UI feedback
    this.engine.onEvent((event) => this.handleEvent(event));
  }

  /** Add a block to the canvas and bind it to a graph node */
  addBlock(
    id: string,
    type: string,
    x: number,
    y: number,
    source: string = ""
  ): HTMLElement {
    const contentEl = this.canvas.addBlock(id, x, y);
    this.bindings.set(id, {
      blockId: id,
      nodeId: id, // 1:1 mapping for simplicity
      nodeType: type,
      source,
    });
    return contentEl;
  }

  /** Remove a block and its wires */
  removeBlock(id: string): void {
    // Remove all wires connected to this block
    const wires = this.wiring.getBlockWires(id);
    for (const wire of wires) {
      this.wiring.removeWire(wire.id);
    }
    this.canvas.removeBlock(id);
    this.bindings.delete(id);
  }

  /** Update block source content */
  updateBlockSource(blockId: string, source: string): void {
    const binding = this.bindings.get(blockId);
    if (binding) {
      binding.source = source;
    }
  }

  /** Build a WorkflowGraph from current canvas state */
  buildGraph(): WorkflowGraph {
    const nodes: GraphNode[] = [];
    const edges: GraphEdge[] = [];
    const wires = this.wiring.getWires();

    // Build adjacency from wires
    const inputMap = new Map<string, string[]>();
    const outputMap = new Map<string, string[]>();

    for (const wire of wires) {
      const inputs = inputMap.get(wire.toBlockId) || [];
      inputs.push(wire.fromBlockId);
      inputMap.set(wire.toBlockId, inputs);

      const outputs = outputMap.get(wire.fromBlockId) || [];
      outputs.push(wire.toBlockId);
      outputMap.set(wire.fromBlockId, outputs);

      edges.push({
        source: wire.fromBlockId,
        target: wire.toBlockId,
        sourcePort: wire.fromPort,
        targetPort: wire.toPort,
      });
    }

    for (const [id, binding] of this.bindings) {
      nodes.push({
        id,
        type: binding.nodeType,
        data: { source: binding.source },
        inputs: inputMap.get(id) || [],
        outputs: outputMap.get(id) || [],
      });
    }

    return { nodes, edges, metadata: { createdAt: new Date().toISOString() } };
  }

  /** Execute the current graph */
  async executeGraph(): Promise<void> {
    const graph = this.buildGraph();
    if (this.statusEl) {
      this.statusEl.textContent = `// executing ${graph.nodes.length} nodes...`;
    }
    await this.engine.execute(graph);
  }

  /** Save graph to JSON string */
  saveGraph(): string {
    return serializeGraph(this.buildGraph());
  }

  /** Load graph from JSON string */
  loadGraph(json: string): void {
    const graph = deserializeGraph(json);
    // Clear existing state
    for (const id of this.bindings.keys()) {
      this.removeBlock(id);
    }
    // Recreate blocks
    let x = 50, y = 50;
    for (const node of graph.nodes) {
      this.addBlock(node.id, node.type, x, y, (node.data.source as string) || "");
      x += 700;
      if (x > 2000) { x = 50; y += 300; }
    }
    // Note: wires need to be recreated visually from edges
    // This requires block position knowledge for SVG endpoints
  }

  /** Get the underlying canvas for direct manipulation */
  getCanvas(): SpatialCanvas { return this.canvas; }

  /** Get the underlying wiring engine */
  getWiring(): WiringEngine { return this.wiring; }

  /** Get the underlying execution engine */
  getEngine(): GraphEngine { return this.engine; }

  private handleEvent(event: ExecutionEvent): void {
    switch (event.type) {
      case "node_start":
        this.highlightBlock(event.nodeId, "executing");
        if (this.statusEl) {
          this.statusEl.textContent = `// running: ${event.nodeId} (${event.nodeType})`;
        }
        break;
      case "node_complete":
        this.highlightBlock(
          event.nodeId,
          event.result.success ? "success" : "error"
        );
        break;
      case "node_skipped":
        this.highlightBlock(event.nodeId, "skipped");
        break;
      case "node_error":
        this.highlightBlock(event.nodeId, "error");
        break;
      case "graph_complete":
        if (this.statusEl) {
          const total = event.results.size;
          const succeeded = Array.from(event.results.values()).filter(r => r.success).length;
          this.statusEl.textContent = `// done: ${succeeded}/${total} nodes succeeded`;
        }
        break;
    }
  }

  private highlightBlock(blockId: string, status: string): void {
    const el = document.querySelector(`[data-block-id="${blockId}"]`);
    if (!el) return;
    // Remove previous status classes
    el.classList.remove("block-executing", "block-success", "block-error", "block-skipped");
    el.classList.add(`block-${status}`);
    // Auto-clear highlight after 3s for non-error states
    if (status !== "error") {
      setTimeout(() => el.classList.remove(`block-${status}`), 3000);
    }
  }
}
