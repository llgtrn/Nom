# Superpowers Output Directory

This directory holds artifacts produced by Claude Code superpowers skills.

## Structure

```
superpowers/
├── plans/     # Implementation plans (from superpowers:writing-plans)
├── specs/     # Design specs (from superpowers:brainstorming)
```

## Naming Convention

Files follow: `<YYYY-MM-DD>-<kebab-case-name>.md`

## How These Are Used

1. **Specs** are created during brainstorming before implementation begins
2. **Plans** are created from specs, breaking work into checkboxed tasks
3. Agents read plans from `plans/` to know what to implement
4. Checkboxes in plan files track completion progress

## Integration

- **GitNexus** impact analysis is referenced in plans for blast radius
- **Ruflo** agents are assigned plan tasks for parallel execution
- Plans link back to specs for design rationale
