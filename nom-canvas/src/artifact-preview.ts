import { invoke } from "@tauri-apps/api/core";
import { CompileResult } from "./types";

export type ArtifactType = "html" | "code" | "json" | "media" | "binary" | "unknown";

export interface Artifact {
  type: ArtifactType;
  content: string;
  mimeType: string;
  entityName: string;
  hash: string;
}

export class ArtifactPreview {
  private container: HTMLElement;
  private iframe: HTMLIFrameElement | null = null;

  constructor(containerId: string) {
    this.container = document.getElementById(containerId) || document.createElement("div");
    this.container.className = "artifact-preview";
  }

  /** Preview a compiled artifact */
  render(artifact: Artifact): void {
    this.container.replaceChildren();

    switch (artifact.type) {
      case "html":
        this.renderHTML(artifact.content);
        break;
      case "code":
        this.renderCode(artifact.content, artifact.entityName);
        break;
      case "json":
        this.renderJSON(artifact.content);
        break;
      case "media":
        this.renderMedia(artifact.content, artifact.mimeType);
        break;
      default:
        this.renderText(artifact.content);
    }
  }

  private renderHTML(html: string): void {
    this.iframe = document.createElement("iframe");
    this.iframe.className = "artifact-iframe";
    // Sandboxed without scripts — safe from XSS
    this.iframe.setAttribute("sandbox", "");
    const cspMeta = `<meta http-equiv="Content-Security-Policy" content="default-src 'none';">`;
    const hasHead = /<head[^>]*>/i.test(html);
    const safeHtml = hasHead
      ? html.replace(/(<head[^>]*>)/i, `$1${cspMeta}`)
      : `<head>${cspMeta}</head>${html}`;
    this.iframe.srcdoc = safeHtml;
    this.container.appendChild(this.iframe);
  }

  private renderCode(code: string, name: string): void {
    const header = document.createElement("div");
    header.className = "artifact-header";
    header.textContent = name;
    this.container.appendChild(header);

    const pre = document.createElement("pre");
    pre.className = "artifact-code";
    const codeEl = document.createElement("code");
    codeEl.textContent = code;
    pre.appendChild(codeEl);
    this.container.appendChild(pre);
  }

  private renderJSON(json: string): void {
    try {
      const formatted = JSON.stringify(JSON.parse(json), null, 2);
      this.renderCode(formatted, "JSON Output");
    } catch {
      this.renderText(json);
    }
  }

  private renderMedia(src: string, mime: string): void {
    if (mime.startsWith("image/")) {
      const img = document.createElement("img");
      img.className = "artifact-image";
      img.src = src;
      this.container.appendChild(img);
    } else if (mime.startsWith("audio/")) {
      const audio = document.createElement("audio");
      audio.controls = true;
      audio.src = src;
      this.container.appendChild(audio);
    } else if (mime.startsWith("video/")) {
      const video = document.createElement("video");
      video.controls = true;
      video.src = src;
      video.className = "artifact-video";
      this.container.appendChild(video);
    }
  }

  private renderText(text: string): void {
    const pre = document.createElement("pre");
    pre.className = "artifact-text";
    pre.textContent = text;
    this.container.appendChild(pre);
  }

  /** Compile source and preview the result */
  async compileAndPreview(source: string): Promise<void> {
    try {
      const result = await invoke<CompileResult>("compile_block", { source });
      if (result.success && result.entities.length > 0) {
        this.render({
          type: "code",
          content: JSON.stringify(result, null, 2),
          mimeType: "application/json",
          entityName: result.entities[0],
          hash: "",
        });
      } else {
        this.render({
          type: "code",
          content: result.diagnostics.join("\n") || "No output",
          mimeType: "text/plain",
          entityName: "Error",
          hash: "",
        });
      }
    } catch (e) {
      this.renderText(`Error: ${e}`);
    }
  }

  clear(): void { this.container.replaceChildren(); }
  getElement(): HTMLElement { return this.container; }
}
