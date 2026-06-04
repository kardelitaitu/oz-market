---
id: 0018-update-affected-documents
title: Update Affected Documents
status: active
owner: backend-team
implementer: agent
priority: P3
---

# Update Affected Documents

Status: `active`
Implementer: `agent`

## Summary

This specification governs updates to all project-wide documents, roadmaps, readmes, and checklists. It ensures they remain aligned with implemented features and the active specification library.

## Scope

### In Scope
- Synchronizing checklists in `TODO.md` with implementation status.
- Keeping index documents like `docs/DOCS-README.md` and `docs/specs/README.md` updated with completed/active specifications.
- Updating `AGENTS.md` rules if governance parameters change.

### Out of Scope
- Code changes or database schema modifications.

## Proposed Direction
1. Review implementation status of Phase 2 and Phase 3:
   - Mark completed items in `TODO.md` as checked.
2. Synchronize Spec Indices:
   - Add new active specs (`0014`, `0015`, `0016`, `0017`, `0018`) to lists in `docs/specs/README.md`.
