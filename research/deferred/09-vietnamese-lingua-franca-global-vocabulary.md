# Part 9: Vietnamese as Lingua Franca, Global Vocabulary as Surface

**How Novel can treat Vietnamese as the lingua-franca grammar of programming while allowing vocabulary, aliases, scripts, and learning surfaces from any world language without fragmenting meaning or disrespecting culture.**

Research conducted: 2026-04-11
Focus: Vietnamese as grammar model, multilingual vocabulary architecture, locale standards, Unicode-safe source design

> **2026-04-13 status update (supersedes §3.2, §9, §12, §7.3 examples):** Per user directive, Nom vocabulary is **fully English**. Vietnamese contributes **grammar STYLE only** (anchored-flexible word order, classifier phrases, effect valence) — zero VN tokens in the codebase. Commit `ecd0609` removed all VN keyword arms from the lexer, tests, and demos. The shipped `vi-VN` locale pack has `keyword_aliases` **intentionally empty**; localized-keyword packs (this doc's §9 VN/JA/ES surfaces, §12 Phase 3) are **not on the roadmap**. Localization is retained for diagnostics, docs, and register metadata only.

---

## Executive Summary

Your idea can be stated very clearly:

> Use Vietnamese as the deep grammar and awareness model for Novel, but let people all over the world speak the language through their own vocabulary, scripts, and cultural registers.

This is not only possible.
It is one of the strongest directions Novel could take.

But it only works if we separate **five layers** that many language projects blur together:

1. **grammar**
2. **semantics**
3. **surface vocabulary**
4. **script/orthography**
5. **culture/community ownership**

The key design move is:

**Vietnamese becomes the lingua-franca grammar, not the only visible lexicon.**

That means:

- the structural logic of the language is Vietnamese-inspired
- the canonical semantic core is language-neutral
- the human-facing vocabulary can be localized per language and script
- all surface variants normalize to one canonical AST and one Nom graph

If done correctly, Novel can become:

- Vietnamese in grammar
- global in vocabulary
- culture-respectful in presentation
- deterministic in compilation

That is the right combination.

---

## 1. What It Means To Treat Vietnamese As Lingua Franca

This does **not** mean:

- everyone must write Vietnamese words
- everyone must use Latin script
- everyone must learn Vietnamese vocabulary first
- the language becomes culturally extractive by borrowing Vietnamese structure but erasing Vietnamese identity

It **does** mean:

- Vietnamese provides the **relational model**
- Vietnamese provides the **compression strategy**
- Vietnamese provides the **disambiguation philosophy**
- Vietnamese provides the **grammar skeleton**

In other words:

```text
Vietnamese = deep grammar and cognition model
Novel = semantic machine language for that grammar
World languages = local lexical skins on top of the same meaning
```

This is similar to how some human lingua francas work socially:

- one shared structural system
- many local lexical accommodations
- strong normalization beneath variation

But Novel can do this more cleanly than human language because it has:

- canonical IDs
- typed contracts
- compiler normalization
- explicit locale metadata

---

## 2. Why Vietnamese Is A Strong Grammar Base

Vietnamese is unusually strong as a grammar model for a programming language because it combines:

- **analyticity**: words do not mutate
- **head-first composition**: declare the thing, then constrain it
- **topic-awareness**: what a program is about can be surfaced cleanly
- **classifier logic**: immediate semantic anchoring
- **serial predication**: compact event chaining
- **discourse recoverability**: light surface, stable interpretation
- **register layering**: native, Hán Việt, and global loanword coexistence

These are not aesthetic features only.
They are exactly the kinds of properties a globally usable semantic language needs.

That is why "Vietnamese as lingua franca" is more than symbolism.
It is a computational design choice.

---

## 3. The Core Separation: Grammar Is Fixed, Vocabulary Is Open

This is the most important rule.

### 3.1 Fixed globally

These should stay globally canonical:

- declaration order
- composition relations
- flow direction
- constraint direction
- classifier roles
- effect semantics
- canonical AST
- NomID identity

### 3.2 Localizable globally

These can be localized:

- keywords
- Nom aliases
- documentation
- diagnostics
- tutorials
- standard library surface names
- comment conventions
- editor labels and explanations

### 3.3 Never localize semantics

Do **not** let translations change:

- contract meaning
- operator precedence
- effect boundaries
- runtime guarantees
- security metadata
- Nom identity

That would split the language.

So the clean formula is:

```text
fixed grammar + fixed semantics + localized vocabulary = one world language
```

---

## 4. Architecture: One Semantic Core, Many Human Surfaces

Novel should be built in four layers:

### Layer 1: Canonical semantic core

This is the machine truth:

- NomID
- kind
- contract
- effect set
- provenance
- scores
- backend metadata

This layer has no dependency on natural language.

### Layer 2: Canonical grammar core

This is the Vietnamese-inspired relational skeleton:

- classifier first
- head before modifier
- qualifier after head
- serial flow chaining
- optional topic-first framing with explicit markers

This layer is also global and fixed.

### Layer 3: Locale vocabulary packs

This is where global adoption happens.

Each locale pack contains:

- localized keywords
- alias lists for common Nom families
- register preferences
- script preferences
- display forms
- educational glosses

### Layer 4: Tooling and learning surface

This is where the IDE, compiler, and report system help the user:

- explain the normalized form
- show alternate aliases
- warn on ambiguous mixed-script/confusable input
- teach the canonical semantics beneath local forms

---

## 5. The Right Mental Model: Grammar Lingua Franca, Lexical Democracy

If Novel becomes global, it should not choose between:

- Vietnamese identity
- global accessibility

It should do both.

The right principle is:

**grammar is singular, vocabulary is plural.**

That means Novel can have:

- one underlying language
- many legitimate ways to say the same concept

All of these should be able to map to the same NomID:

- plain Vietnamese
- Hán Việt
- English
- Arabic
- Spanish
- Japanese
- Swahili
- Hindi
- etc.

This is lexical democracy without semantic fragmentation.

---

## 6. Standards Novel Should Use

If Novel supports world language vocabulary, it must do it on real internationalization standards.

### 6.1 Language tags: BCP 47 — ✅ shipped (M3a)

W3C guidance around BCP 47 is the correct base for language tagging.
`nom-locale::LocaleTag::parse` ships M3a: language + script + region + variants, with extension/private-use subtags captured as `unsupported`. Novel tags localized resources using standard language tags such as:

- `vi`
- `en`
- `es`
- `ar`
- `ja`
- `zh-Hans`
- `zh-Hant`
- `sr-Cyrl`
- `sr-Latn`

This prevents vague labels like "Chinese" or "Serbian" where script/region matter.

### 6.2 Locale data: CLDR

Unicode CLDR should be used for locale metadata and language display conventions.

This is important for:

- language names
- script names
- region-sensitive presentation
- sorting/display consistency

### 6.3 Unicode identifiers: UAX #31

If users can write identifiers in many scripts, Novel should use a Unicode identifier profile based on UAX #31.

This gives a real standard for:

- which characters can start identifiers
- which characters can continue identifiers
- how to define identifier-safe profiles

### 6.4 Normalization: UAX #15 — ✅ shipped

Novel source normalizes Unicode text to NFC via the `unicode_normalization` crate in `nom-locale`. Without this, visually identical identifiers can have different binary forms.

### 6.5 Security and confusables: UTS #39

The moment multiple scripts are allowed, you need confusable detection.

Novel must detect:

- mixed-script confusables
- whole-script confusables
- suspicious homograph identifiers

Otherwise multilingual support becomes a security hole.

> **Status (2026-04-13):** M3b-minimal shipped in `nom-locale` — `is_confusable()` + `ConfusableResult` backed by ~30 baked high-value pairs (Cyrillic/Greek/Latin homographs). Full UTS #39 `confusables.txt` (~15K entries) import is the remaining TODO for M3b-full.

### 6.6 Source code handling: UTS #55

Unicode source code handling guidance matters for:

- bidirectional text safety
- display stability
- secure rendering of source files

### 6.7 Transliteration support: ICU

ICU transforms are useful for:

- optional transliteration
- search/index fallback
- script conversion support
- user-friendly alias lookup

But transliteration must remain an **aid**, not the canonical source form.

---

## 7. A Better Novel Model For Worldwide Adoption

Novel should support three kinds of names for every important concept.

### 7.1 Canonical machine name

Internal only:

- `NomID`

### 7.2 Canonical educational gloss

One stable explanation string that defines the concept.

Example:

```text
authenticate = prove identity and establish trust for access
```

This is the semantic anchor for translators and users.

### 7.3 Localized aliases

Per locale pack:

- preferred alias
- accepted aliases
- deprecated aliases
- register tags
- script tags

Example conceptually:

```yaml
nom: authenticate
nom_id: 02-...
gloss: prove identity and establish trust for access
aliases:
  vi:
    preferred: xac_thuc
    accepted: [xac-minh]
    register: han_viet
  en:
    preferred: authenticate
    accepted: [auth]
    register: technical
  es:
    preferred: autenticar
  ar:
    preferred: تحقق_الهوية
  ja:
    preferred: 認証
```

All of these should resolve to the same semantic node.

---

## 8. How Source Code Would Work

Here is the clean design:

### 8.1 Source files declare a primary lexical locale

Example:

```novel
locale en
lexicon global
```

or

```novel
locale ar
lexicon global
script Arab
```

This does **not** change grammar.
It changes only the active vocabulary pack.

### 8.2 Parser reads localized keywords, then canonicalizes

The parser should:

1. decode UTF-8
2. normalize source text
3. tokenize via Unicode identifier profile
4. resolve localized keywords/aliases from the active locale pack
5. rewrite to canonical grammar tokens
6. continue parsing as one global language

### 8.3 Compiler emits one canonical AST

No matter what human lexicon was used, the compiler should emit the same AST/IR.

This is the non-negotiable core.

---

## 9. Example: Same Novel, Different Lexical Worlds

Same underlying semantics:

```text
flow login:
  request -> authenticate -> response
```

Possible localized surfaces:

### English lexical pack

```novel
flow login {
    need authenticate
    flow: request -> authenticate -> response
}
```

### Vietnamese lexical pack

```novel
luong dang_nhap {
    can xac_thuc
    luong: yeu_cau -> xac_thuc -> phan_hoi
}
```

### Spanish lexical pack

```novel
flujo inicio_sesion {
    necesita autenticar
    flujo: solicitud -> autenticar -> respuesta
}
```

### Japanese lexical pack

```novel
流れ ログイン {
    必要 認証
    流れ: 要求 -> 認証 -> 応答
}
```

The exact keyword choices would require human curation.
The point is structural:

- the **order** stays Novel
- the **meaning** stays Novel
- the **surface words** change by locale

That is how Vietnamese can be the grammar lingua franca without becoming a lexical empire.

---

## 10. Cultural Respect Rules

If Novel wants to respect world cultures, it needs policy, not just parser support.

### Rule 1: Endonym-first, not English-first

Locale packs should use names that native speakers recognize as natural in their own linguistic tradition.

### Rule 2: Human review over machine translation

Do not auto-generate core keywords by machine translation and call it localization.
Keywords, aliases, and glosses need community review.

### Rule 3: Keep script dignity

Do not require transliteration into Latin script.
Allow Arabic, Devanagari, Hangul, Kana/Kanji, Cyrillic, Thai, etc.

### Rule 4: Distinguish locale, language, and script

Do not flatten:

- Serbian Latin and Serbian Cyrillic
- Simplified and Traditional Chinese
- regional orthographic preferences

### Rule 5: Protect against cultural flattening

Do not let one "global" pack overwrite all local distinctions for convenience.

### Rule 6: Preserve provenance

Every localized alias should record:

- who proposed it
- who reviewed it
- which locale/script it belongs to
- whether it is beginner, formal, technical, or deprecated

### Rule 7: Prefer coexistence over replacement

If two cultures or regions use different technical vocabulary for the same concept, keep both as aliases if both are valid.

---

## 11. The Main Risks

### 11.1 Synonym explosion

If too many aliases are accepted without governance, readability collapses.

### 11.2 Security attacks through confusables

Multiscript identifiers create spoofing risk immediately.

### 11.3 Translation drift

If a localized keyword gradually shifts in meaning, semantic confusion appears between communities.

### 11.4 Tooling burden

Editor, formatter, linter, parser, diagnostics, and docs all become harder when vocabulary is localized.

### 11.5 Cultural asymmetry

If Vietnamese provides the deep grammar but English still dominates the visible tooling and docs, the system will quietly re-centralize around English.

Novel must avoid that.

---

## 12. Recommended Novel Strategy

### Phase 1

Keep:

- one canonical grammar
- one canonical keyword set
- multilingual Nom aliases only

This is the safest MVP.

### Phase 2

Add:

- locale-tagged alias packs
- localized diagnostics
- localized docs and tutorials

### Phase 3

Add:

- localized keyword packs
- transliteration-aware search
- culture-reviewed global community governance

### Phase 4

Add:

- per-file locale declarations
- mixed-locale workspaces with explicit boundaries
- locale-aware formatter and linter rules

This phased path lets Novel grow global without losing determinism.

---

## 13. The Best Final Form

The strongest version of this idea is:

```text
Vietnamese gives Novel its deep grammar.
Nom gives Novel its semantic core.
Every world language gets to speak that grammar through its own lexicon.
The compiler sees one language.
Humans see their own.
```

That is a genuinely new model of programming-language internationalization.

It is not:

- English with translated docs
- a bag of multilingual identifiers
- a locale skin over an English core

It is:

- Vietnamese-informed computation
- global lexical participation
- culture-aware canonical semantics

---

## 14. Primary References

- Unicode UAX #31, Unicode Identifier and Pattern Syntax  
  https://www.unicode.org/reports/tr31/

- Unicode UAX #15, Unicode Normalization Forms  
  https://www.unicode.org/reports/tr15/

- Unicode UTS #39, Unicode Security Mechanisms  
  https://www.unicode.org/reports/tr39/

- Unicode UTS #55, Unicode Source Code Handling  
  https://www.unicode.org/reports/tr55/

- Unicode CLDR Project  
  https://cldr.unicode.org/

- W3C, Understanding the New Language Tags (BCP 47)  
  https://www.w3.org/International/articles/bcp47/

- ICU Transforms / Transliteration  
  https://unicode-org.github.io/icu/userguide/transforms/general/

- Hedy official site  
  https://www.hedy.org/

- Hedy, A Framework for the Localization of Programming Languages  
  https://www.hedy.org/research/A_Framework_for_the_Localization_of_Programming_Languages_2023.pdf

---

## Final Take

If Novel wants to treat Vietnamese as a lingua franca for programming, it should not ask the world to become Vietnamese on the surface.

It should ask the world to share a Vietnamese-inspired way of structuring meaning,
while letting each culture speak that structure in its own words.

That is how Novel can be both deeply Vietnamese and genuinely global.
