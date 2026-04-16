import { invoke } from "@tauri-apps/api/core";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { Schema, DOMParser } from "prosemirror-model";
import { schema as basicSchema } from "prosemirror-schema-basic";
import { addListNodes } from "prosemirror-schema-list";
import { keymap } from "prosemirror-keymap";
import { baseKeymap } from "prosemirror-commands";
import { history, undo, redo } from "prosemirror-history";

// Schema — basic prose + code blocks
const schema = new Schema({
  nodes: addListNodes(basicSchema.spec.nodes, "paragraph block*", "block"),
  marks: basicSchema.spec.marks,
});

// Editor state
const state = EditorState.create({
  doc: DOMParser.fromSchema(schema).parse(
    document.createElement("div")
  ),
  plugins: [
    history(),
    keymap({ "Mod-z": undo, "Mod-y": redo }),
    keymap(baseKeymap),
  ],
});

// Mount editor
const editorEl = document.getElementById("editor")!;
const view = new EditorView(editorEl, { state });

// Compile button handler
const compileBtn = document.getElementById("btn-compile")!;
const previewOutput = document.getElementById("preview-output")!;

interface CompileResult {
  success: boolean;
  diagnostics: string[];
  entities: string[];
}

compileBtn.addEventListener("click", async () => {
  const content = view.state.doc.textContent;
  if (!content.trim()) {
    previewOutput.textContent = "// empty block — type some .nomx prose first";
    return;
  }
  try {
    const result = await invoke<CompileResult>("compile_block", { source: content });
    if (result.success) {
      previewOutput.textContent =
        `// Compiled successfully\n// Entities: ${result.entities.join(", ")}\n` +
        JSON.stringify(result, null, 2);
    } else {
      previewOutput.textContent =
        `// Compilation failed\n${result.diagnostics.join("\n")}`;
    }
  } catch (e) {
    previewOutput.textContent = `// Error: ${e}`;
  }
});

// Placeholder text
view.dispatch(
  view.state.tr.insertText(
    "the function greet is given a name of text, returns text."
  )
);
