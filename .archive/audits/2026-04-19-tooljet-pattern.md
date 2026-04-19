# ToolJet Widget Registry Pattern Audit

**Date:** 2026-04-19  
**Analyst:** Pattern-extraction subagent  
**Source repo:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\ToolJet-develop\` (branch `develop`)  
**Nom target:** `nom-blocks` / `nom-panels` widget registry (51 kinds seeded, 55 target)  
**Scope:** Frontend widget registry, dependency graph, runtime component system, 55-widget discovery/definition/registration.

---

## 1. Pattern Summary

### 1.1 Registry Architecture — Flat Config ? Component Map ? Palette

ToolJet uses a **three-tier registration pipeline**:

1. **Config objects** — One `.js` file per widget (e.g. `widgets/button.js`) exporting a plain object that declares the widget schema.
2. **Config aggregation** — `widgetConfig.js` imports every config and exports a flat `widgets[]` array. This is the single source of truth for "what exists in the registry."
3. **Component type map** — `componentTypes.js` merges each config with `universalProps` (tooltip, box-shadow, etc.) and exports:
   - `componentTypes` — array of fully-hydrated widget schemas
   - `componentTypeDefinitionMap` — `Map<componentName, schema>` for O(1) lookup
4. **Runtime component map** — `editorHelpers.js` imports the actual React component classes and builds `AllComponents = { Button, Table, … }`. `getComponentToRender(name)` does a simple keyed lookup.

The palette UI (`ComponentsManagerTab.jsx`) consumes `componentTypes`, filters out `IGNORED_ITEMS` (`ModuleContainer`, `ModuleViewer`), and groups the rest into accordion sections via `sectionConfig.js`.

### 1.2 Widget Config Schema (per-widget contract)

Every widget config object follows this shape (cited from `button.js`, `table.js`, `container.js`):

| Key | Purpose | Example values |
|-----|---------|----------------|
| `name` / `displayName` | Human label | `"Button"`, `"Table"` |
| `component` | Runtime lookup key into `AllComponents` | `"Button"`, `"Table"` |
| `defaultSize` | Grid units (width/height) on drop | `{ width: 4, height: 40 }` |
| `properties` | Data-bound props exposed in the inspector | `{ text: { type: "code", displayName: "Label", validation: { schema: { type: "string" } } } }` |
| `styles` | Visual props (colors, radius, shadow) | `{ backgroundColor: { type: "colorSwatches", displayName: "Background", accordian: "button" } }` |
| `events` | Named event hooks the user can wire | `{ onClick: { displayName: "On click" } }` |
| `exposedVariables` | Default values for runtime-exposed state | `{ buttonText: "Button", isVisible: true }` |
| `actions` | Programmatic handles other widgets can call | `{ handle: "setText", displayName: "Set text", params: [...] }` |
| `definition` | Default serialized state for new instances | `{ properties: { text: { value: "Button" } }, styles: { ... }, events: [] }` |
| `others` | Platform toggles (desktop/mobile) | `{ showOnDesktop: { type: "toggle" } }` |
| `defaultChildren` | (Container widgets only) Auto-inserted child slots | Container defines a header `Text` child |

**Key insight:** ToolJet does not use TypeScript interfaces or Rust enums for the schema. It is a pure JavaScript-object convention enforced by convention and by the inspector UI. The `validation.schema` sub-objects loosely describe types (`string`, `boolean`, `union`, `array`) but are consumed only by the inspector and the runtime resolver, not by a compiler.

### 1.3 Runtime Component System — RenderWidget ? Zustand ? Component

The runtime path from canvas pixel to React component:

```
WidgetWrapper.jsx (layout/selection wrapper)
  +-> RenderWidget.jsx (property resolver + error boundary)
        +- reads: resolvedProperties, resolvedStyles, resolvedGeneralProperties
        +- reads: fireEvent, setExposedValue, validateWidget
        +-> getComponentToRender(componentType)  // from AllComponents map
              +-> <Button ...props />
