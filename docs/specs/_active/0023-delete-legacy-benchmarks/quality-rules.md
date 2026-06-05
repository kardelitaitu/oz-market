# Quality Rules - Delete Legacy Benchmarks

- **Zero Orphan Configuration**: Cargo.toml must be completely clean of any target config, reference, or feature flags tied to deleted files.
- **Clean Workspace Compile**: Deleting files must not break core library checks, tests, or other valid workspace binaries.
- **No Shared Helper Deletions**: Ensure that database migration files, database helpers, or shared config structs (like `populate_db.rs`) are not inadvertently deleted.
