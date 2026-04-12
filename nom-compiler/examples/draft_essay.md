# Continuous Deploy Pipeline

<!-- Long-form fixture: essay-style intent + nested sketch. Exercises
     section tracking across multiple `## Section` headers and
     preserves the concept grouping heuristic. -->

## Intent

Every time a developer pushes to the main branch, the source tree
should be built, tested, packaged, and rolled out to a small set of
canary servers. If the canaries stay healthy for ten minutes, the
change should fan out to the remaining fleet. If any canary fails
its health check, the pipeline should roll back to the previous
known-good build and page the on-call engineer.

## Sketch

- listen for git pushes to the main branch
- fetch the pushed commit
- build the source tree
- run the full test suite
- fail fast if any test fails
- package the build as a container image
- push the image to the registry
- deploy to the canary servers
- probe each canary for health
- wait ten minutes while watching canary metrics
- promote to the full fleet if canaries are green
- roll back to the previous image if any canary fails
- page the on-call engineer on rollback

## Constraints

- the pipeline must be idempotent so retries are safe
- build outputs must be content-addressed for rollback
- no step may take longer than five minutes alone
- logs from every step must survive for thirty days
