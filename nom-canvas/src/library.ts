import { invoke } from "@tauri-apps/api/core";

export interface LibraryEntry {
  word: string;
  kind: string;
  score: number;
  snippet?: string;
}

export class ComponentLibrary {
  private panel: HTMLElement;
  private searchInput: HTMLInputElement;
  private resultsList: HTMLElement;
  private entries: LibraryEntry[] = [];
  private onDragStart: ((entry: LibraryEntry) => void) | null = null;

  constructor(containerId: string) {
    const container = document.getElementById(containerId);
    if (!container) {
      // Create the panel if it doesn't exist
      this.panel = document.createElement("aside");
      this.panel.id = containerId;
      this.panel.className = "component-library";
      document.getElementById("app")?.appendChild(this.panel);
    } else {
      this.panel = container;
      this.panel.className = "component-library";
    }

    // Header
    const header = document.createElement("div");
    header.className = "library-header";
    const heading = document.createElement("h3");
    heading.textContent = "Components";
    header.appendChild(heading);
    this.panel.appendChild(header);

    // Search input
    this.searchInput = document.createElement("input");
    this.searchInput.type = "text";
    this.searchInput.placeholder = "Search nomtu...";
    this.searchInput.className = "library-search";
    this.panel.appendChild(this.searchInput);

    // Results list
    this.resultsList = document.createElement("div");
    this.resultsList.className = "library-results";
    this.panel.appendChild(this.resultsList);

    // Debounced search
    let timeout: ReturnType<typeof setTimeout> | null = null;
    this.searchInput.addEventListener("input", () => {
      if (timeout) clearTimeout(timeout);
      timeout = setTimeout(() => this.search(this.searchInput.value), 300);
    });

    // Load initial entries
    this.search("");
  }

  /** Register a callback for when an entry starts being dragged */
  onEntryDragStart(callback: (entry: LibraryEntry) => void): void {
    this.onDragStart = callback;
  }

  /** Search the dictionary */
  async search(query: string): Promise<void> {
    try {
      if (query.trim()) {
        this.entries = await invoke<LibraryEntry[]>("search_dict", { query });
      } else {
        // Show recent/popular entries when no query
        this.entries = await invoke<LibraryEntry[]>("search_dict", { query: "function" });
      }
    } catch {
      this.entries = [];
    }
    this.render();
  }

  private render(): void {
    // Clear with replaceChildren (safe DOM)
    this.resultsList.replaceChildren();

    if (this.entries.length === 0) {
      const empty = document.createElement("div");
      empty.className = "library-empty";
      empty.textContent = "No entries found";
      this.resultsList.appendChild(empty);
      return;
    }

    for (const entry of this.entries) {
      const item = document.createElement("div");
      item.className = "library-item";
      item.draggable = true;
      item.dataset.word = entry.word;
      item.dataset.kind = entry.kind;

      const kindBadge = document.createElement("span");
      kindBadge.className = `library-kind kind-${entry.kind}`;
      kindBadge.textContent = entry.kind.slice(0, 3).toUpperCase();

      const wordLabel = document.createElement("span");
      wordLabel.className = "library-word";
      wordLabel.textContent = entry.word;

      item.appendChild(kindBadge);
      item.appendChild(wordLabel);

      // Drag handling
      item.addEventListener("dragstart", (e) => {
        e.dataTransfer?.setData("text/plain", JSON.stringify(entry));
        item.classList.add("dragging");
        if (this.onDragStart) this.onDragStart(entry);
      });
      item.addEventListener("dragend", () => {
        item.classList.remove("dragging");
      });

      this.resultsList.appendChild(item);
    }
  }

  /** Show/hide the panel */
  toggle(): void {
    this.panel.classList.toggle("library-hidden");
  }

  /** Get current entries */
  getEntries(): LibraryEntry[] {
    return this.entries;
  }
}
