/** Parser state — cached between keystrokes for incremental parsing */
export interface ParserState {
  position: number;
  inputHash: number; // hash of input[0..position] — detects edits before cursor
  insideEntity: boolean;
  insideContract: boolean;
  currentEntityKind: string | null;
  currentEntityName: string | null;
  entities: ParsedEntity[];
  errors: ParseError[];
}

export interface ParsedEntity {
  kind: string;
  name: string;
  startOffset: number;
  endOffset: number;
  signature: string | null;
  contracts: string[];
  effects: string[];
}

export interface ParseError {
  offset: number;
  length: number;
  message: string;
  severity: "error" | "warning" | "info";
}

const ENTITY_KINDS = new Set(["function", "module", "concept", "screen", "data", "event", "media", "property", "scenario"]);

export function createInitialState(): ParserState {
  return { position: 0, inputHash: simpleHash(""), insideEntity: false, insideContract: false, currentEntityKind: null, currentEntityName: null, entities: [], errors: [] };
}

function simpleHash(s: string): number {
  let hash = 5381;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) + hash + s.charCodeAt(i)) | 0;
  }
  return hash;
}

/** Resume parsing from cached state — only processes new content after state.position */
export function parseIncremental(input: string, state: ParserState): ParserState {
  // If content before the cached position changed, all offsets are invalid — restart
  const prefixHash = simpleHash(input.slice(0, state.position));
  if (prefixHash !== state.inputHash) {
    return parseIncremental(input, createInitialState());
  }

  const newState = { ...state, entities: [...state.entities], errors: [...state.errors] };
  let pos = newState.position;
  const lower = input.toLowerCase();

  while (pos < input.length) {
    // Look for "the <kind> <name> is" pattern
    if (!newState.insideEntity) {
      const theIdx = lower.indexOf("the ", pos);
      if (theIdx === -1) { pos = input.length; break; }

      // Check if next word is a kind
      const afterThe = theIdx + 4;
      let kindEnd = lower.indexOf(" ", afterThe);
      if (kindEnd === -1) { pos = input.length; break; } // incomplete

      const kind = lower.slice(afterThe, kindEnd);
      if (ENTITY_KINDS.has(kind)) {
        // Find entity name (next word after kind)
        const nameStart = kindEnd + 1;
        let nameEnd = lower.indexOf(" ", nameStart);
        if (nameEnd === -1) nameEnd = lower.indexOf(".", nameStart);
        if (nameEnd === -1) { pos = input.length; break; } // incomplete — early break

        const name = input.slice(nameStart, nameEnd).trim();
        newState.insideEntity = true;
        newState.currentEntityKind = kind;
        newState.currentEntityName = name;
        pos = nameEnd;
        continue;
      }
      pos = afterThe;
      continue;
    }

    // Inside entity — look for period (end of declaration) or contract keywords
    const periodIdx = input.indexOf(".", pos);
    const requiresIdx = lower.indexOf("requires ", pos);
    const ensuresIdx = lower.indexOf("ensures ", pos);
    const benefitIdx = lower.indexOf("benefit ", pos);
    const hazardIdx = lower.indexOf("hazard ", pos);

    if (periodIdx !== -1 && (periodIdx < requiresIdx || requiresIdx === -1) && (periodIdx < ensuresIdx || ensuresIdx === -1)) {
      // End of current entity block
      const entityText = input.slice(newState.position, periodIdx + 1);
      newState.entities.push({
        kind: newState.currentEntityKind!,
        name: newState.currentEntityName!,
        startOffset: newState.position,
        endOffset: periodIdx + 1,
        signature: extractSignature(entityText),
        contracts: extractContracts(entityText),
        effects: extractEffects(entityText),
      });
      newState.insideEntity = false;
      newState.currentEntityKind = null;
      newState.currentEntityName = null;
      pos = periodIdx + 1;
    } else {
      // Still accumulating — wait for more input (early break)
      break;
    }
  }

  newState.position = pos;
  newState.inputHash = simpleHash(input.slice(0, pos));
  return newState;
}

function extractSignature(text: string): string | null {
  const lower = text.toLowerCase();
  const givenIdx = lower.indexOf("given");
  const returnsIdx = lower.indexOf("returns");
  if (givenIdx !== -1 && returnsIdx !== -1) {
    return text.slice(givenIdx, text.indexOf(".", returnsIdx) + 1).trim();
  }
  if (returnsIdx !== -1) {
    return text.slice(returnsIdx, text.indexOf(".", returnsIdx) + 1).trim();
  }
  return null;
}

function extractContracts(text: string): string[] {
  const contracts: string[] = [];
  const lower = text.toLowerCase();
  for (const keyword of ["requires ", "ensures "]) {
    let idx = lower.indexOf(keyword);
    while (idx !== -1) {
      const end = text.indexOf(".", idx);
      if (end !== -1) contracts.push(text.slice(idx, end + 1).trim());
      idx = lower.indexOf(keyword, idx + keyword.length);
    }
  }
  return contracts;
}

function extractEffects(text: string): string[] {
  const effects: string[] = [];
  const lower = text.toLowerCase();
  for (const keyword of ["benefit ", "hazard "]) {
    let idx = lower.indexOf(keyword);
    while (idx !== -1) {
      const end = text.indexOf(".", idx);
      if (end !== -1) effects.push(text.slice(idx, end + 1).trim());
      idx = lower.indexOf(keyword, idx + keyword.length);
    }
  }
  return effects;
}

/** Full parse (non-incremental) — for initial load */
export function parseFull(input: string): ParserState {
  return parseIncremental(input, createInitialState());
}
