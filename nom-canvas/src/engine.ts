import { invoke } from "@tauri-apps/api/core";
import { hashString } from "./transform";

// ── Graph Data Model (Dify pattern: declarative JSON) ───

export interface GraphNode {
  id: string;
  type: string;            // "compile" | "score" | "wire" | "build" | "custom"
  data: Record<string, unknown>;
  inputs: string[];         // IDs of nodes this depends on
  outputs: string[];        // IDs of nodes that depend on this
}

export interface GraphEdge {
  source: string;
  target: string;
  sourcePort: string;
  targetPort: string;
}

export interface WorkflowGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: Record<string, unknown>;
}

// ── Variable Pool (Dify pattern: hierarchical shared state) ──

export class VariablePool {
  private store: Map<string, unknown> = new Map();

  set(path: string[], value: unknown): void {
    this.store.set(path.join("."), value);
  }

  get(path: string[]): unknown | undefined {
    return this.store.get(path.join("."));
  }

  getNodeOutput(nodeId: string, key: string): unknown | undefined {
    return this.store.get(`${nodeId}.${key}`);
  }

  setNodeOutput(nodeId: string, key: string, value: unknown): void {
    this.store.set(`${nodeId}.${key}`, value);
  }

  clear(): void {
    this.store.clear();
  }
}

// ── Node Type Registry (Dify NodeFactory pattern) ──────

export type NodeExecutor = (
  node: GraphNode,
  pool: VariablePool
) => Promise<NodeResult>;

export interface NodeResult {
  success: boolean;
  outputs: Record<string, unknown>;
  error?: string;
}

const nodeRegistry = new Map<string, NodeExecutor>();

export function registerNodeType(type: string, executor: NodeExecutor): void {
  nodeRegistry.set(type, executor);
}

// ── Built-in node executors ─────────────────────────────

registerNodeType("compile", async (node, pool) => {
  const source = (node.data.source as string) || "";
  try {
    const result = await invoke<{
      success: boolean;
      diagnostics: string[];
      entities: string[];
    }>("compile_block", { source });
    pool.setNodeOutput(node.id, "result", result);
    pool.setNodeOutput(node.id, "entities", result.entities);
    return { success: result.success, outputs: { result, entities: result.entities } };
  } catch (e) {
    return { success: false, outputs: {}, error: String(e) };
  }
});

registerNodeType("score", async (node, pool) => {
  const source = (node.data.source as string) || "";
  try {
    const scores = await invoke<Record<string, number>>("score_block", { source });
    pool.setNodeOutput(node.id, "scores", scores);
    return { success: true, outputs: { scores } };
  } catch (e) {
    return { success: false, outputs: {}, error: String(e) };
  }
});

registerNodeType("plan", async (node, pool) => {
  const source = (node.data.source as string) || "";
  try {
    const plan = await invoke<{ nodes: number; edges: number; fusion_passes: string[] }>(
      "plan_flow", { source }
    );
    pool.setNodeOutput(node.id, "plan", plan);
    return { success: true, outputs: { plan } };
  } catch (e) {
    return { success: false, outputs: {}, error: String(e) };
  }
});

registerNodeType("search", async (node, pool) => {
  const query = (node.data.query as string) || "";
  try {
    const results = await invoke<unknown[]>("search_dict", { query });
    pool.setNodeOutput(node.id, "results", results);
    return { success: true, outputs: { results } };
  } catch (e) {
    return { success: false, outputs: {}, error: String(e) };
  }
});

registerNodeType("security", async (node, pool) => {
  const source = (node.data.source as string) || "";
  try {
    const scan = await invoke<{ findings: string[]; risk_level: string }>(
      "security_scan", { source }
    );
    pool.setNodeOutput(node.id, "scan", scan);
    return { success: true, outputs: { scan } };
  } catch (e) {
    return { success: false, outputs: {}, error: String(e) };
  }
});

// ── IS_CHANGED Fingerprinting (ComfyUI pattern) ────────

const nodeFingerprints = new Map<string, number>();

function isNodeChanged(node: GraphNode): boolean {
  const hash = hashString(JSON.stringify(node.data));
  const cached = nodeFingerprints.get(node.id);
  if (cached === hash) return false;
  nodeFingerprints.set(node.id, hash);
  return true;
}

// ── Topological Sort (ComfyUI pattern) ──────────────────

