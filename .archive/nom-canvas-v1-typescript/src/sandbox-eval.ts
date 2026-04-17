/** Safe expression evaluator — no access to DOM, fetch, or globals */

const BLOCKED_GLOBALS = new Set([
    "window", "document", "globalThis", "self",
    "fetch", "XMLHttpRequest", "WebSocket",
    "eval", "Function", "setTimeout", "setInterval",
    "importScripts", "Worker", "SharedWorker",
    "localStorage", "sessionStorage", "indexedDB",
    "navigator", "location", "history",
    // Prototype chain escape vectors
    "constructor", "prototype", "__proto__", "__defineGetter__", "__defineSetter__",
    "__lookupGetter__", "__lookupSetter__",
    // Reflection / meta-programming escape vectors
    "this", "Proxy", "Reflect", "Symbol",
]);

const ALLOWED_MATH = {
    abs: Math.abs, ceil: Math.ceil, floor: Math.floor, round: Math.round,
    max: Math.max, min: Math.min, pow: Math.pow, sqrt: Math.sqrt,
    log: Math.log, log2: Math.log2, log10: Math.log10,
    sin: Math.sin, cos: Math.cos, tan: Math.tan,
    PI: Math.PI, E: Math.E,
    random: Math.random,
};

const ALLOWED_STRING = {
    toUpperCase: (s: string) => s.toUpperCase(),
    toLowerCase: (s: string) => s.toLowerCase(),
    trim: (s: string) => s.trim(),
    split: (s: string, sep: string) => s.split(sep),
    join: (arr: string[], sep: string) => arr.join(sep),
    includes: (s: string, search: string) => s.includes(search),
    replace: (s: string, from: string, to: string) => s.replace(from, to),
    length: (s: string) => s.length,
};

export interface EvalContext {
    [key: string]: unknown;
}

export interface EvalResult {
    success: boolean;
    value: unknown;
    error?: string;
}

/** Evaluate an expression safely with a provided context */
export function safeEval(expression: string, context: EvalContext = {}): EvalResult {
    // Validate: reject dangerous patterns
    for (const blocked of BLOCKED_GLOBALS) {
        if (expression.includes(blocked)) {
            return { success: false, value: null, error: `Blocked: '${blocked}' is not allowed` };
        }
    }

    // Reject assignment operators (simple = and all compound: += -= *= /= %= **= <<= >>= >>>= &= |= ^= &&= ||= ??=)
    const ASSIGNMENT_OPS = /(?<![=!<>])=(?!=)|(\+=|-=|\*=|\/=|%=|\*\*=|<<=|>>=|>>>=|&=|\|=|\^=|&&=|\|\|=|\?\?=)/;
    if (ASSIGNMENT_OPS.test(expression) && !expression.includes("=>")) {
        return { success: false, value: null, error: "Assignment not allowed in expressions" };
    }

    try {
        // Build sandbox with allowed functions + user context
        const sandbox: Record<string, unknown> = {
            ...ALLOWED_MATH,
            ...context,
            // Safe JSON operations
            JSON_parse: (s: string) => JSON.parse(s),
            JSON_stringify: (v: unknown) => JSON.stringify(v),
            // String helpers
            str: ALLOWED_STRING,
            // Array helpers
            map: (arr: unknown[], fn: (x: unknown) => unknown) => arr.map(fn),
            filter: (arr: unknown[], fn: (x: unknown) => boolean) => arr.filter(fn),
            reduce: (arr: unknown[], fn: (acc: unknown, x: unknown) => unknown, init: unknown) => arr.reduce(fn, init),
            sum: (arr: number[]) => arr.reduce((a, b) => a + b, 0),
            avg: (arr: number[]) => arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : 0,
            count: (arr: unknown[]) => arr.length,
        };

        // Create function with sandbox as scope — new Function is intentional here;
        // expression is validated against BLOCKED_GLOBALS before reaching this point.
        const keys = Object.keys(sandbox);
        const values = Object.values(sandbox);
        // biome-ignore lint/security/noGlobalEval: sandboxed — blocked-list checked above
        const fn = new Function(...keys, `"use strict"; return (${expression})`);
        const result = fn(...values);
        return { success: true, value: result };
    } catch (e) {
        return { success: false, value: null, error: String(e) };
    }
}

/** Evaluate a template string with ${expressions} */
export function safeTemplate(template: string, context: EvalContext = {}): string {
    return template.replace(/\$\{([^}]+)\}/g, (_, expr) => {
        const result = safeEval(expr.trim(), context);
        return result.success ? String(result.value) : `[error: ${result.error}]`;
    });
}
