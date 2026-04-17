import { invoke } from "@tauri-apps/api/core";
import { CompileResult } from "./types";
import { EditorState } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { DOMParser } from "prosemirror-model";
import { keymap } from "prosemirror-keymap";
import { baseKeymap } from "prosemirror-commands";
import { history, undo, redo } from "prosemirror-history";
import { nomCanvasSchema } from "./schema";
import { createTransformPlugin } from "./transform";
import { createHintsPlugin } from "./hints";
import { GraphUI } from "./graph-ui";
import { ComponentLibrary } from "./library";
import { ShortcutManager } from "./shortcuts";
import { CommandPalette } from "./palette";
import { Minimap } from "./minimap";
import { StatusBar } from "./statusbar";
import { Toolbar } from "./toolbar";
import { ThemeManager } from "./theme";
import { ActionRunner } from "./actions";
import { ProjectManager } from "./project";
import { CanvasRenderer } from "./renderer";
import { ElementStore } from "./elements";
import { Viewport } from "./canvas";

// ---------- Theme (sets CSS variables first) ----------
const theme = new ThemeManager();
theme.watchSystemPreference();

// ---------- Project state ----------
const project = new ProjectManager();
project.onChange((files) => {
  const dirty = files.filter(f => f.isDirty).length;
  statusBar.update({ dirtyFiles: dirty });
});

// ---------- Canvas + GraphUI ----------
const graphUI = new GraphUI("canvas-container", "compile-status");

// Add initial block
const blockContent = graphUI.addBlock("block-1", "nomx", 60, 60);

// ---------- Canvas overlay renderer ----------
const canvasEl = document.getElementById("canvas-overlay") as HTMLCanvasElement;
const elementStore = new ElementStore();
const rendererViewport = new Viewport();
const renderer = new CanvasRenderer(canvasEl, rendererViewport);

function resizeCanvas() {
  const container = document.getElementById("canvas-container")!;
  canvasEl.width = container.clientWidth;
  canvasEl.height = container.clientHeight;
  renderer.markDirty();
}
// Note: resize listener is intentionally not removed — this is a single-page app
// with a single canvas instance that lives for the full page lifetime.
window.addEventListener("resize", resizeCanvas);
resizeCanvas();

function renderLoop() {
  if (!document.hidden) {
    renderer.renderAll(elementStore.getAll(), new Set());
  }
  requestAnimationFrame(renderLoop);
}
requestAnimationFrame(renderLoop);

// ---------- Status bar (auto-appended to #app) ----------
const statusBar = new StatusBar();
statusBar.update({ zoom: 1, blockCount: 1, compileStatus: "idle", projectName: "NomCanvas" });

// ---------- Prosemirror editor (mounted inside canvas block) ----------
const state = EditorState.create({
  doc: DOMParser.fromSchema(nomCanvasSchema).parse(document.createElement("div")),
  plugins: [
    createTransformPlugin(),
    createHintsPlugin(),
    history(),
    keymap({ "Mod-z": undo, "Mod-y": redo }),
    keymap(baseKeymap),
  ],
});

const view = new EditorView(blockContent, { state });

// ---------- Library panel ----------
const library = new ComponentLibrary("library-panel");

// ---------- Toolbar ----------
const toolbar = new Toolbar("toolbar");
toolbar.addLabel("NomCanvas");
toolbar.addSeparator();
toolbar.addGroup("create", Toolbar.defaultButtons({
  newProse: () => {
    const count = graphUI.getCanvas().getVisibleBlocks().length + 1;
    const el = graphUI.addBlock(`block-${count}`, "nomx", 60 + (count - 1) * 80, 60);
    const newState = EditorState.create({
      doc: DOMParser.fromSchema(nomCanvasSchema).parse(document.createElement("div")),
      plugins: [createTransformPlugin(), createHintsPlugin(), history(),
        keymap({ "Mod-z": undo, "Mod-y": redo }), keymap(baseKeymap)],
    });
    new EditorView(el, { state: newState });
    statusBar.update({ blockCount: count });
  },
  newNomx: () => palette.open(),
  newCode: () => palette.open(),
  newGraph: () => graphUI.executeGraph(),
  compile: () => compileCurrentBlock(),
  runGraph: () => graphUI.executeGraph(),
  score: () => runner.enqueue("score", "block-1", { source: view.state.doc.textContent }),
  scan: () => runner.enqueue("scan", "block-1", { source: view.state.doc.textContent }),
  zoomIn: () => graphUI.getCanvas().setZoom(1.5),
  zoomOut: () => graphUI.getCanvas().setZoom(0.75),
  fit: () => graphUI.getCanvas().resetView(),
  library: () => library.toggle(),
  palette: () => palette.toggle(),
  minimap: () => {
    const mw = document.getElementById("minimap");
    if (mw) mw.classList.toggle("minimap-hidden");
  },
}));

// ---------- Minimap ----------
const minimap = new Minimap("minimap");

// ---------- Action runner ----------
const runner = new ActionRunner();
runner.onAction((action) => {
  statusBar.update({
    compileStatus: action.status === "running" ? "compiling"
      : action.status === "success" ? "success"
      : action.status === "error" ? "error"
      : "idle",
  });
  if (action.status === "success" || action.status === "error") {
    previewOutput.textContent = action.output;
  }
});

