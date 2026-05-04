04-05-26--11-31
- added authz enforcement layer and service wrappers in backend/server so scope, role, and ownership checks follow the whitepaper instead of being ad hoc
- added idempotency enforcement with idempotency key storage and replay handling so create/open flows can be retried safely without duplicate writes
