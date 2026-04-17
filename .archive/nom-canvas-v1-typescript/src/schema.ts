import { Schema, NodeSpec, MarkSpec } from "prosemirror-model";
import { addListNodes } from "prosemirror-schema-list";

// ── Node specs ───────────────────────────────────────────

const proseBlock: NodeSpec = {
  content: "inline*",
  group: "block",
  parseDOM: [{ tag: "div.prose-block" }],
  toDOM() { return ["div", { class: "prose-block" }, 0]; },
};

const nomxBlock: NodeSpec = {
  content: "text*",
  group: "block",
  attrs: {
    compiled: { default: false },
    entities: { default: [] },
    diagnostics: { default: [] },
  },
  parseDOM: [{ tag: "div.nomx-block" }],
  toDOM(node) {
    const cls = node.attrs.compiled ? "nomx-block compiled" : "nomx-block";
    return ["div", { class: cls }, ["code", 0]];
  },
};

const mediaNodeBlock: NodeSpec = {
  group: "block",
  attrs: {
    src: { default: "" },
    mime: { default: "" },
    hash: { default: "" },
  },
  parseDOM: [{ tag: "div.media-node-block" }],
  toDOM(node) {
    return ["div", { class: "media-node-block" },
      ["span", { class: "media-label" }, `Media: ${node.attrs.hash.slice(0, 8) || "empty"}`]
    ];
  },
};

const appPreviewBlock: NodeSpec = {
  group: "block",
  attrs: {
    manifestHash: { default: "" },
    status: { default: "idle" }, // idle | building | ready | error
  },
  parseDOM: [{ tag: "div.app-preview-block" }],
  toDOM(node) {
    return ["div", { class: `app-preview-block ${node.attrs.status}` },
      ["span", {}, `App Preview [${node.attrs.status}]`]
    ];
  },
};

const drawingBlock: NodeSpec = {
  group: "block",
  attrs: {
    width: { default: 400 },
    height: { default: 300 },
  },
  parseDOM: [{ tag: "div.drawing-block" }],
  toDOM(node) {
    return ["div", { class: "drawing-block" },
      ["canvas", { width: String(node.attrs.width), height: String(node.attrs.height) }]
    ];
  },
};

const codeBlock: NodeSpec = {
  content: "text*",
  group: "block",
  attrs: { language: { default: "nomx" } },
  parseDOM: [{ tag: "pre.code-block" }],
  toDOM(node) { return ["pre", { class: `code-block lang-${node.attrs.language}` }, ["code", 0]]; },
};

const terminalBlock: NodeSpec = {
  content: "text*",
  group: "block",
  attrs: { title: { default: "Terminal" } },
  parseDOM: [{ tag: "div.terminal-block" }],
  toDOM() { return ["div", { class: "terminal-block" }, ["pre", 0]]; },
};

const graphBlock: NodeSpec = {
  group: "block",
  atom: true,
  attrs: { graphJson: { default: "" } },
  parseDOM: [{ tag: "div.graph-block" }],
  toDOM() { return ["div", { class: "graph-block" }, "[workflow graph]"]; },
};

const tableBlock: NodeSpec = {
  content: "block+",
  group: "block",
  attrs: { rows: { default: 2 }, cols: { default: 3 } },
  parseDOM: [{ tag: "table.table-block" }],
  toDOM() { return ["table", { class: "table-block" }, ["tbody", 0]]; },
};

const divider: NodeSpec = {
  group: "block",
  atom: true,
  parseDOM: [{ tag: "hr.divider-block" }],
  toDOM() { return ["hr", { class: "divider-block" }]; },
};

const calloutBlock: NodeSpec = {
  content: "inline*",
  group: "block",
  attrs: { variant: { default: "info" } }, // info | warning | error
  parseDOM: [{ tag: "div.callout-block" }],
  toDOM(node) {
    return ["div", { class: `callout-block ${node.attrs.variant}` }, 0];
  },
};

const embedBlock: NodeSpec = {
  group: "block",
  atom: true,
  attrs: { url: { default: "" }, title: { default: "" } },
  parseDOM: [{ tag: "div.embed-block" }],
  toDOM(node) {
    return ["div", { class: "embed-block" },
      ["span", {}, node.attrs.title || node.attrs.url || "Embed"]
    ];
  },
};