```

**RenderWidget.jsx** (`frontend/src/AppBuilder/AppCanvas/RenderWidget.jsx`) is the critical runtime glue. It:
- Subscribes to the Zustand store (`useStore`) for resolved properties/styles
- Injects a `setExposedVariable(key, value)` callback that writes back into `resolvedStore.modules[moduleId].exposedValues.components[id]` and triggers dependency updates
- Injects a `fireEvent(eventName)` callback that routes to `eventsSlice.fireEvent`, which looks up user-defined event handlers for that component
- Wraps everything in `ErrorBoundary` and `OverlayTrigger` (tooltips)

### 1.4 Dependency Graph — How Widget Props/Outputs Connect

ToolJet implements a **reactive dependency graph** for the `{{ }}` template syntax.

**Graph construction (`DependencyClass.js`)**
- Uses the npm package `dependency-graph` (`DepGraph`)
- Nodes are path strings like `components.button1.text`, `queries.getUsers.data`, `variables.foo`
- Edges represent "A depends on B" — e.g. `components.text1.text` depends on `queries.getUsers.data`
- `addDependency(fromPath, toPath, nodeData)` is called whenever a component property contains `{{...}}`

**Graph update flow (`componentsSlice.js` / `resolvedSlice.js`)**
1. `extractAndReplaceReferencesFromString(value, componentNameIdMapping, queryNameIdMapping)` parses `{{...}}` and converts human names to internal IDs
2. `resolveDynamicValues(valueWithBrackets, exposedValues, customResolvables, ...)` evaluates the JS-like expression against the current state
3. `setResolvedComponentByProperty(...)` writes the resolved value into `resolvedStore.modules[moduleId].components[componentId].properties[key]`
4. `updateDependencyValues(changedPath, moduleId)` walks the graph to find all direct/indirect dependents and re-resolves them

**Graph path conventions**
- LHS (source) split into 3 parts: `queries.queryID.data`
- RHS (consumer) split into 2 parts: `components.componentID.properties.text`
- Variables: `variables.key` (special-cased in `DependencyClass.js`)
- Custom resolvables: scoped to Listview/Kanban sub-container indices (`components.componentID[0].properties.text`)

### 1.5 Widget-to-Widget Communication

ToolJet widgets communicate through **three channels**:

1. **Reactive property binding** — Widget A sets `{{components.B.text}}` in one of its properties. When B’s exposed `text` changes, the dependency graph re-resolves A’s property and re-renders A.
2. **Event + Action system** — Widget A fires `onClick` ? `eventsSlice.fireEvent` finds the matching event definition ? executes an action such as `control-component` (calling an action handle on Widget B) or `set-custom-variable` (updating state that other widgets reference).
3. **Exposed variables** — Each widget instance exposes a flat object of variables (e.g. `buttonText`, `isVisible`, `isLoading`). These are readable by other widgets via `{{components.button1.buttonText}}` and writable via the widget’s own `actions` (e.g. `setText`).

**Notable:** There is no direct "prop callback" between siblings. All inter-widget communication is mediated by the Zustand store + dependency graph.

### 1.6 Container / Nesting Model

Container widgets (`Container`, `Form`, `Listview`, `Tabs`, `Kanban`, `ModalV2`) own **sub-canvases**:

- `Container.jsx` (`AppCanvas/Container.jsx`) renders a `real-canvas` div that acts as a drop target (react-dnd)
- `containerChildrenMapping` in Zustand state maps `parentId -> [childId, ...]`
- `WidgetWrapper` receives `subContainerIndex` for Listview/Kanban item instances; resolved values are stored as arrays per component ID (`components[componentId][index]`)
- Restricted nesting is enforced by `RESTRICTED_WIDGETS_CONFIG` (e.g. `Form` cannot contain `Calendar`, `Kanban`, `Form`, `Tabs`, `Modal`, `Listview`, `Container`)

### 1.7 Widget Inventory Count

In the **AppBuilder** (new editor) registry (`AppBuilder/WidgetManager/configs/widgetConfig.js`):

- **Total config imports:** 72 unique widget config files imported in `widgets/index.js`
- **Active palette entries:** ~55 (after excluding deprecated/legacy and internal-only modules)
- **Deprecated/Legacy:** 7 — `Modal`, `Datepicker`, `RadioButton`, `ToggleSwitch`, `DropDown`, `Multiselect`, `RangeSlider` (marked `//!Depreciated` in comments and grouped under "Legacy" in the palette)
- **Internal-only:** 2 — `ModuleContainer`, `ModuleViewer` (ignored in palette via `IGNORED_ITEMS`)
- **KanbanBoard** is also marked deprecated (`//!Depreciated`) in the widget index