// ---------- Shortcuts ----------
const shortcuts = new ShortcutManager();
shortcuts.on("toggle_library", () => library.toggle());
shortcuts.on("toggle_palette", () => palette.toggle());
shortcuts.on("compile_block", () => compileCurrentBlock());
shortcuts.on("execute_graph", () => graphUI.executeGraph());
shortcuts.on("reset_zoom", () => graphUI.getCanvas().resetView());
shortcuts.on("zoom_in", () => graphUI.getCanvas().setZoom(1.2));
shortcuts.on("zoom_out", () => graphUI.getCanvas().setZoom(0.8));
shortcuts.on("fit_view", () => graphUI.getCanvas().resetView());
shortcuts.on("save_project", () => {
  const json = graphUI.saveGraph();
  project.registerFile("canvas.json", json);
  statusBar.update({ dirtyFiles: 0 });
});
shortcuts.on("new_block", () => {
  const count = 1;
  graphUI.addBlock(`block-${Date.now()}`, "nomx", 60, 60);
  statusBar.update({ blockCount: count + 1 });
});

// ---------- Command palette ----------
const palette = new CommandPalette();
palette.registerCommands([
  { id: "compile", label: "Compile Block", category: "Execute", shortcut: "Ctrl+Enter", action: () => compileCurrentBlock() },
  { id: "execute_graph", label: "Execute Graph", category: "Execute", shortcut: "Ctrl+Shift+Enter", action: () => graphUI.executeGraph() },
  { id: "toggle_library", label: "Toggle Library Panel", category: "View", shortcut: "Ctrl+L", action: () => library.toggle() },
  { id: "reset_zoom", label: "Reset Zoom to 100%", category: "View", shortcut: "Ctrl+0", action: () => graphUI.getCanvas().resetView() },
  { id: "zoom_in", label: "Zoom In", category: "View", shortcut: "Ctrl++", action: () => graphUI.getCanvas().setZoom(1.2) },
  { id: "zoom_out", label: "Zoom Out", category: "View", shortcut: "Ctrl+-", action: () => graphUI.getCanvas().setZoom(0.8) },
  { id: "fit_view", label: "Fit All Blocks in View", category: "View", action: () => graphUI.getCanvas().resetView() },
  { id: "theme_dark", label: "Switch to Dark Theme", category: "Theme", action: () => theme.switchTo("dark") },
  { id: "theme_light", label: "Switch to Light Theme", category: "Theme", action: () => theme.switchTo("light") },
  { id: "theme_solarized", label: "Switch to Solarized Theme", category: "Theme", action: () => theme.switchTo("solarized") },
  { id: "score_block", label: "Score Current Block", category: "Quality", action: () => runner.enqueue("score", "block-1", { source: view.state.doc.textContent }) },
  { id: "scan_block", label: "Security Scan Block", category: "Quality", action: () => runner.enqueue("scan", "block-1", { source: view.state.doc.textContent }) },
  { id: "save_project", label: "Save Project", category: "File", shortcut: "Ctrl+S", action: () => {
    const json = graphUI.saveGraph();
    project.registerFile("canvas.json", json);
    statusBar.update({ dirtyFiles: 0 });
  }},
]);

// Wire toolbar compile button (existing DOM element)
const compileBtn = document.getElementById("btn-compile");
if (compileBtn) {
  compileBtn.addEventListener("click", () => compileCurrentBlock());
}

// ---------- Compile helper ----------
const previewOutput = document.getElementById("preview-output")!;

async function compileCurrentBlock(): Promise<void> {
  const content = view.state.doc.textContent;
  if (!content.trim()) {
    previewOutput.textContent = "// empty block — type some .nomx prose first";
    statusBar.update({ compileStatus: "idle" });
    return;
  }
  statusBar.update({ compileStatus: "compiling" });
  try {
    const result = await invoke<CompileResult>("compile_block", { source: content });
    if (result.success) {
      previewOutput.textContent =
        `// Compiled successfully\n// Entities: ${result.entities.join(", ")}\n` +
        JSON.stringify(result, null, 2);
      statusBar.update({ compileStatus: "success" });
    } else {
      previewOutput.textContent =
        `// Compilation failed\n${result.diagnostics.join("\n")}`;
      statusBar.update({ compileStatus: "error" });
    }
  } catch (e) {
    previewOutput.textContent = `// Error: ${e}`;
    statusBar.update({ compileStatus: "error" });
  }
}

// ---------- Cursor position tracking for status bar ----------
document.addEventListener("mousemove", (e) => {
  statusBar.update({ cursorX: e.clientX, cursorY: e.clientY });
});

// ---------- Placeholder text ----------
view.dispatch(
  view.state.tr.insertText(
    "the function greet is given a name of text, returns text."
  )
);

// ---------- Initial minimap render (empty) ----------
minimap.render(new Map(), { panX: 0, panY: 0, zoom: 1, width: window.innerWidth, height: window.innerHeight });
