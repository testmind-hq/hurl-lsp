# VS Code Extension Optimization Roadmap

## Objective

Evolve Hurl LSP from a language-aware request editor into a reliable daily HTTP workflow: transparent execution, selectable environments, trustworthy results, richer language intelligence, and production-grade distribution.

## Priorities

### P0: Execution task management

- Represent a run as a uniquely identified task with `queued`, `running`, `cancelling`, `succeeded`, `failed`, `cancelled`, and `timedOut` states.
- Show live running state and elapsed time in CodeLens and Hurl Inspector.
- Allow cancellation from CodeLens, the command palette, and Inspector.
- Add configurable process timeout and terminate the Hurl child process on timeout.
- Prevent stale task notifications from overwriting the selected document or a newer run.
- Report preparation, process startup/overhead, HTTP, and report parsing timing separately.
- Stream bounded stdout/stderr updates while a task runs.

### P0: Environment Profiles

- Support named profiles such as Local, Test, Staging, and Production.
- Allow profiles to declare one or more variable files, in deterministic order.
- Keep automatic variable-file discovery as the zero-configuration default profile.
- Add a status-bar selector scoped per workspace folder.
- Show the active profile and resolved variable sources in Inspector.
- Apply the same profile to diagnostics, hover, completion, inlay hints, cURL generation, Run with vars, Run Chain, and Run File.
- Never expose sensitive variable values in UI, logs, or notifications.

### P1: End-to-end reliability

- Add VS Code extension-host tests for editor selection, document versions, completion, inlay hints, execution, cancellation, profile changes, and Inspector synchronization.
- Add regression fixtures for Unicode, malformed variable braces, multiple requests, POST bodies, redirects, binary bodies, and failed assertions.

### P1: Inspector experience

- Add Raw/Pretty modes, search, line numbers, JSON folding, and response-save actions.
- Support full request/response copying and previews for JSON, XML, HTML, text, and images.
- Add run comparison and request/response body diff.
- Render timing as a waterfall and link failed assertions back to source.

### P1: Language intelligence

- Add semantic tokens and deeper contextual completion for Hurl sections, queries, filters, and predicates.
- Add diagnostics and quick fixes for malformed `{{variables}}`, unused captures, cyclic dependencies, and unreachable chain steps.
- Add Rename Symbol, Find References, and document links for URLs and referenced files.

### P2: Performance and maintainability

- Cache parsed variable and OpenAPI files by path and modification time; invalidate them with file watchers.
- Cancel obsolete completion, hover, and diagnostic work.
- Split the large language-server backend into document, workspace, runner, command, and diagnostic services.
- Bundle the VS Code extension and tighten `.vscodeignore` to reduce VSIX file count and activation cost.

### P2: Distribution hardening

- Verify downloaded release binaries against `SHA256SUMS`.
- Download to a temporary file and atomically install only after verification.
- Add timeout, redirect limits, cleanup, retry, progress reporting, and proxy-aware errors.
- Add Linux ARM64 artifacts and automated installation tests for every supported platform.

## Recommended delivery sequence

1. Execution task protocol, cancellation, timeout, live state, and phase timing.
2. Environment Profile configuration, resolver, selector, and consistent consumption.
3. VS Code extension-host tests for both features.
4. Inspector waterfall, output streaming, and profile/source display.
5. Variable/OpenAPI caching and backend decomposition.
6. Download verification, Linux ARM64, and VSIX bundling.

## Current milestone

The next development milestone implements **Execution Task Management + Environment Profiles**. It preserves automatic variable discovery, current Run commands, wire compatibility for completed results, and secret masking while adding explicit task lifecycle and profile selection.