The palette sections in `sectionConfig.js` currently expose:
- Commonly used (6), Buttons (3), Data (2), Layouts (7), Text inputs (5), Number inputs (5), Select inputs (7), Date/time inputs (4), Navigation (3), Media (7), Presentation (9), Custom (3), Miscellaneous (6), Legacy (7) = **74 items across sections**, but many are overlapping counts and legacy widgets are still present.

The **old Editor** (`frontend/src/Editor/WidgetManager/`) has a parallel registry with fewer widgets (~45), but the AppBuilder registry is the canonical one for new development.

---

## 2. Key Source Files

### 2.1 Registry & Schema Definition

| File | Lines | Role |
|------|-------|------|
| `frontend/src/AppBuilder/WidgetManager/configs/widgetConfig.js` | 172 | Aggregates all widget configs into `widgets[]` array. The canonical registry list. |
| `frontend/src/AppBuilder/WidgetManager/widgets/index.js` | 145 | Barrel export of all 72 widget config objects. |
| `frontend/src/AppBuilder/WidgetManager/componentTypes.js` | 49 | Merges `universalProps` into each widget; exports `componentTypes` and `componentTypeDefinitionMap`. |
| `frontend/src/AppBuilder/WidgetManager/configs/restrictedWidgetsConfig.js` | 17 | Defines which widgets cannot nest inside which containers. |
| `frontend/src/AppBuilder/RightSideBar/ComponentManagerTab/sectionConfig.js` | 70 | Palette section grouping (Commonly used, Buttons, Data, Layouts, etc.). |
| `frontend/src/AppBuilder/RightSideBar/ComponentManagerTab/constants.js` | 11 | `LEGACY_ITEMS` and `IGNORED_ITEMS` lists. |

### 2.2 Runtime Rendering

| File | Lines | Role |
|------|-------|------|
| `frontend/src/AppBuilder/AppCanvas/RenderWidget.jsx` | 265 | Core runtime renderer. Resolves properties, binds events, injects `setExposedVariable`, selects component class. |
| `frontend/src/AppBuilder/AppCanvas/WidgetWrapper.jsx` | 187 | Layout wrapper: grid positioning, visibility, selection state, dynamic height, passes `subContainerIndex`. |
| `frontend/src/AppBuilder/AppCanvas/Container.jsx` | 247 | Sub-canvas renderer for containers. Manages `containerChildrenMapping`, react-dnd drop target, ghost widgets. |
| `frontend/src/AppBuilder/_helpers/editorHelpers.js` | 362 | `AllComponents` map, `getComponentToRender()`, `findComponentsWithReferences()`, low-priority task scheduler. |
| `frontend/src/AppBuilder/_stores/store.js` | 72 | Zustand root store composing ~20 slices (components, resolved, events, dependency, grid, etc.). |

### 2.3 State & Dependency Resolution

