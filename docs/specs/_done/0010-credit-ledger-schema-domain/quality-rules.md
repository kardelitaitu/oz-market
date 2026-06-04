# Quality Rules - Credit/Balance DB Schema and Domain Logic

- **No SQL Interpolation**: All SQL queries executing credit updates or ledger queries must use strict parameterized placeholder queries.
- **Negative Balance Constraint**: The `agent_balances` table must contain a SQL check constraint: `CHECK (balance_credits >= 0.0000)`.
- **Compile-Time Checks**: The domain enum mapping must be exhaustive to prevent uncaught transaction type additions.
