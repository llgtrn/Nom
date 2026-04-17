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

// More comprehensive private IP check
function isPrivateHost(hostname: string): boolean {
  const h = hostname.toLowerCase();
  // Loopback
  if (h === "localhost" || h === "127.0.0.1" || h === "0.0.0.0" || h === "[::1]" || h === "::1") return true;
  // RFC 1918
  if (h.startsWith("10.") || h.startsWith("192.168.")) return true;
  if (/^172\.(1[6-9]|2\d|3[01])\./.test(h)) return true;
  // Link-local
  if (h.startsWith("169.254.")) return true;
  // Octal notation (0177.0.0.1 = 127.0.0.1)
  if (/^0\d/.test(h)) return true;
  // IPv6 private
  if (h.startsWith("fe80:") || h.startsWith("fc") || h.startsWith("fd")) return true;
  return false;
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
  const url = config.params.url || "";
  // Validate URL before fetch — prevent SSRF
  try {
    const parsed = new URL(url);
    if (!["http:", "https:"].includes(parsed.protocol)) {
      return { success: false, data: null, error: "Only http/https URLs allowed" };
    }
    // Block localhost/private IPs
    if (isPrivateHost(parsed.hostname)) {
      return { success: false, data: null, error: "Private/local URLs not allowed" };
    }
  } catch {
    return { success: false, data: null, error: "Invalid URL" };
  }
  try {
    const resp = await fetch(url);
    const data = await resp.json();
    return { success: true, data, rowCount: Array.isArray(data) ? data.length : 1 };
  } catch (e) { return { success: false, data: null, error: String(e) }; }
});
