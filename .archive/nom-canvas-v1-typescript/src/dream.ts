import { invoke } from "@tauri-apps/api/core";

export interface DreamReport {
  score: number;
  proposals: string[];
  dict_hints: string[];
}

export class DreamPanel {
  private panel: HTMLElement;
  private scoreEl: HTMLElement;
  private proposalsList: HTMLElement;
  private hintsList: HTMLElement;
  private lastReport: DreamReport | null = null;

  constructor(containerId: string) {
    this.panel = document.createElement("div");
    this.panel.className = "dream-panel";
    this.panel.id = containerId;

    // Score header
    this.scoreEl = document.createElement("div");
    this.scoreEl.className = "dream-score";
    this.scoreEl.textContent = "Dream Score: --";
    this.panel.appendChild(this.scoreEl);

    // Proposals section
    const proposalsHeader = document.createElement("h4");
    proposalsHeader.textContent = "Proposals";
    proposalsHeader.className = "dream-section-header";
    this.panel.appendChild(proposalsHeader);

    this.proposalsList = document.createElement("ul");
    this.proposalsList.className = "dream-proposals";
    this.panel.appendChild(this.proposalsList);

    // Hints section
    const hintsHeader = document.createElement("h4");
    hintsHeader.textContent = "Dictionary Hints";
    hintsHeader.className = "dream-section-header";
    this.panel.appendChild(hintsHeader);

    this.hintsList = document.createElement("ul");
    this.hintsList.className = "dream-hints";
    this.panel.appendChild(this.hintsList);
  }

  /** Get the panel element for mounting */
  getElement(): HTMLElement { return this.panel; }

  /** Run dream report for a manifest */
  async dream(manifest: string): Promise<DreamReport> {
    try {
      const report = await invoke<DreamReport>("dream_report", { manifest });
      this.lastReport = report;
      this.render(report);
      return report;
    } catch (e) {
      const fallback: DreamReport = { score: 0, proposals: [`Error: ${e}`], dict_hints: [] };
      this.render(fallback);
      return fallback;
    }
  }

  /** Check if score meets the epic threshold (95) */
  isEpic(): boolean {
    return (this.lastReport?.score ?? 0) >= 95;
  }

  private render(report: DreamReport): void {
    // Score
    const pct = report.score.toFixed(0);
    const color = report.score >= 95 ? "#22C55E" :
                  report.score >= 70 ? "#F59E0B" : "#EF4444";
    this.scoreEl.textContent = `Dream Score: ${pct}`;
    this.scoreEl.style.color = color;
    this.scoreEl.style.borderColor = color;

    // Score bar
    const existingBar = this.scoreEl.querySelector(".score-bar-fill");
    if (existingBar) existingBar.remove();
    const bar = document.createElement("div");
    bar.className = "score-bar";
    const fill = document.createElement("div");
    fill.className = "score-bar-fill";
    fill.style.width = `${Math.min(100, report.score)}%`;
    fill.style.background = color;
    bar.appendChild(fill);
    this.scoreEl.appendChild(bar);

    // Proposals
    this.proposalsList.replaceChildren();
    for (const p of report.proposals) {
      const li = document.createElement("li");
      li.textContent = p;
      li.className = "dream-proposal-item";
      this.proposalsList.appendChild(li);
    }
    if (report.proposals.length === 0) {
      const li = document.createElement("li");
      li.textContent = "No proposals";
      li.className = "dream-empty";
      this.proposalsList.appendChild(li);
    }

    // Hints
    this.hintsList.replaceChildren();
    for (const h of report.dict_hints) {
      const li = document.createElement("li");
      li.textContent = h;
      li.className = "dream-hint-item";
      this.hintsList.appendChild(li);
    }
    if (report.dict_hints.length === 0) {
      const li = document.createElement("li");
      li.textContent = "No hints";
      li.className = "dream-empty";
      this.hintsList.appendChild(li);
    }
  }
}
