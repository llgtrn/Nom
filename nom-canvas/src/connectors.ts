export type ConnectorType = "file" | "http" | "sqlite" | "json" | "csv" | "nomdict" | "custom";

export interface ConnectorConfig {
  type: ConnectorType;
  name: string;
  params: Record<string, string>;
}

export interface ConnectorResult {
  success: boolean;
  data: unknown;
  error?: string;
  rowCount?: number;
}

export type ConnectorHandler = (config: ConnectorConfig) => Promise<ConnectorResult>;

const registry = new Map<ConnectorType, ConnectorHandler>();

export function registerConnector(type: ConnectorType, handler: ConnectorHandler): void {
  registry.set(type, handler);
}

export async function executeConnector(config: ConnectorConfig): Promise<ConnectorResult> {
  const handler = registry.get(config.type);
  if (!handler) return { success: false, data: null, error: `Unknown connector: ${config.type}` };
  return handler(config);
}

export function getAvailableConnectors(): ConnectorType[] {
  return Array.from(registry.keys());
}

// Built-in connectors
import { invoke } from "@tauri-apps/api/core";

registerConnector("nomdict", async (config) => {
  try {
    const results = await invoke<unknown[]>("search_dict", { query: config.params.query || "" });
    return { success: true, data: results, rowCount: (results as unknown[]).length };
  } catch (e) { return { success: false, data: null, error: String(e) }; }
});

registerConnector("json", async (config) => {
  try {
    const data = JSON.parse(config.params.data || "{}");
    return { success: true, data };
  } catch (e) { return { success: false, data: null, error: String(e) }; }
});

registerConnector("http", async (config) => {
  try {
    const resp = await fetch(config.params.url || "");
    const data = await resp.json();
    return { success: true, data, rowCount: Array.isArray(data) ? data.length : 1 };
  } catch (e) { return { success: false, data: null, error: String(e) }; }
});
