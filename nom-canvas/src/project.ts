/** Project file entry */
export interface ProjectFile {
  path: string;
  name: string;
  extension: string;
  lastModified: number;
  size: number;
  type: "nomx" | "nom" | "nomtu" | "media" | "other";
  isDirty: boolean;
}

/** Project state */
export interface ProjectState {
  rootPath: string | null;
  files: Map<string, ProjectFile>;
  openFiles: string[];
  activeFile: string | null;
  lastSaved: number | null;
}

export class ProjectManager {
  private state: ProjectState = {
    rootPath: null,
    files: new Map(),
    openFiles: [],
    activeFile: null,
    lastSaved: null,
  };
  private changeListeners: ((files: ProjectFile[]) => void)[] = [];
  private watchInterval: ReturnType<typeof setInterval> | null = null;

  /** Set project root and scan for files */
  async openProject(rootPath: string): Promise<void> {
    this.state.rootPath = rootPath;
    this.state.files.clear();
    // In Tauri, we'd use fs plugin — for now, track files manually
    this.state.lastSaved = Date.now();
  }

  /** Register a file in the project */
  registerFile(path: string, content?: string): ProjectFile {
    const name = path.split(/[/\\]/).pop() || path;
    const ext = name.includes(".") ? name.split(".").pop() || "" : "";
    const type = this.classifyFile(ext);
    const file: ProjectFile = {
      path,
      name,
      extension: ext,
      lastModified: Date.now(),
      size: content?.length || 0,
      type,
      isDirty: false,
    };
    this.state.files.set(path, file);
    return file;
  }

  /** Mark a file as modified */
  markDirty(path: string): void {
    const file = this.state.files.get(path);
    if (file) {
      file.isDirty = true;
      file.lastModified = Date.now();
      this.notifyChange();
    }
  }

  /** Mark a file as saved */
  markClean(path: string): void {
    const file = this.state.files.get(path);
    if (file) {
      file.isDirty = false;
      this.state.lastSaved = Date.now();
    }
  }

  /** Open a file in the editor */
  openFile(path: string): void {
    if (!this.state.openFiles.includes(path)) {
      this.state.openFiles.push(path);
    }
    this.state.activeFile = path;
  }

  /** Close a file */
  closeFile(path: string): void {
    this.state.openFiles = this.state.openFiles.filter(f => f !== path);
    if (this.state.activeFile === path) {
      this.state.activeFile = this.state.openFiles[0] || null;
    }
  }

  /** Get all files of a type */
  getFilesByType(type: ProjectFile["type"]): ProjectFile[] {
    return Array.from(this.state.files.values()).filter(f => f.type === type);
  }

  /** Get dirty files */
  getDirtyFiles(): ProjectFile[] {
    return Array.from(this.state.files.values()).filter(f => f.isDirty);
  }

  /** Get project state */
  getState(): Readonly<ProjectState> { return this.state; }

  /** Listen for file changes */
  onChange(callback: (files: ProjectFile[]) => void): void {
    this.changeListeners.push(callback);
  }

  /** Start polling for changes (simple watcher) */
  startWatching(intervalMs = 2000): void {
    if (this.watchInterval) return;
    this.watchInterval = setInterval(() => {
      // In a real implementation, this would use Tauri's fs watcher
      // For now, just notify listeners of current state
      this.notifyChange();
    }, intervalMs);
  }

  /** Stop watching */
  stopWatching(): void {
    if (this.watchInterval) {
      clearInterval(this.watchInterval);
      this.watchInterval = null;
    }
  }

  /** Serialize project state for persistence */
  serialize(): string {
    return JSON.stringify({
      rootPath: this.state.rootPath,
      files: Array.from(this.state.files.entries()),
      openFiles: this.state.openFiles,
      activeFile: this.state.activeFile,
    }, null, 2);
  }

  /** Deserialize project state */
  deserialize(json: string): void {
    const data = JSON.parse(json);
    this.state.rootPath = data.rootPath;
    this.state.files = new Map(data.files);
    this.state.openFiles = data.openFiles || [];
    this.state.activeFile = data.activeFile || null;
  }

  private classifyFile(ext: string): ProjectFile["type"] {
    switch (ext.toLowerCase()) {
      case "nomx": return "nomx";
      case "nom": return "nom";
      case "nomtu": return "nomtu";
      case "png": case "jpg": case "avif": case "mp4": case "wav": case "svg": return "media";
      default: return "other";
    }
  }

  private notifyChange(): void {
    const files = Array.from(this.state.files.values());
    for (const cb of this.changeListeners) cb(files);
  }
}
