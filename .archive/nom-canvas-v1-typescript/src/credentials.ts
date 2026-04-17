import { invoke } from "@tauri-apps/api/core";

interface CredentialResult {
  success: boolean;
  value: string | null;
  error: string | null;
}

export interface CredentialEntry {
  key: string;
  hasValue: boolean;
}

const KNOWN_KEYS = [
  { key: "anthropic_api_key", label: "Anthropic API Key", placeholder: "sk-ant-..." },
  { key: "openai_api_key", label: "OpenAI API Key", placeholder: "sk-..." },
  { key: "github_token", label: "GitHub Token", placeholder: "ghp_..." },
  { key: "custom_llm_endpoint", label: "Custom LLM Endpoint", placeholder: "https://..." },
];

export class CredentialManager {
  private panel: HTMLElement;
  private isOpen = false;

  constructor() {
    this.panel = document.createElement("div");
    this.panel.className = "credential-panel";
    this.panel.style.display = "none";

    const header = document.createElement("div");
    header.className = "credential-header";

    const title = document.createElement("h3");
    title.textContent = "Credentials";
    header.appendChild(title);

    const closeBtn = document.createElement("button");
    closeBtn.className = "credential-close";
    closeBtn.textContent = "X";
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);

    this.panel.appendChild(header);

    const form = document.createElement("div");
    form.className = "credential-form";

    for (const { key, label, placeholder } of KNOWN_KEYS) {
      const row = document.createElement("div");
      row.className = "credential-row";

      const lbl = document.createElement("label");
      lbl.textContent = label;
      lbl.className = "credential-label";
      row.appendChild(lbl);

      const input = document.createElement("input");
      input.type = "password";
      input.placeholder = placeholder;
      input.className = "credential-input";
      input.dataset.key = key;
      row.appendChild(input);

      const saveBtn = document.createElement("button");
      saveBtn.className = "credential-save";
      saveBtn.textContent = "Save";
      saveBtn.addEventListener("click", async () => {
        const value = input.value.trim();
        if (!value) return;
        const result = await invoke<CredentialResult>("store_credential", { key, value });
        if (result.success) {
          input.value = "";
          input.placeholder = "Saved";
          status.textContent = "Saved";
          status.className = "credential-status saved";
        } else {
          status.textContent = result.error || "Error";
          status.className = "credential-status error";
        }
      });
      row.appendChild(saveBtn);

      const status = document.createElement("span");
      status.className = "credential-status";
      row.appendChild(status);

      form.appendChild(row);
    }

    this.panel.appendChild(form);

    const note = document.createElement("p");
    note.className = "credential-note";
    note.textContent = "Credentials are stored locally at ~/.nom/credentials/. In production, use OS keyring.";
    this.panel.appendChild(note);

    document.body.appendChild(this.panel);
  }

  open(): void {
    this.isOpen = true;
    this.panel.style.display = "block";
    this.checkExisting();
  }

  close(): void {
    this.isOpen = false;
    this.panel.style.display = "none";
  }

  toggle(): void {
    if (this.isOpen) this.close(); else this.open();
  }

  private async checkExisting(): Promise<void> {
    for (const { key } of KNOWN_KEYS) {
      const result = await invoke<CredentialResult>("get_credential", { key });
      const input = this.panel.querySelector(`[data-key="${key}"]`) as HTMLInputElement;
      if (input && result.success && result.value) {
        input.placeholder = "*** saved ***";
      }
    }
  }

  getElement(): HTMLElement { return this.panel; }
}