export function topologicalSort(graph: WorkflowGraph): string[] {
  const inDegree = new Map<string, number>();
  const adjacency = new Map<string, string[]>();

  for (const node of graph.nodes) {
    inDegree.set(node.id, 0);
    adjacency.set(node.id, []);
  }

  for (const edge of graph.edges) {
    const prev = inDegree.get(edge.target) || 0;
    inDegree.set(edge.target, prev + 1);
    const adj = adjacency.get(edge.source) || [];
    adj.push(edge.target);
    adjacency.set(edge.source, adj);
  }

  const queue: string[] = [];
  for (const [id, degree] of inDegree) {
    if (degree === 0) queue.push(id);
  }

  const sorted: string[] = [];
  while (queue.length > 0) {
    const nodeId = queue.shift()!;
    sorted.push(nodeId);
    for (const neighbor of adjacency.get(nodeId) || []) {
      const deg = (inDegree.get(neighbor) || 1) - 1;
      inDegree.set(neighbor, deg);
      if (deg === 0) queue.push(neighbor);
    }
  }

  return sorted;
}

// ── Subgraph Extraction (n8n partial execution pattern) ──

export function extractSubgraph(graph: WorkflowGraph, destinationId: string): WorkflowGraph {
  // BFS backward from destination to find all required ancestors
  const required = new Set<string>();
  const queue = [destinationId];
  required.add(destinationId);

  while (queue.length > 0) {
    const nodeId = queue.shift()!;
    // Find all edges pointing TO this node
    for (const edge of graph.edges) {
      if (edge.target === nodeId && !required.has(edge.source)) {
        required.add(edge.source);
        queue.push(edge.source);
      }
    }
  }

  return {
    nodes: graph.nodes.filter(n => required.has(n.id)),
    edges: graph.edges.filter(e => required.has(e.source) && required.has(e.target)),
    metadata: { ...graph.metadata, partial: true, destination: destinationId },
  };
}

// ── Execution Events (Dify streaming pattern) ───────────

export type ExecutionEvent =
  | { type: "node_start"; nodeId: string; nodeType: string }
  | { type: "node_complete"; nodeId: string; result: NodeResult }
  | { type: "node_skipped"; nodeId: string; reason: string }
  | { type: "node_error"; nodeId: string; error: string }
  | { type: "graph_complete"; results: Map<string, NodeResult> }
  | { type: "execution_cancelled"; results: Map<string, NodeResult> };

export type EventCallback = (event: ExecutionEvent) => void;

// ── Graph Engine (main orchestrator) ────────────────────

export class GraphEngine {
  private pool = new VariablePool();
  private results = new Map<string, NodeResult>();
  private listeners: EventCallback[] = [];
  private abortController: AbortController | null = null;

  onEvent(callback: EventCallback): void {
    this.listeners.push(callback);
  }

  private emit(event: ExecutionEvent): void {
    for (const cb of this.listeners) cb(event);
  }

  async execute(graph: WorkflowGraph): Promise<Map<string, NodeResult>> {
    this.abortController = new AbortController();
    this.pool.clear();
    this.results.clear();

    const order = topologicalSort(graph);
    const nodeMap = new Map(graph.nodes.map(n => [n.id, n]));

    for (const nodeId of order) {
      if (this.abortController.signal.aborted) {
        this.emit({ type: "node_skipped", nodeId, reason: "cancelled" });
        continue;
      }

      const node = nodeMap.get(nodeId);
      if (!node) continue;

      // IS_CHANGED check — skip if unchanged
      if (!isNodeChanged(node)) {
        this.emit({ type: "node_skipped", nodeId, reason: "unchanged" });
        continue;
      }

      const executor = nodeRegistry.get(node.type);
      if (!executor) {
        this.emit({ type: "node_error", nodeId, error: `unknown node type: ${node.type}` });
        continue;
      }

      this.emit({ type: "node_start", nodeId, nodeType: node.type });

      try {
        const result = await executor(node, this.pool);
        this.results.set(nodeId, result);
        this.emit({ type: "node_complete", nodeId, result });
      } catch (e) {
        const error = String(e);
        this.results.set(nodeId, { success: false, outputs: {}, error });
        this.emit({ type: "node_error", nodeId, error });
      }
    }

    const wasCancelled = this.abortController.signal.aborted;
    this.abortController = null;

    if (wasCancelled) {
      this.emit({ type: "execution_cancelled", results: this.results });
    } else {
      this.emit({ type: "graph_complete", results: this.results });
    }
    return this.results;
  }

  async executePartial(graph: WorkflowGraph, destinationNodeId: string): Promise<Map<string, NodeResult>> {
    const subgraph = extractSubgraph(graph, destinationNodeId);
    return this.execute(subgraph);
  }

  cancel(): void {
    if (this.abortController) {
      this.abortController.abort();
    }
  }
}

// ── Workflow Persistence (JSON schema) ──────────────────

export function serializeGraph(graph: WorkflowGraph): string {
  return JSON.stringify(graph, null, 2);
}

export function deserializeGraph(json: string): WorkflowGraph {
  return JSON.parse(json) as WorkflowGraph;
}
