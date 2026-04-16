/** Intent categories (WrenAI pattern: 4-way classification) */
export type IntentCategory = "compile" | "query" | "create" | "navigate";

export interface IntentResult {
  category: IntentCategory;
  confidence: number;
  suggestedAction: string;
  entities: string[];
}

// Rule-based patterns for each intent (deterministic floor before LLM)
const COMPILE_PATTERNS = [
  /^the\s+(function|module|concept|screen|data|event|media|property|scenario)\s+/i,
  /\breturns?\b/i,
  /\bgiven\b.*\bof\b/i,
  /\brequires?\b/i,
  /\bensures?\b/i,
];

const QUERY_PATTERNS = [
  /\bfind\b/i,
  /\bsearch\b/i,
  /\blook\s*up\b/i,
  /\bshow\s+me\b/i,
  /\bwhat\s+is\b/i,
  /\bhow\s+does\b/i,
  /\blist\b.*\b(functions?|modules?|entities)\b/i,
];

const CREATE_PATTERNS = [
  /\bcreate\b/i,
  /\bnew\s+(block|function|module|app)\b/i,
  /\badd\s+a?\s*(block|function|module)\b/i,
  /\bbuild\b/i,
  /\bgenerate\b/i,
  /\bdesign\b/i,
];

const NAVIGATE_PATTERNS = [
  /\bgo\s+to\b/i,
  /\bopen\b/i,
  /\bnavigate\b/i,
  /\bjump\s+to\b/i,
  /\bswitch\s+to\b/i,
  /\bzoom\b/i,
  /\bpan\b/i,
];

function matchScore(input: string, patterns: RegExp[]): number {
  let hits = 0;
  for (const p of patterns) {
    if (p.test(input)) hits++;
  }
  return hits / patterns.length;
}

/** Classify user input into an intent category */
export function classifyIntent(input: string): IntentResult {
  const trimmed = input.trim();
  if (!trimmed) {
    return { category: "compile", confidence: 0, suggestedAction: "type_prose", entities: [] };
  }

  const scores: [IntentCategory, number][] = [
    ["compile", matchScore(trimmed, COMPILE_PATTERNS)],
    ["query", matchScore(trimmed, QUERY_PATTERNS)],
    ["create", matchScore(trimmed, CREATE_PATTERNS)],
    ["navigate", matchScore(trimmed, NAVIGATE_PATTERNS)],
  ];

  scores.sort((a, b) => b[1] - a[1]);
  const [category, confidence] = scores[0];

  // Extract entity names (snake_case or camelCase words)
  const entities = (trimmed.match(/\b[a-z][a-z0-9]*(?:_[a-z0-9]+)+\b/g) || []);

  const suggestedAction = {
    compile: "compile_block",
    query: "search_dict",
    create: "add_block",
    navigate: "pan_to",
  }[category];

  return { category, confidence, suggestedAction, entities };
}

/** ReAct-style prompt formatter (LlamaIndex pattern) */
export interface ToolDescription {
  name: string;
  description: string;
  parameters: string[];
}

const CANVAS_TOOLS: ToolDescription[] = [
  { name: "compile_block", description: "Compile .nomx prose to entities via S1-S6 pipeline", parameters: ["source: string"] },
  { name: "search_dict", description: "Search dictionary entities by word or pattern", parameters: ["query: string"] },
  { name: "score_block", description: "Compute 8-dimension quality scores", parameters: ["source: string"] },
  { name: "wire_check", description: "Check compatibility between two blocks", parameters: ["fromHash: string", "toHash: string"] },
  { name: "plan_flow", description: "Plan execution flow from source", parameters: ["source: string"] },
  { name: "security_scan", description: "Scan source for security issues", parameters: ["source: string"] },
  { name: "hover_info", description: "Get entity info for a word", parameters: ["word: string"] },
  { name: "complete_word", description: "Get completions for a prefix", parameters: ["prefix: string"] },
];

/** Format tool descriptions for LLM prompt injection */
export function formatToolPrompt(): string {
  const toolDescs = CANVAS_TOOLS.map(t =>
    `- ${t.name}(${t.parameters.join(", ")}): ${t.description}`
  ).join("\n");
  return `Available tools:\n${toolDescs}\n\nTo use a tool, respond with: ACTION: <tool_name> PARAMS: <json>`;
}

/** Parse an LLM response to extract action + params */
export function parseActionResponse(response: string): { tool: string; params: Record<string, string> } | null {
  const actionMatch = response.match(/ACTION:\s*(\w+)/);
  const paramsMatch = response.match(/PARAMS:\s*(\{[^}]+\})/);
  if (!actionMatch) return null;
  const tool = actionMatch[1];
  let params: Record<string, string> = {};
  if (paramsMatch) {
    try { params = JSON.parse(paramsMatch[1]); } catch { /* ignore */ }
  }
  return { tool, params };
}

export function getCanvasTools(): ToolDescription[] {
  return CANVAS_TOOLS;
}
