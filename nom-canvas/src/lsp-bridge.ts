import { invoke } from "@tauri-apps/api/core";
import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";

export const lspBridgeKey = new PluginKey("lsp-bridge");

interface LspHoverResult {
  contents?: { kind: string; value: string } | string;
}

interface LspCompletionItem {
  label: string;
  kind?: number;
  detail?: string;
  insertText?: string;
}

interface LspDiagnostic {
  range: { start: { line: number; character: number }; end: { line: number; character: number } };
  severity?: number; // 1=error, 2=warning, 3=info, 4=hint
  message: string;
}

/** Send an LSP request via Tauri */
async function lspRequest(method: string, params: unknown): Promise<unknown> {
  try {
    return await invoke<unknown>("lsp_request", {
      method,
      params: params as Record<string, unknown>,
    });
  } catch {
    return { error: "LSP unavailable" };
  }
}

/** Get hover info for a word */
export async function lspHover(word: string, line: number, character: number): Promise<string | null> {
  const result = await lspRequest("textDocument/hover", {
    textDocument: { uri: "inmemory://canvas" },
    position: { line, character },
  }) as { result?: LspHoverResult };

  if (!result?.result?.contents) return null;
  const contents = result.result.contents;
  if (typeof contents === "string") return contents;
  if (contents.value) return contents.value;
  return null;
}

/** Get completions for a position */
export async function lspComplete(line: number, character: number): Promise<LspCompletionItem[]> {
  const result = await lspRequest("textDocument/completion", {
    textDocument: { uri: "inmemory://canvas" },
    position: { line, character },
  }) as { result?: { items?: LspCompletionItem[] } | LspCompletionItem[] };

  if (!result?.result) return [];
  if (Array.isArray(result.result)) return result.result;
  return result.result.items || [];
}

/** Get diagnostics for a document */
export async function lspDiagnostics(content: string): Promise<LspDiagnostic[]> {
  // Send didOpen/didChange to trigger diagnostics
  await lspRequest("textDocument/didOpen", {
    textDocument: {
      uri: "inmemory://canvas",
      languageId: "nomx",
      version: 1,
      text: content,
    },
  });

  // Diagnostics come as notifications — for now, use our compile_block
  // as the diagnostic source (already wired in transform.ts)
  return [];
}

/** Create a hover tooltip element */
function createTooltip(content: string, x: number, y: number): HTMLElement {
  // Remove existing tooltip
  document.getElementById("lsp-tooltip")?.remove();

  const tooltip = document.createElement("div");
  tooltip.id = "lsp-tooltip";
  tooltip.className = "lsp-tooltip";
  tooltip.textContent = content;
  tooltip.style.left = `${x}px`;
  tooltip.style.top = `${y}px`;
  document.body.appendChild(tooltip);

  // Auto-hide after 3s
  setTimeout(() => tooltip.remove(), 3000);
  return tooltip;
}

/** Create a completion dropdown */
function showCompletionDropdown(
  items: LspCompletionItem[],
  x: number,
  y: number,
  onSelect: (item: LspCompletionItem) => void
): HTMLElement {
  document.getElementById("lsp-completions")?.remove();

  const dropdown = document.createElement("div");
  dropdown.id = "lsp-completions";
  dropdown.className = "lsp-completions";
  dropdown.style.left = `${x}px`;
  dropdown.style.top = `${y}px`;

  const max = Math.min(items.length, 8);
  for (let i = 0; i < max; i++) {
    const item = items[i];
    const row = document.createElement("div");
    row.className = "lsp-completion-item";

    const label = document.createElement("span");
    label.className = "completion-label";
    label.textContent = item.label;
    row.appendChild(label);

    if (item.detail) {
      const detail = document.createElement("span");
      detail.className = "completion-detail";
      detail.textContent = item.detail;
      row.appendChild(detail);
    }

    row.addEventListener("click", () => {
      onSelect(item);
      dropdown.remove();
    });
    dropdown.appendChild(row);
  }

  document.body.appendChild(dropdown);

  // Auto-hide on click outside
  const hideHandler = (e: MouseEvent) => {
    if (!dropdown.contains(e.target as Node)) {
      dropdown.remove();
      document.removeEventListener("click", hideHandler);
    }
  };
  setTimeout(() => document.addEventListener("click", hideHandler), 100);

  return dropdown;
}

/** Prosemirror plugin that bridges LSP interactions */
export function createLspBridgePlugin(): Plugin {
  let hoverTimeout: ReturnType<typeof setTimeout> | null = null;

  return new Plugin({
    key: lspBridgeKey,
    props: {
      handleDOMEvents: {
        mouseover(view, event) {
          const target = event.target as HTMLElement;
          if (!target.closest(".ProseMirror")) return false;

          // Debounced hover
          if (hoverTimeout) clearTimeout(hoverTimeout);
          hoverTimeout = setTimeout(async () => {
            const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
            if (!pos) return;

            // Get the word under cursor
            const $pos = view.state.doc.resolve(pos.pos);
            const text = $pos.parent.textContent;
            const offset = pos.pos - $pos.start();
            const wordMatch = getWordAtOffset(text, offset);
            if (!wordMatch) return;

            const hoverContent = await lspHover(wordMatch, 0, offset);
            if (hoverContent) {
              createTooltip(hoverContent, event.clientX, event.clientY - 30);
            }
          }, 500);

          return false;
        },
      },
    },
  });
}

function getWordAtOffset(text: string, offset: number): string | null {
  if (offset < 0 || offset >= text.length) return null;
  let start = offset;
  let end = offset;
  while (start > 0 && /\w/.test(text[start - 1])) start--;
  while (end < text.length && /\w/.test(text[end])) end++;
  const word = text.slice(start, end);
  return word.length >= 2 ? word : null;
}

/** Hide all LSP UI elements */
export function hideLspUI(): void {
  document.getElementById("lsp-tooltip")?.remove();
  document.getElementById("lsp-completions")?.remove();
}
