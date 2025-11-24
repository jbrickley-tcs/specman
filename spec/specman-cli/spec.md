---
name: specman-cli
version: "1.0.0"
dependencies:
  - ../specman-core/spec.md
  - ../specman-data-model/spec.md
  - ../specman-templates/spec.md
---

# Specification — SpecMan CLI

The SpecMan CLI defines a command-line binary that orchestrates SpecMan Core capabilities through declarative commands. It standardizes how operators trigger workspace discovery, artifact creation, and safe deletions while remaining agnostic to distribution or PATH management concerns.

## Terminology & References

This document uses the normative keywords defined in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119). Consumers SHOULD review `specman-core`, `specman-data-model`, and `specman-templates` to understand the lifecycle rules, data entities, and template contracts enforced by this CLI.

## Concepts

### Concept: CLI Invocation Model

- The CLI MUST be executable as a standalone binary; installation, PATH configuration, or shell-integration steps are explicitly out of scope for this specification.
- Every command MUST provide structured stdout/stderr suitable for automation, and SHOULD exit with non-zero codes on validation failures so scripts can detect errors deterministically.
- Commands MUST accept positional arguments and flags that can be scripted without interactive prompts; optional interactive flows MAY exist but MUST have equivalent flag-driven variants.
- The CLI MUST emit human-readable help text describing each command, argument, and related specification reference, and this help text MUST be accessible via `--help` for every command level.

### Concept: Workspace Context Resolution

- On startup, the CLI MUST discover the active SpecMan workspace by scanning the current working directory and its ancestors for the nearest `.specman` folder, mirroring the `Workspace Discovery` concept defined by `specman-core`.
- Callers MAY provide an explicit `--workspace` flag (or environment variable) to override the search path; the CLI MUST validate that the supplied path contains a `.specman` directory and MUST fall back to nearest-ancestor detection when the override is absent or invalid.
- Workspace resolution MUST surface both the workspace root and the `.specman` directory paths to downstream subsystems without recomputing filesystem state per command.
- If no `.specman` folder is found, the CLI MUST fail fast with an actionable error message that includes the search path that was attempted.

### Concept: Lifecycle Command Surface

- The CLI MUST expose create commands for specifications, implementations, and scratch pads, each of which MUST enforce the naming rules defined in the `specman-data-model` and founding specifications.
- Creation commands MUST invoke the dependency mapping and template orchestration behaviors defined by `specman-core`, ensuring that generated artifacts include compliant front matter and section scaffolding.
- Delete commands MUST refuse to proceed when dependency analysis reveals downstream consumers unless the operator explicitly supplies `--force`; forced deletions MUST still print the blocking dependency tree, require explicit confirmation (flag or prompt), and MUST record in the command result that dependencies were overridden.
- All lifecycle commands MUST persist results to the canonical workspace paths (`spec/`, `impl/`, `.specman/scratchpad/`) returned by workspace discovery, and MUST error when filesystem writes fail.

### Concept: Data Model Activation

- The CLI MUST bundle a SpecMan data-model implementation (adapter) as an internal library so every installation has a compliant default aligned with the major version of `specman-data-model` declared in this specification.
- Workspaces MAY reference alternative adapters (for example, via `.specman/config`), but the CLI MUST fall back to the bundled adapter whenever no override is provided or the override fails validation.
- CLI commands MUST serialize entities exactly as defined in the data model before persisting or emitting them, and MUST surface validation errors from the adapter verbatim to the caller.
- When both the bundled adapter and any configured override are unavailable or incompatible, the CLI MUST fail the command with guidance describing how to remediate the adapter configuration.

### Concept: Template Integration & Token Handling

- Creation commands MUST load the appropriate Markdown template from `templates/spec/`, `templates/impl/`, or `templates/scratch/` (or workspace overrides) before rendering artifacts.
- The CLI MUST require callers to supply every declared `{{token}}` before rendering; missing tokens MUST result in descriptive errors that reference the originating template and token name.
- Template rendering MUST respect HTML comment directives embedded in templates and MUST only remove a directive after its instruction has been satisfied or explicitly recorded in the generated artifact.
- The CLI SHOULD cache template metadata (required tokens, scenario type) for the duration of a command invocation to avoid redundant filesystem reads, but MUST NOT cache it across workspaces unless the template version is part of the cache key.

### Concept: Observability & Error Surfacing

- Each CLI command SHOULD emit structured logs (for example JSON lines) when `--verbose` or `--json` flags are supplied, capturing workspace paths, template locators, and adapter identifiers used during execution.
- Error messages MUST reference the specification section (Concept or Entity) that mandated the failed behavior whenever possible, enabling downstream tooling to triage issues quickly.

## Key Entities

### Entity: CliSession

- Represents a single CLI invocation, including parsed flags, environment overrides, and references to the data-model adapter.
- MUST capture the workspace context, resolved template catalog, and logging preferences for downstream components.
- SHOULD expose helpers to format consistent success/error payloads.

### Entity: WorkspaceContext

- Encapsulates the workspace root, `.specman` directory, detected templates directory, and adapter configuration for the active invocation.
- MUST be derived from the Workspace Context Resolution concept and reused across all subcommands invoked within the same process.
- MAY cache derived paths (spec, impl, scratchpad roots) for efficiency.

### Entity: LifecycleRequest

- Describes a create or delete operation, including artifact type, target name, template locator, dependency tree, and requested flags (`--force`, `--json`, etc.).
- MUST validate names against the data-model naming constraints before dispatching to the adapter.
- SHOULD record rendered template output (for create) or dependency trees (for delete) to support auditing.

### Entity: DeletionPlan

- Captures the dependency analysis for a delete request, including upstream/downstream relationships, whether deletion is permitted, and any required confirmations.
- MUST be produced before any filesystem mutation occurs.
- MUST reference SpecMan Core dependency mapping outputs and annotate whether the current request respects or overrides blocking dependents.

### Entity: TemplateRenderPlan

- Contains the template locator, token map, resolved output path, and any post-processing steps (such as removing satisfied HTML comments).
- MUST ensure every required token is supplied, and MUST record whether default values were injected.
- SHOULD expose dry-run output for tooling that wants to preview generated artifacts.

## Constraints

- Commands MUST be deterministic: identical inputs (workspace, flags, templates) MUST yield identical outputs aside from timestamps or IDs explicitly documented as variable.
- The CLI MUST reject simultaneous create and delete requests within a single invocation to avoid partial state mutations; batching MUST run operations sequentially.
- Configuration files under `.specman/` MAY supply defaults (such as adapter identifiers or template overrides), but command-line flags MUST take precedence.
- The CLI MUST guard against executing outside the detected workspace by refusing to read or write files that resolve beyond the workspace root.
- Extensions or plugins MUST NOT bypass dependency checks or naming validations defined by this specification.

## Additional Notes

- Distribution, install scripts, and binary naming conventions are intentionally unspecified; downstream teams MAY package the CLI for their ecosystems as long as runtime semantics remain compliant.
- Future versions MAY introduce additional command groups (for example, validation or status) provided they reuse the concepts and entities defined here.
- Implementations MAY integrate with credential stores or secrets managers to access remote template catalogs, but such integrations MUST continue to respect the template governance defined in `specman-templates`.
- Persistent audit logging is out of scope for this version because the CLI does not prescribe a storage location for historical records.
