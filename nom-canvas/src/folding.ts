import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";

export const foldingKey = new PluginKey("folding");

/** Foldable region detected in document */
export interface FoldRange {
  from: number;
  to: number;
  label: string;
  isFolded: boolean;
}

/** Snippet template */
export interface Snippet {
  trigger: string;
  label: string;
  template: string;
}

// Built-in .nomx snippets
const NOMX_SNIPPETS: Snippet[] = [
  {
    trigger: "fn",
    label: "Function entity",
    template: "the function $1 is given $2 of text, returns text.",
  },
  {
    trigger: "mod",
    label: "Module composition",
    template: "the module $1 is intended to $2.\n  uses the function $3.",
  },
  {
    trigger: "con",
    label: "Concept declaration",
    template: "the concept $1 is intended to $2.\n  uses the module $3.\n  exposes $4.\n  this works when $5.",
  },
  {
    trigger: "scr",
    label: "Screen entity",
    template: "the screen $1 is intended to $2.\n  uses the function $3.",
  },
  {
    trigger: "dat",
    label: "Data entity",
    template: "the data $1 is given $2 of text.\n  requires $3.",
  },
  {
    trigger: "evt",
    label: "Event entity",
    template: "the event $1 is intended to $2.\n  benefit $3.\n  hazard $4.",
  },
  {
    trigger: "req",
    label: "Requires contract",
    template: "requires $1.",
  },
  {
    trigger: "ens",
    label: "Ensures contract",
    template: "ensures $1.",
  },
  {
    trigger: "ben",
    label: "Benefit effect",
    template: "benefit $1.",
  },
  {
    trigger: "haz",
    label: "Hazard effect",
    template: "hazard $1.",
  },
];

/** Detect foldable regions in .nomx text */
function detectFoldRanges(doc: any): FoldRange[] {
  const ranges: FoldRange[] = [];
  let currentStart = -1;
  let currentLabel = "";

  doc.descendants((node: any, pos: number) => {
    if (!node.isText || !node.text) return;
    const text = node.text;
    const lower = text.toLowerCase();

    // "the <kind> <name>" starts a new foldable block
    const match = lower.match(/^the\s+(function|module|concept|screen|data|event|media|property|scenario)\s+(\w+)/);
    if (match) {
      if (currentStart >= 0) {
        ranges.push({ from: currentStart, to: pos - 1, label: currentLabel, isFolded: false });
      }
      currentStart = pos;
      currentLabel = `${match[1]} ${match[2]}`;
    }
  });

  // Close last range
  if (currentStart >= 0) {
    ranges.push({ from: currentStart, to: doc.content.size, label: currentLabel, isFolded: false });
  }

  return ranges;
}

/** Create the folding + snippets plugin */
export function createFoldingPlugin(): Plugin {
  const foldedRanges = new Set<number>(); // track folded positions by `from`

  return new Plugin({
    key: foldingKey,
    state: {
      init(_, { doc }) {
        return buildFoldDecorations(doc, foldedRanges);
      },
      apply(tr, decos) {
        const meta = tr.getMeta(foldingKey);
        if (meta?.toggleFold !== undefined) {
          if (foldedRanges.has(meta.toggleFold)) {
            foldedRanges.delete(meta.toggleFold);
          } else {
            foldedRanges.add(meta.toggleFold);
          }
          return buildFoldDecorations(tr.doc, foldedRanges);
        }
        if (tr.docChanged) {
          return buildFoldDecorations(tr.doc, foldedRanges);
        }
        return decos.map(tr.mapping, tr.doc);
      },
    },
    props: {
      decorations(state) {
        return this.getState(state);
      },
      handleKeyDown(view, event) {
        // Tab triggers snippet expansion
        if (event.key === "Tab") {
          const { from, empty } = view.state.selection;
          if (!empty) return false;

          // Get word before cursor
          const $pos = view.state.doc.resolve(from);
          const textBefore = $pos.parent.textBetween(0, $pos.parentOffset);
          const wordMatch = textBefore.match(/(\w+)$/);
          if (!wordMatch) return false;

          const trigger = wordMatch[1].toLowerCase();
          const snippet = NOMX_SNIPPETS.find(s => s.trigger === trigger);
          if (!snippet) return false;

          // Expand snippet
          const startPos = from - trigger.length;
          const expanded = snippet.template.replace(/\$\d/g, "...");
          const tr = view.state.tr.replaceWith(
            startPos, from,
            view.state.schema.text(expanded)
          );
          view.dispatch(tr);
          event.preventDefault();
          return true;
        }
        return false;
      },
    },
  });
}

function buildFoldDecorations(doc: any, foldedRanges: Set<number>): DecorationSet {
  const ranges = detectFoldRanges(doc);
  const decos: Decoration[] = [];

  for (const range of ranges) {
    const isFolded = foldedRanges.has(range.from);

    // Fold indicator widget at start of range
    decos.push(
      Decoration.widget(range.from, () => {
        const btn = document.createElement("span");
        btn.className = `fold-indicator ${isFolded ? "folded" : ""}`;
        btn.textContent = isFolded ? "+" : "-";
        btn.title = `${isFolded ? "Expand" : "Collapse"} ${range.label}`;
        return btn;
      }, { side: -1 })
    );

    // If folded, replace content with placeholder
    if (isFolded && range.to > range.from + 1) {
      decos.push(
        Decoration.node(range.from, range.to, {
          class: "fold-collapsed",
          "data-label": range.label,
        })
      );
    }
  }

  return DecorationSet.create(doc, decos);
}

/** Get all available snippets */
export function getSnippets(): Snippet[] {
  return [...NOMX_SNIPPETS];
}
