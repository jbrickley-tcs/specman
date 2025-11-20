# Specification Manager

Specification Manager is a CLI tool that helps AI agents and teams create, read, and follow clear, code-friendly specifications. It favors concise, implementable constraints over task-tracking or corporate jargon.

## Dependency Mapping

The `specman` Rust crate (under `src/crates/specman`) now ships `FilesystemDependencyMapper`, a workspace-aware traversal engine that:
- Normalizes filesystem/HTTPS locators through `WorkspaceLocator` before parsing Markdown front matter.
- Builds upstream, downstream, and aggregate `DependencyTree` views while annotating artifacts whose metadata was inferred from paths.
- Detects cycles and workspace boundary violations early, returning `SpecmanError` diagnostics that lifecycle tooling can surface directly.

## Testing

Run repository tests from the `src` directory to exercise filesystem + HTTPS traversal paths, cycle detection, and metadata fallbacks:

```bash
cd src
cargo test -p specman
```

CI should also run `cargo fmt` and `cargo clippy` to keep formatting and lint gates aligned with Rust 1.91.

