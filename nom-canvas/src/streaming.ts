import { invoke } from "@tauri-apps/api/core";

export interface StreamChunk {
  type: "token" | "entity" | "diagnostic" | "complete" | "error";
  content: string;
  timestamp: number;
}

export type StreamCallback = (chunk: StreamChunk) => void;

export class StreamingPipeline {
  private listeners: StreamCallback[] = [];
  private isStreaming = false;
  private abortController: AbortController | null = null;
  private generation = 0;

  onChunk(callback: StreamCallback): void {
    this.listeners.push(callback);
  }

  private emit(chunk: StreamChunk): void {
    for (const cb of this.listeners) cb(chunk);
  }

  /** Stream compilation results for a block of text.
   * NOTE: This is simulated streaming — words are emitted immediately as tokens,
   * then the full compile result arrives after the Tauri invoke completes.
   * True token-level streaming requires a streaming pipeline in nom-concept. */
  async streamCompile(source: string): Promise<void> {
    if (this.isStreaming) {
      this.cancel();
    }

    this.generation++;
    const myGeneration = this.generation;
    this.isStreaming = true;
    this.abortController = new AbortController();

    // Emit start
    if (myGeneration !== this.generation) return; // stale — newer stream started
    this.emit({ type: "token", content: "// compiling...", timestamp: Date.now() });

    try {
      // Phase 1: Tokenize (fast — emit immediately)
      const words = source.split(/\s+/).filter(w => w.length > 0);
      for (const word of words) {
        if (this.abortController.signal.aborted) break;
        if (myGeneration !== this.generation) return; // stale — newer stream started
        this.emit({ type: "token", content: word, timestamp: Date.now() });
      }

      // Phase 2: Compile via Tauri (async)
      if (!this.abortController.signal.aborted) {
        const result = await invoke<{
          success: boolean;
          diagnostics: string[];
          entities: string[];
        }>("compile_block", { source });

        if (myGeneration !== this.generation) return; // stale — newer stream started

        if (result.success) {
          for (const entity of result.entities) {
            if (myGeneration !== this.generation) return; // stale — newer stream started
            this.emit({ type: "entity", content: entity, timestamp: Date.now() });
          }
          this.emit({ type: "complete", content: `${result.entities.length} entities`, timestamp: Date.now() });
        } else {
          for (const diag of result.diagnostics) {
            if (myGeneration !== this.generation) return; // stale — newer stream started
            this.emit({ type: "diagnostic", content: diag, timestamp: Date.now() });
          }
          this.emit({ type: "error", content: result.diagnostics[0] || "compilation failed", timestamp: Date.now() });
        }
      }
    } catch (e) {
      if (myGeneration !== this.generation) return; // stale — newer stream started
      this.emit({ type: "error", content: String(e), timestamp: Date.now() });
    } finally {
      if (myGeneration === this.generation) {
        this.isStreaming = false;
        this.abortController = null;
      }
    }
  }

  /** Stream quality scoring */
  async streamScore(source: string): Promise<void> {
    this.emit({ type: "token", content: "// scoring...", timestamp: Date.now() });
    try {
      const scores = await invoke<Record<string, number>>("score_block", { source });
      for (const [dim, value] of Object.entries(scores)) {
        this.emit({
          type: "entity",
          content: `${dim}: ${(value * 100).toFixed(0)}%`,
          timestamp: Date.now(),
        });
      }
      this.emit({ type: "complete", content: "scoring done", timestamp: Date.now() });
    } catch (e) {
      this.emit({ type: "error", content: String(e), timestamp: Date.now() });
    }
  }

  /** Cancel current stream */
  cancel(): void {
    if (this.abortController) {
      this.abortController.abort();
    }
    this.isStreaming = false;
  }

  /** Check if currently streaming */
  getIsStreaming(): boolean {
    return this.isStreaming;
  }
}

/** Render streaming chunks into a container element */
export function createStreamRenderer(container: HTMLElement): StreamCallback {
  return (chunk: StreamChunk) => {
    switch (chunk.type) {
      case "token": {
        const span = document.createElement("span");
        span.className = "stream-token";
        span.textContent = chunk.content + " ";
        container.appendChild(span);
        break;
      }
      case "entity": {
        const div = document.createElement("div");
        div.className = "stream-entity";
        div.textContent = chunk.content;
        container.appendChild(div);
        break;
      }
      case "diagnostic": {
        const div = document.createElement("div");
        div.className = "stream-diagnostic";
        div.textContent = chunk.content;
        container.appendChild(div);
        break;
      }
      case "complete": {
        const div = document.createElement("div");
        div.className = "stream-complete";
        div.textContent = `// ${chunk.content}`;
        container.appendChild(div);
        break;
      }
      case "error": {
        const div = document.createElement("div");
        div.className = "stream-error";
        div.textContent = `// error: ${chunk.content}`;
        container.appendChild(div);
        break;
      }
    }
    // Auto-scroll to bottom
    container.scrollTop = container.scrollHeight;
  };
}
