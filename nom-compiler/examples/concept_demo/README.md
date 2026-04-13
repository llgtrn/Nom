# concept_demo

Smallest end-to-end example of the layered concept architecture
(research/language-analysis/08-layered-concept-component-architecture.md).

Pipeline:

    nom store sync .             # parses *.nom + *.nomtu, writes DB1+DB2
    nom build status .           # walks closures, resolves prose-matching refs
    nom build status . --write-locks   # rewrites *.nom to pin resolved hashes

Files:
- app.nom                       — root concept
- auth/auth.nom                 — the authentication_demo concept (initially uses prose `matching "..."`)
- auth/auth_helpers.nomtu       — 2 helper entities + 1 composition module

After first build:
- DB has 2 concept_defs rows + 3 words_v2 rows.
- auth/auth.nom has been rewritten so `the module auth_session_compose_demo`
  becomes `the module auth_session_compose_demo@<hash>`.
- Subsequent builds are reproducible: same source, same DB, same hashes.
