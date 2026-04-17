import { invoke } from "@tauri-apps/api/core";

export interface RAGContext {
    query: string;
    retrievedEntities: RAGEntity[];
    assembledPrompt: string;
    confidence: number;
}

export interface RAGEntity {
    word: string;
    kind: string;
    signature: string | null;
    score: number;
    source: "dictionary" | "pattern" | "recent";
}

export class RAGKnowledgeBase {
    private recentContext: RAGEntity[] = [];
    private maxRecent = 20;

    /** Retrieve relevant entities for a query */
    async retrieve(query: string, limit = 10): Promise<RAGEntity[]> {
        const entities: RAGEntity[] = [];

        // Phase 1: Dictionary search
        try {
            const dictResults = await invoke<Array<{ word: string; kind: string; score: number }>>(
                "search_dict", { query }
            );
            for (const r of dictResults.slice(0, limit)) {
                entities.push({
                    word: r.word, kind: r.kind, signature: null,
                    score: r.score, source: "dictionary",
                });
            }
        } catch { /* graceful degradation */ }

        // Phase 2: Grammar pattern search
        try {
            const patterns = await invoke<Array<{ pattern: string; score: number }>>(
                "match_grammar", { input: query }
            );
            for (const p of patterns.slice(0, 5)) {
                entities.push({
                    word: p.pattern, kind: "pattern", signature: null,
                    score: p.score, source: "pattern",
                });
            }
        } catch { /* graceful degradation */ }

        // Phase 3: Recent context
        for (const recent of this.recentContext.slice(0, 5)) {
            if (!entities.some(e => e.word === recent.word)) {
                entities.push({ ...recent, source: "recent" });
            }
        }

        // Sort by score descending
        entities.sort((a, b) => b.score - a.score);
        return entities.slice(0, limit);
    }

    /** Assemble a RAG context for AI prompting */
    async assembleContext(query: string): Promise<RAGContext> {
        const entities = await this.retrieve(query);

        // Build context prompt
        const entityDescriptions = entities
            .map(e => `- ${e.word} (${e.kind}): ${e.signature || "no signature"}`)
            .join("\n");

        const assembledPrompt = [
            "## Available Dictionary Entities",
            entityDescriptions || "(no matching entities found)",
            "",
            "## User Query",
            query,
            "",
            "## Instructions",
            "Using the available entities above, help the user write valid .nomx source.",
            "Each entity declaration follows: the <kind> <name> is intended to <purpose>.",
        ].join("\n");

        const confidence = entities.length > 0
            ? entities.reduce((sum, e) => sum + e.score, 0) / entities.length
            : 0;

        return { query, retrievedEntities: entities, assembledPrompt, confidence };
    }

    /** Add to recent context */
    addToRecent(entity: RAGEntity): void {
        this.recentContext = [entity, ...this.recentContext.filter(e => e.word !== entity.word)].slice(0, this.maxRecent);
    }

    /** Clear recent context */
    clearRecent(): void {
        this.recentContext = [];
    }

    /** Get recent context */
    getRecent(): RAGEntity[] {
        return [...this.recentContext];
    }
}
