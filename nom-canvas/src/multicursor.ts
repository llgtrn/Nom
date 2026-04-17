import { Plugin, PluginKey, TextSelection, Selection } from "prosemirror-state";
import { Decoration, DecorationSet } from "prosemirror-view";

export const multicursorKey = new PluginKey("multicursor");

interface CursorPosition {
  pos: number;
  head: number;
  anchor: number;
}

interface MultiCursorState {
  cursors: CursorPosition[];
  decorations: DecorationSet;
}

function buildCursorDecorations(doc: any, cursors: CursorPosition[]): DecorationSet {
  const decos: Decoration[] = [];
  for (const cursor of cursors) {
    // Cursor line decoration
    decos.push(
      Decoration.widget(cursor.pos, () => {
        const el = document.createElement("span");
        el.className = "extra-cursor";
        return el;
      }, { side: 0 })
    );
    // Selection highlight if anchor != head
    if (cursor.anchor !== cursor.head) {
      const from = Math.min(cursor.anchor, cursor.head);
      const to = Math.max(cursor.anchor, cursor.head);
      decos.push(Decoration.inline(from, to, { class: "extra-selection" }));
    }
  }
  return DecorationSet.create(doc, decos);
}

export function createMultiCursorPlugin(): Plugin {
  return new Plugin({
    key: multicursorKey,
    state: {
      init(): MultiCursorState {
        return { cursors: [], decorations: DecorationSet.empty };
      },
      apply(tr, state): MultiCursorState {
        const meta = tr.getMeta(multicursorKey);
        if (meta?.addCursor) {
          const newCursors = [...state.cursors, meta.addCursor];
          return {
            cursors: newCursors,
            decorations: buildCursorDecorations(tr.doc, newCursors),
          };
        }
        if (meta?.clearCursors) {
          return { cursors: [], decorations: DecorationSet.empty };
        }
        if (meta?.selectNext) {
          // Add cursor at next occurrence of current selection
          return state; // placeholder — needs word search
        }
        if (tr.docChanged) {
          // Map existing cursors through the transaction
          const mapped = state.cursors.map(c => ({
            pos: tr.mapping.map(c.pos),
            head: tr.mapping.map(c.head),
            anchor: tr.mapping.map(c.anchor),
          }));
          return {
            cursors: mapped,
            decorations: buildCursorDecorations(tr.doc, mapped),
          };
        }
        return {
          ...state,
          decorations: state.decorations.map(tr.mapping, tr.doc),
        };
      },
    },
    props: {
      decorations(state) {
        return this.getState(state)?.decorations ?? DecorationSet.empty;
      },
      handleClick(view, pos, event) {
        // Ctrl+Click adds a cursor
        if (event.ctrlKey || event.metaKey) {
          const cursor: CursorPosition = { pos, head: pos, anchor: pos };
          view.dispatch(view.state.tr.setMeta(multicursorKey, { addCursor: cursor }));
          return true;
        }
        // Regular click clears extra cursors
        const state = multicursorKey.getState(view.state);
        if (state && state.cursors.length > 0) {
          view.dispatch(view.state.tr.setMeta(multicursorKey, { clearCursors: true }));
        }
        return false;
      },
    },
  });
}

/** Get current extra cursor count */
export function getCursorCount(editorState: any): number {
  const state = multicursorKey.getState(editorState);
  return state ? state.cursors.length + 1 : 1; // +1 for primary cursor
}