const heading: NodeSpec = {
  content: "inline*",
  group: "block",
  attrs: { level: { default: 1 } }, // 1 | 2 | 3
  parseDOM: [
    { tag: "h1.heading-block", attrs: { level: 1 } },
    { tag: "h2.heading-block", attrs: { level: 2 } },
    { tag: "h3.heading-block", attrs: { level: 3 } },
  ],
  toDOM(node) {
    const tag = `h${node.attrs.level}` as "h1" | "h2" | "h3";
    return [tag, { class: `heading-block h${node.attrs.level}` }, 0];
  },
};

const listItem: NodeSpec = {
  content: "paragraph block*",
  parseDOM: [{ tag: "li.list-item" }],
  toDOM() { return ["li", { class: "list-item" }, 0]; },
};

const todoBlock: NodeSpec = {
  content: "inline*",
  group: "block",
  attrs: { checked: { default: false } },
  parseDOM: [{ tag: "div.todo-block" }],
  toDOM(node) {
    const cls = node.attrs.checked ? "todo-block completed" : "todo-block";
    return ["div", { class: cls },
      ["input", { type: "checkbox", ...(node.attrs.checked ? { checked: "" } : {}) }],
      ["span", 0],
    ];
  },
};

// ── Mark specs for inline nomtu highlighting ─────────────

const nomtuMatch: MarkSpec = {
  attrs: {
    word: { default: "" },
    kind: { default: "" },
    confidence: { default: 0 },
  },
  parseDOM: [{ tag: "span.nomtu-match" }],
  toDOM(mark) {
    const cls = mark.attrs.confidence > 0.8 ? "nomtu-match high" :
                mark.attrs.confidence > 0.5 ? "nomtu-match medium" : "nomtu-match low";
    return ["span", {
      class: cls,
      title: `${mark.attrs.word} (${mark.attrs.kind}) — ${(mark.attrs.confidence * 100).toFixed(0)}%`,
    }, 0];
  },
};

const nomxKeyword: MarkSpec = {
  parseDOM: [{ tag: "span.nomx-keyword" }],
  toDOM() { return ["span", { class: "nomx-keyword" }, 0]; },
};

// ── Schema assembly ──────────────────────────────────────

const baseNodes = {
  doc: { content: "block+" } as NodeSpec,
  text: { group: "inline" } as NodeSpec,
  paragraph: {
    content: "inline*",
    group: "block",
    parseDOM: [{ tag: "p" }],
    toDOM() { return ["p", 0] as const; },
  } as NodeSpec,
  prose_block: proseBlock,
  nomx_block: nomxBlock,
  media_node_block: mediaNodeBlock,
  app_preview_block: appPreviewBlock,
  drawing_block: drawingBlock,
  code_block: codeBlock,
  terminal_block: terminalBlock,
  graph_block: graphBlock,
  table_block: tableBlock,
  divider: divider,
  callout_block: calloutBlock,
  embed_block: embedBlock,
  heading: heading,
  list_item: listItem,
  todo_block: todoBlock,
};

const marks = {
  nomtu_match: nomtuMatch,
  nomx_keyword: nomxKeyword,
  // Basic marks
  em: {
    parseDOM: [{ tag: "em" }, { tag: "i" }],
    toDOM() { return ["em", 0]; },
  } as MarkSpec,
  strong: {
    parseDOM: [{ tag: "strong" }, { tag: "b" }],
    toDOM() { return ["strong", 0]; },
  } as MarkSpec,
  code: {
    parseDOM: [{ tag: "code" }],
    toDOM() { return ["code", 0]; },
  } as MarkSpec,
};

export const nomCanvasSchema = new Schema({
  nodes: addListNodes(baseNodes as any, "paragraph block*", "block"),
  marks,
});

export type BlockType =
  | "prose_block"
  | "nomx_block"
  | "media_node_block"
  | "app_preview_block"
  | "drawing_block"
  | "code_block"
  | "terminal_block"
  | "graph_block"
  | "table_block"
  | "divider"
  | "callout_block"
  | "embed_block"
  | "heading"
  | "list_item"
  | "todo_block";