| File | Lines | Role |
|------|-------|------|
| `frontend/src/AppBuilder/_stores/slices/resolvedSlice.js` | 674 | Resolved value store: `components`, `exposedValues`, `queries`, `variables`, `globals`, `customResolvables`. |
| `frontend/src/AppBuilder/_stores/slices/componentsSlice.js` | 2457 | Component tree management: add/delete/move, dependency graph init, property resolution, validation. |
| `frontend/src/AppBuilder/_stores/slices/dependencySlice.js` | 72 | Thin Zustand slice around `DependencyGraph` class. |
| `frontend/src/AppBuilder/_stores/slices/DependencyClass.js` | 170 | Wrapper over `dependency-graph` npm package. Path splitting (2-part vs 3-part), orphan cleanup. |
| `frontend/src/AppBuilder/_stores/slices/eventsSlice.js` | 1357 | Event handler storage, `fireEvent`, action executor (show-alert, run-query, control-component, switch-page, etc.). |

### 2.4 Example Widget Implementations

| File | Widget | Notable patterns |
|------|--------|------------------|
| `frontend/src/AppBuilder/Widgets/Button.jsx` | `Button` | Simple props ? local state ? `setExposedVariables` for actions (`click`, `setText`, `disable`, `visibility`, `loading`). |
| `frontend/src/AppBuilder/Widgets/NewTable/Table.jsx` | `Table` | Complex widget with its own Zustand sub-store (`useTableStore`), column transformations, dynamic height. |
| `frontend/src/AppBuilder/Widgets/Container/Container.jsx` | `Container` | Sub-canvas host. Uses `HorizontalSlot` for header area. Delegates children to `ContainerComponent`. |
| `frontend/src/AppBuilder/WidgetManager/widgets/button.js` | `Button` config | Schema definition: properties, styles, events, exposedVariables, actions, definition defaults. |
| `frontend/src/AppBuilder/WidgetManager/widgets/table.js` | `Table` config | Large schema (~750 lines): column definitions, server-side pagination toggles, validation rules. |
| `frontend/src/AppBuilder/WidgetManager/widgets/container.js` | `Container` config | `defaultChildren` array for auto-inserted header text widget. |

### 2.5 Palette UI

| File | Role |
|------|------|
| `frontend/src/AppBuilder/RightSideBar/ComponentManagerTab/ComponentsManagerTab.jsx` | Palette sidebar: search (Fuse.js), section accordion, tabs for Components vs Modules. |
| `frontend/src/AppBuilder/RightSideBar/ComponentManagerTab/DragLayer.jsx` | react-dnd drag source for each widget card. |
| `frontend/src/AppBuilder/RightSideBar/WidgetBox/WidgetBox.jsx` | Visual card: icon, legacy/new badge, translated display name. |

---

## 3. Nom Mapping

### 3.1 What Nom Already Has

From `nom-canvas/crates/nom-panels/src/left/widget_registry.rs`:

- **51 `WidgetKind` variants** organized into 6 categories: Basic (10), Display (9), Layout (7), Data (9), Form (7), Custom/Nom-native (9)
- **Registry struct** with `with_all()`, `by_category()`, `search()`
- **Rust enum-based** — strongly typed, no runtime schema objects

From `nom-canvas/crates/nom-blocks/src/block_model.rs`:

- **`BlockModel`** — typed block with `id`, `entity: NomtuRef`, `flavour`, `slots: Vec<(String, SlotValue)>`, `children: Vec<BlockId>`, `parent: Option<BlockId>`
- **Slot-based properties** — no fixed schema per block type; properties live in slots

From `docs/superpowers/plans/2026-04-17-phase3-blocks-panels.md`:

- Plan already references ToolJet patterns for property inspector, `RenderWidget`, and `component.definition.properties`
- Proposed `BlockProperty { key, label, value, section }` + `PropertyValue` enum directly inspired by ToolJet

### 3.2 Gap Analysis: 51 ? 55

Nom is missing **4 widget kinds** to reach 55. Comparing ToolJet’s active palette against Nom’s `WidgetKind`:

