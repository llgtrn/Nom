export interface ComponentProperty {
    name: string;
    type: "text" | "number" | "boolean" | "select" | "color" | "code";
    defaultValue: unknown;
    options?: string[]; // for select type
    required: boolean;
    description: string;
}

export interface ComponentDefinition {
    type: string;
    label: string;
    category: string;
    icon: string; // text abbreviation
    properties: ComponentProperty[];
    defaultWidth: number;
    defaultHeight: number;
}

const registry = new Map<string, ComponentDefinition>();

export function registerComponent(def: ComponentDefinition): void {
    registry.set(def.type, def);
}

export function getComponent(type: string): ComponentDefinition | undefined {
    return registry.get(type);
}

export function getAllComponents(): ComponentDefinition[] {
    return Array.from(registry.values());
}

export function getComponentsByCategory(category: string): ComponentDefinition[] {
    return getAllComponents().filter(c => c.category === category);
}

export function getCategories(): string[] {
    return [...new Set(getAllComponents().map(c => c.category))];
}

// Built-in NomCanvas components
registerComponent({
    type: "nomx-editor", label: "NomX Editor", category: "Input",
    icon: "NX", defaultWidth: 600, defaultHeight: 250,
    properties: [
        { name: "source", type: "code", defaultValue: "", required: false, description: "Initial .nomx source" },
        { name: "autoCompile", type: "boolean", defaultValue: true, required: false, description: "Auto-compile on change" },
    ],
});

registerComponent({
    type: "preview", label: "Preview Panel", category: "Output",
    icon: "PV", defaultWidth: 400, defaultHeight: 300,
    properties: [
        { name: "mode", type: "select", defaultValue: "json", options: ["json", "entities", "plan", "quality", "security", "dream"], required: false, description: "Preview mode" },
    ],
});

registerComponent({
    type: "quality-badge", label: "Quality Badge", category: "Output",
    icon: "Q", defaultWidth: 300, defaultHeight: 60,
    properties: [
        { name: "source", type: "code", defaultValue: "", required: true, description: "Source to score" },
    ],
});

registerComponent({
    type: "graph-view", label: "Graph View", category: "Visualization",
    icon: "GV", defaultWidth: 600, defaultHeight: 400,
    properties: [
        { name: "graphJson", type: "code", defaultValue: "", required: false, description: "WorkflowGraph JSON" },
    ],
});

registerComponent({
    type: "terminal", label: "Terminal Output", category: "Output",
    icon: ">_", defaultWidth: 500, defaultHeight: 200,
    properties: [
        { name: "title", type: "text", defaultValue: "Terminal", required: false, description: "Terminal title" },
    ],
});

registerComponent({
    type: "media-viewer", label: "Media Viewer", category: "Output",
    icon: "MV", defaultWidth: 400, defaultHeight: 300,
    properties: [
        { name: "hash", type: "text", defaultValue: "", required: true, description: "Media entity hash" },
        { name: "mime", type: "text", defaultValue: "", required: false, description: "MIME type" },
    ],
});

registerComponent({
    type: "data-table", label: "Data Table", category: "Data",
    icon: "DT", defaultWidth: 600, defaultHeight: 300,
    properties: [
        { name: "query", type: "text", defaultValue: "", required: false, description: "Search query" },
        { name: "limit", type: "number", defaultValue: 20, required: false, description: "Max rows" },
    ],
});

registerComponent({
    type: "dream-panel", label: "Dream Report", category: "AI",
    icon: "DR", defaultWidth: 400, defaultHeight: 350,
    properties: [
        { name: "manifest", type: "text", defaultValue: "", required: true, description: "App manifest hash" },
    ],
});
