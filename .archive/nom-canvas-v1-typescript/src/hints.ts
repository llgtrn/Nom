import { Plugin, PluginKey } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";

export const hintsPluginKey = new PluginKey("nom-hints");

// Known Nom type signatures for inlay hints
const TYPE_HINTS: Record<string, string> = {
  "given": "→ params",
  "returns": "→ type",
  "requires": "→ precondition",
  "ensures": "→ postcondition",
  "benefit": "→ positive effect",
  "hazard": "→ negative effect",
  "favor": "→ quality preference",
  "uses": "→ dependency",
  "composes": "→ composition",
};

// Bracket pairs for colorization
const BRACKET_PAIRS: [string, string][] = [
  ["(", ")"],
  ["[", "]"],
  ["{", "}"],
];

const BRACKET_COLORS = [
  "#FFD700", // gold
  "#DA70D6", // orchid
  "#00CED1", // dark turquoise
  "#FF6347", // tomato
  "#32CD32", // lime green
];

interface BracketInfo {
  char: string;
  pos: number;
  depth: number;
  isOpen: boolean;
}

function findBrackets(text: string, offset: number): BracketInfo[] {
  const brackets: BracketInfo[] = [];
  const depthStack: number[] = [];
  let depth = 0;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    for (const [open, close] of BRACKET_PAIRS) {
      if (ch === open) {
        brackets.push({ char: ch, pos: offset + i, depth, isOpen: true });
        depthStack.push(depth);
        depth++;
      } else if (ch === close) {
        depth = Math.max(0, depth - 1);
        brackets.push({ char: ch, pos: offset + i, depth, isOpen: false });
      }
    }
  }
  return brackets;
}

function buildHintDecorations(doc: any): Decoration[] {
  const decorations: Decoration[] = [];

  doc.descendants((node: any, pos: number) => {
    if (!node.isText || !node.text) return;
    const text = node.text;

    // Inlay hints: after keywords, show type hint as widget
    for (const [keyword, hint] of Object.entries(TYPE_HINTS)) {
      const regex = new RegExp(`\\b${keyword}\\b`, "gi");
      let match;
      while ((match = regex.exec(text)) !== null) {
        const end = pos + match.index + match[0].length;
        decorations.push(
          Decoration.widget(end, () => {
            const span = document.createElement("span");
            span.className = "inlay-hint";
            span.textContent = ` ${hint}`;
            return span;
          }, { side: 1 })
        );
      }
    }

    // Bracket colorization
    const brackets = findBrackets(text, pos);
    for (const bracket of brackets) {
      const colorIdx = bracket.depth % BRACKET_COLORS.length;
      decorations.push(
        Decoration.inline(bracket.pos, bracket.pos + 1, {
          class: "bracket-colored",
          style: `color: ${BRACKET_COLORS[colorIdx]}; font-weight: 600;`,
        })
      );
    }
  });

  return decorations;
}

export function createHintsPlugin(): Plugin {
  return new Plugin({
    key: hintsPluginKey,
    state: {
      init(_, { doc }) {
        return DecorationSet.create(doc, buildHintDecorations(doc));
      },
      apply(tr, decorations) {
        if (tr.docChanged) {
          return DecorationSet.create(tr.doc, buildHintDecorations(tr.doc));
        }
        return decorations.map(tr.mapping, tr.doc);
      },
    },
    props: {
      decorations(state) {
        return this.getState(state);
      },
    },
  });
}