| ToolJet Active Widget | Nom Equivalent? | Gap |
|-----------------------|-----------------|-----|
| `PopoverMenu` | No | **Gap #1** — floating menu triggered by button |
| `AudioRecorder` | No | **Gap #2** — media capture widget |
| `Camera` | No | **Gap #3** — media capture widget |
| `Chat` | No | **Gap #4** — chat/message stream widget |
| `Steps` | No | Could map to `Timeline` or be distinct |
| `QRScanner` | No | Could be absorbed into `Image` or distinct |
| `BoundedBox` | No | Annotation overlay; niche |
| `PDF` | No | Document viewer; could be `Embed` block |
| `Iframe` / `Html` / `CustomComponent` | Partial | Covered by `EmbedBlock` in Phase 3 plan |
| `Calendar` | No | Could be distinct or part of `DatePicker` |
| `Kanban` | No | Could be `Table` view mode or distinct |
| `Map` | No | Geographic display; niche |
| `CodeEditor` | Partial | Covered by `NomxBlock` |

**Recommended 4 additions** (minimal, high-utility):
1. `PopoverMenu` — essential for rich UI patterns
2. `Steps` / `Stepper` — wizard/onboarding UI
3. `QRScanner` — common in mobile/low-code apps
4. `Chat` / `MessageStream` — AI-era essential

Alternatively, if Nom wants to stay closer to its canvas-first model, the 4 gaps could be:
1. `PopoverMenu`
2. `Steps`
3. `Calendar`
4. `Kanban`

### 3.3 Architectural Mapping

| ToolJet Concept | Nom Target Concept | Notes |
|-----------------|--------------------|-------|
| `widgetConfig.js` flat array | `WidgetKind::all()` + `WidgetRegistry` | Nom already uses enum; needs runtime schema description for inspector |
| `componentTypeDefinitionMap` | `WidgetRegistry` + `HashMap<WidgetKind, WidgetSchema>` | Add a schema map for inspector metadata (display name, default size, categories) |
| `AllComponents` map | `BlockRegistry` (planned in Phase 3) | Factory map `BlockKind -> Box<dyn Block>`; currently planned with 7 block kinds, not 51 widget kinds |
| `RenderWidget.jsx` | `Block::paint()` + viewport dispatch | ToolJet’s React render becomes GPUI primitive emission; property resolution happens before paint |
| `properties` / `styles` / `events` | `BlockProperty` + `PropertySection` | Phase 3 plan already defines these structs. Need to map 51 widget kinds to property schemas |
| `exposedVariables` | `BlockModel` slots + `resolvedStore.exposedValues` | Nom slots are the data model; need a runtime "exposed values" cache for reactive reads |
| `dependency-graph` npm + `DependencyClass.js` | Custom DAG in Rust | Need a Rust-native dependency graph for `{{...}}`-style reactive bindings between blocks/slots |
| `eventsSlice.fireEvent` + actions | Nom event bus (not yet built) | Need event routing: block event ? action list ? target block action handle |
| `Container` + sub-canvas | `BlockModel.children` + `ContainerBlock` | Nom already has `children: Vec<BlockId>`; need sub-canvas layout + drag-drop |
| `RESTRICTED_WIDGETS_CONFIG` | Validation in `BlockRegistry::create()` or inspector | Simple allow-list per parent block kind |
| `WidgetManager` palette | `SidebarPanel` (planned) + widget registry search | Phase 3 `sidebar.rs` is for document tree; need a **widget palette panel** distinct from sidebar |

### 3.4 Property Schema Translation

ToolJet widget configs are **~200–800 lines each** of deeply nested objects. To adopt these patterns in Nom, each `WidgetKind` would need a corresponding `WidgetSchema` that describes:

```rust
pub struct WidgetSchema {
    pub kind: WidgetKind,
    pub display_name: &'static str,
    pub default_size: (u32, u32), // grid units
    pub properties: Vec<PropertyDef>,
    pub styles: Vec<PropertyDef>,
    pub events: Vec<EventDef>,
    pub exposed_variables: Vec<(&'static str, PropertyValue)>,
    pub actions: Vec<ActionDef>,
    pub default_definition: HashMap<String, PropertyValue>,
}
```

