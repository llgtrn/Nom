/** Shared TypeScript types used across NomCanvas modules.
 * Eliminates inline duplicates of common Tauri command result types. */

/** Result from compile_block Tauri command */
export interface CompileResult {
    success: boolean;
    diagnostics: string[];
    entities: string[];
}

/** Result from score_block Tauri command */
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

/** Result from wire_check Tauri command */
export interface WireCheckResult {
    status: "compatible" | "needs_adapter" | "incompatible";
    reason: string | null;
}

/** Result from security_scan Tauri command */
export interface SecurityScanResult {
    findings: string[];
    risk_level: string;
}

/** Result from plan_flow Tauri command */
export interface PlanFlowResult {
    nodes: number;
    edges: number;
    fusion_passes: string[];
}

/** Result from dream_report Tauri command */
export interface DreamReportResult {
    score: number;
    proposals: string[];
    dict_hints: string[];
}
