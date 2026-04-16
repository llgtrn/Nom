import { invoke } from "@tauri-apps/api/core";

export interface QualityScores {
  security: number;
  reliability: number;
  performance: number;
  readability: number;
  testability: number;
  portability: number;
  composability: number;
  maturity: number;
  overall: number;
}

const DIMENSION_LABELS: [keyof QualityScores, string][] = [
  ["security", "SEC"],
  ["reliability", "REL"],
  ["performance", "PERF"],
  ["readability", "READ"],
  ["testability", "TEST"],
  ["portability", "PORT"],
  ["composability", "COMP"],
  ["maturity", "MAT"],
];

function scoreColor(score: number): string {
  if (score >= 0.8) return "#22C55E";  // green — good
  if (score >= 0.5) return "#F59E0B";  // amber — needs work
  return "#EF4444";                     // red — poor
}

/** Render a quality score bar into a target element */
export function renderQualityBar(container: HTMLElement, scores: QualityScores): void {
  container.replaceChildren();
  container.className = "quality-bar";

  // Overall score badge (larger)
  const overallEl = document.createElement("span");
  overallEl.className = "quality-overall";
  const overallPct = (scores.overall * 100).toFixed(0);
  overallEl.style.color = scoreColor(scores.overall);
  overallEl.style.borderColor = scoreColor(scores.overall);
  overallEl.textContent = `Q${overallPct}`;
  overallEl.title = `Overall quality: ${overallPct}%`;
  container.appendChild(overallEl);

  // Individual dimension badges
  for (const [key, label] of DIMENSION_LABELS) {
    const score = scores[key];
    const badge = document.createElement("span");
    badge.className = "quality-badge";
    badge.style.borderColor = scoreColor(score);
    badge.style.color = scoreColor(score);
    badge.textContent = `${label} ${(score * 100).toFixed(0)}`;
    badge.title = `${key}: ${(score * 100).toFixed(0)}%`;
    container.appendChild(badge);
  }
}

/** Fetch quality scores for a block and render them */
export async function scoreAndRender(source: string, container: HTMLElement): Promise<void> {
  try {
    const scores = await invoke<QualityScores>("score_block", { source });
    renderQualityBar(container, scores);
  } catch (e) {
    container.textContent = `// score error: ${e}`;
    container.className = "quality-bar quality-error";
  }
}