This is **not yet present** in Nom. The Phase 3 plan sketches `BlockProperty` and `PropertyValue` but does not wire them to a per-widget schema registry.

---

## 4. Licensing / Complexity Notes

### 4.1 License

- ToolJet is licensed under **GNU Affero General Public License v3 (AGPL-3.0)**
- File: `ToolJet-develop/LICENSE` (661 lines)
- **Implication:** Any code copied or derived from ToolJet source must be released under AGPL-3.0. Nom is a separate project; **pattern inspiration is safe, copy-paste is not**.
- Recommendation: Treat this audit as **clean-room pattern documentation**. Re-implement concepts with original code and Nom-native data types.

### 4.2 Complexity Hotspots

| Area | Complexity | Risk for Nom |
|------|-----------|--------------|
| `componentsSlice.js` (2,457 lines) | **Very High** | Property resolution, dependency graph initialization, undo/redo, form field syncing, array notation parsing — all in one file. |
| `eventsSlice.js` (1,357 lines) | **High** | 20+ action types with async execution, error logging, query parameter resolution, modal control. |
| `resolvedSlice.js` (674 lines) | **Medium-High** | Multi-module resolved store, exposed values, custom resolvables, page variables, secrets. |
| `DependencyClass.js` (170 lines) | **Medium** | Thin wrapper over `dependency-graph`, but path-splitting logic (2-part vs 3-part) is subtle. |
| `RenderWidget.jsx` + `WidgetWrapper.jsx` | **Medium** | React-specific (Zustand `shallow`, `useMemo`, `memo`). Core logic is "resolve props, bind callbacks, render component." |
| Widget configs (~72 files) | **Medium** | Each is repetitive but large. Collective volume is ~15,000+ lines of schema definition. |
| `Table` widget sub-system | **Very High** | Own Zustand store, column transformations, server-side pagination, cell editing, action buttons. |

### 4.3 Tech-Stack Assumptions That Do Not Transfer Directly

- **React + Zustand + Immer** — Nom uses Rust/GPUI. The reactive update model must be rebuilt with Rust ownership patterns.
- **react-dnd** — Drag-and-drop from palette to canvas. Nom will need GPUI-native or winit-based DnD.
- **`dependency-graph` npm package** — A Rust DAG library (e.g. `petgraph`) or a custom impl is needed.
- **JavaScript `{{...}}` expression evaluation** — Nom would need a safe expression evaluator (Wasm QuickJS, Rhai, or a custom Nom expression language).
- **CSS-in-JS / Tailwind** — ToolJet styles are resolved to CSS objects. Nom uses `nom-theme` tokens + GPUI primitives.

---

## 5. Adoption Effort Estimate

### 5.1 Breakdown by Work Stream

| Work Stream | ToolJet Analog | Nom Effort | Notes |
|-------------|---------------|------------|-------|
| **A. Widget Schema DSL** | `widgets/*.js` + `componentTypes.js` | **2–3 weeks** | Define `WidgetSchema` per `WidgetKind` (51 kinds). Could be macro-generated from a compact YAML/JSON spec to avoid 15k lines of hand-written Rust. |
| **B. Dependency Graph (Rust)** | `DependencyClass.js` + `dependencySlice.js` | **1–2 weeks** | `petgraph` DAG + path parser for `components.*`, `queries.*`, `variables.*`. Needs incremental update on slot change. |
| **C. Property Resolver** | `componentsSlice.js` (resolution half) | **2–3 weeks** | Parse `{{...}}` expressions, evaluate against `resolvedStore`, write into `BlockModel` slots or a parallel resolved cache. |
| **D. Event + Action System** | `eventsSlice.js` | **2–3 weeks** | Event handler storage, action dispatcher, `control-component` action (calling action handles on other blocks), `run-query` integration. |
| **E. Widget Palette Panel** | `ComponentsManagerTab.jsx` | **1 week** | GPUI panel with search, category accordion, drag-source for canvas drop. Reuses `WidgetRegistry::search()` and `by_category()`. |
| **F. Runtime Renderer** | `RenderWidget.jsx` + `WidgetWrapper.jsx` | **2–3 weeks** | GPUI element that reads resolved slots, selects block implementation from registry, paints primitives. Must handle `subContainerIndex` for list/kanban. |
| **G. Container / Sub-canvas** | `Container.jsx` | **2 weeks** | Sub-canvas layout, child block enumeration, drag-drop target, restricted nesting validation. |
| **H. 4 New Widget Kinds** | `popoverMenu.js`, `chat.js`, etc. | **1 week** | Add enum variants + schemas + minimal paint implementations. |
| **I. Validation + Inspector** | `Inspector/` + `validateWidget` | **2 weeks** | Per-property validation (regex, min/max, mandatory), property inspector panel rendering. |

