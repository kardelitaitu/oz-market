# Validation Checklist - Transactional Outbox Pattern

This checklist is used to confirm the completion of Spec 0026:

- [ ] Outbox table schema migration is applied with `status` and `published_at` columns.
- [ ] Outbox events are created inside the same active transactional context (`sqlx::Transaction`) as database writes.
- [ ] Polling background runner retrieves pending events and skips locked rows.
- [ ] Events are marked as published upon dispatch.
- [ ] Stale published records are swept cleanly after 24 hours.
