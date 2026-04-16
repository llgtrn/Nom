import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";
import { invoke } from "@tauri-apps/api/core";

export const transformPluginKey = new PluginKey("nomx-transform");

// Nom keywords that get highlighted in prose
const NOM_KEYWORDS = new Set([
  "the", "function", "module", "concept", "screen", "data", "event",
  "media", "property", "scenario", "is", "given", "returns", "requires",
  "ensures", "benefit", "hazard", "uses", "composes", "intended",
  "favor", "matching", "with", "at-least", "confidence",
]);

interface TransformState {
  decorations: DecorationSet;
  diagnosticDecos: DecorationSet;
  pendingCompile: ReturnType<typeof setTimeout> | null;
}

/**
 * Find keyword spans in text content.
 * Returns array of {from, to, word} for each keyword match.
 */
function findKeywords(text: string, offset: number): Array<{ from: number; to: number; word: string }> {
  const results: Array<{ from: number; to: number; word: string }> = [];
  const regex = /\b([a-z][\w-]*)\b/gi;
  let match;
  while ((match = regex.exec(text)) !== null) {
    const word = match[1].toLowerCase();
    if (NOM_KEYWORDS.has(word)) {
      results.push({
        from: offset + match.index,
        to: offset + match.index + match[1].length,
        word,
      });
    }
  }
  return results;
}

/**
 * Build keyword decorations for the entire document.
 */
function buildKeywordDecorations(doc: any): Decoration[] {
  const decorations: Decoration[] = [];
  doc.descendants((node: any, pos: number) => {
    if (node.isText) {
      const keywords = findKeywords(node.text || "", pos);
      for (const kw of keywords) {
        decorations.push(
          Decoration.inline(kw.from, kw.to, {
            class: "nomx-keyword",
            title: `Nom keyword: ${kw.word}`,
          })
        );
      }
    }
  });
  return decorations;
}

/**
 * Check if a block looks like it could be .nomx source.
 * A block is nomx-like if it starts with "the <kind>" pattern.
 */
function looksLikeNomx(text: string): boolean {
  const trimmed = text.trim().toLowerCase();
  return /^the\s+(function|module|concept|screen|data|event|media|property|scenario)\s+/.test(trimmed);
}

/**
 * Create the inline transformation plugin.
 *
 * Features:
 * - Highlights Nom keywords in real time
 * - Debounced compilation check (500ms after last keystroke)
 * - Visual feedback when a block compiles successfully
 */
export function createTransformPlugin(): Plugin {
  return new Plugin({
    key: transformPluginKey,

    state: {
      init(_, state): TransformState {
        const decos = buildKeywordDecorations(state.doc);
        return {
          decorations: DecorationSet.create(state.doc, decos),
          diagnosticDecos: DecorationSet.empty,
          pendingCompile: null,
        };
      },
      apply(tr, pluginState, _oldState, newState): TransformState {
        // Handle diagnostic meta messages dispatched from the view lifecycle
        const meta = tr.getMeta(transformPluginKey);
        if (meta?.clearDiagnostics) {
          const base = tr.docChanged
            ? DecorationSet.create(newState.doc, buildKeywordDecorations(newState.doc))
            : pluginState.decorations.map(tr.mapping, tr.doc);
          return { ...pluginState, decorations: base, diagnosticDecos: DecorationSet.empty };
        }
        if (meta?.diagnostics) {
          const diagDecos: Decoration[] = [];
          tr.doc.descendants((node: any, pos: number) => {
            if (node.isText) {
              diagDecos.push(
                Decoration.inline(pos, pos + node.nodeSize, {
                  class: "nom-diagnostic-error",
                  title: (meta.diagnostics as string[]).join("; "),
                })
              );
            }
          });
          const base = tr.docChanged
            ? DecorationSet.create(newState.doc, buildKeywordDecorations(newState.doc))
            : pluginState.decorations.map(tr.mapping, tr.doc);
          return {
            ...pluginState,
            decorations: base,
            diagnosticDecos: DecorationSet.create(tr.doc, diagDecos),
          };
        }

        if (!tr.docChanged) {
          return {
            ...pluginState,
            decorations: pluginState.decorations.map(tr.mapping, tr.doc),
            diagnosticDecos: pluginState.diagnosticDecos.map(tr.mapping, tr.doc),
          };
        }
        // Rebuild keyword decorations on every change; clear stale diagnostic underlines
        const decos = buildKeywordDecorations(newState.doc);
        return {
          decorations: DecorationSet.create(newState.doc, decos),
          diagnosticDecos: DecorationSet.empty,
          pendingCompile: pluginState.pendingCompile,
        };
      },
    },

    props: {
      decorations(state) {
        const pluginState = this.getState(state);
        if (!pluginState) return DecorationSet.empty;
        // Merge keyword decorations and diagnostic decorations into one set
        return pluginState.decorations.add(
          state.doc,
          pluginState.diagnosticDecos.find()
        );
      },
    },

    view(editorView) {
      let compileTimeout: ReturnType<typeof setTimeout> | null = null;
      const statusEl = document.getElementById("compile-status");

      return {
        update(view, prevState) {
          if (!view.state.doc.eq(prevState.doc)) {
            // Debounce: compile 500ms after last keystroke
            if (compileTimeout) clearTimeout(compileTimeout);
            compileTimeout = setTimeout(async () => {
              const text = view.state.doc.textContent;
              if (!looksLikeNomx(text)) {
                if (statusEl) statusEl.textContent = "// prose mode";
                return;
              }
              if (statusEl) statusEl.textContent = "// compiling...";
              try {
                const result = await invoke<{
                  success: boolean;
                  diagnostics: string[];
                  entities: string[];
                }>("compile_block", { source: text });

                if (result.success) {
                  if (statusEl) {
                    statusEl.textContent =
                      `// compiled: ${result.entities.join(", ")}`;
                  }
                  view.dispatch(
                    view.state.tr.setMeta(transformPluginKey, { clearDiagnostics: true })
                  );
                } else {
                  if (statusEl) {
                    statusEl.textContent =
                      `// error: ${result.diagnostics[0] || "unknown"}`;
                  }
                  view.dispatch(
                    view.state.tr.setMeta(transformPluginKey, { diagnostics: result.diagnostics })
                  );
                }
              } catch (e) {
                if (statusEl) statusEl.textContent = `// invoke error: ${e}`;
              }
            }, 500);
          }
        },
        destroy() {
          if (compileTimeout) clearTimeout(compileTimeout);
        },
      };
    },
  });
}