### 5.2 Total Effort

- **Minimum viable wiring (51 ? 55, basic registry + palette + resolver):** **6–8 weeks** (1 engineer)
- **Full ToolJet-equivalent runtime (dependency graph, events, actions, containers, validation):** **12–16 weeks** (1 engineer) or **6–8 weeks** (2 engineers parallel)

### 5.3 Priority Order for Nom

1. **Week 1–2:** Widget schema DSL + `WidgetRegistry` augmentation (stream A)
2. **Week 3–4:** Palette panel + drag-drop (stream E)
3. **Week 5–6:** Property resolver + dependency graph (streams B + C)
4. **Week 7–8:** Runtime renderer + container sub-canvas (streams F + G)
5. **Week 9–10:** Event/action system + validation (streams D + I)
6. **Week 11–12:** 4 new widget kinds + polish (stream H)

### 5.4 Risk Factors

- **Expression language:** If Nom wants `{{...}}` compatibility, evaluating JS-like expressions in Rust is non-trivial. A safer subset (e.g. Nom’s own template syntax) would reduce effort.
- **Table widget:** ToolJet’s Table is essentially a mini-application (~2,000+ lines). A Nom `TableBlock` will require significant investment for parity.
- **Server-side data:** ToolJet widgets like Table and Chart deeply integrate with query runners. Nom’s data layer (`nom-graph`) is not yet ready; widgets may need mock data bindings initially.

---

## 6. Appendix: Complete Widget Inventory (AppBuilder)

Active (non-legacy, non-internal) widgets from `widgetConfig.js`:

**Buttons:** Button, ButtonGroup, PopoverMenu  
**Data:** Table, Chart  
**Layouts:** Form, ModalV2, Container, Tabs, Listview, Calendar, Kanban  
**Text Inputs:** TextInput, EmailInput, PasswordInput, TextArea, RichTextEditor  
**Number Inputs:** NumberInput, PhoneInput, CurrencyInput, RangeSliderV2, StarRating  
**Select Inputs:** DropdownV2, MultiselectV2, TagsInput, Checkbox, ToggleSwitchV2, RadioButtonV2, TreeSelect  
**Date/Time Inputs:** DatePickerV2, TimePicker, DatetimePickerV2, DaterangePicker  
**Navigation:** Link, Steps, Pagination  
**Media:** Icon, Image, SvgImage, PDF, Map, AudioRecorder, Camera  
**Presentation:** Text, Tags, CircularProgressBar, Divider, VerticalDivider, Statistics, Timeline, Timer, Spinner  
**Custom:** CustomComponent, Html, IFrame  
**Miscellaneous:** FilePicker, CodeEditor, ColorPicker, BoundedBox, QrScanner, Chat  
**Legacy (deprecated):** Modal, Datepicker, RadioButton, ToggleSwitch, DropDown, Multiselect, RangeSlider  
**Internal:** ModuleContainer, ModuleViewer  

**Total unique configs:** 72  
**Palette-visible (active):** ~55  
**Nom current:** 51  
**Nom target:** 55
