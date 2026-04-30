# Rules to Follow for Agents and Human

## Rust Code Quality

### File Size
- Each source file must be under **1000 lines of code** including comments and blank lines.
- If a file grows beyond this limit, split it into focused submodules.
- Try to minimize the exposed interface to only the minimal necessary scope on newly added code, unless explicitly specified. 

### Function Length
- Keep functions **short and focused** — ideally under 25 lines, unless needed for one-off pattern matching or chained calls.
- Extract helper functions rather than writing deeply nested or long bodies.

### Preferences
- Always prefer desugared functions over async fn in trait definitions.
- Always prefer async fn over functions returning Future everywhere except traits.
- Always prefer the use of super when importing from direct parent module's scope.
- Always prefer the use of first mod module, then pub use mod::* after the modules style when doing reexports.
- Always prefer the use of wildcard reexports and control visibility range in modules. 

### Deduplication
- Prefer generics over macro_rules.
- Snippets repeated over two times and is over 15 lines should be extracted as helper functions.

### Naming
- Use idiomatic Rust naming: `snake_case` for functions/variables, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Prefer descriptive names over abbreviations.

### Error Handling
- Use `Result` and `?` for propagation; avoid `.unwrap()` in library code.
- Define clear error types per crate when appropriate.

### Safety
- Minimize `unsafe` code. Isolate it in small, well-documented helper functions.
- Every `unsafe` block must have a `// SAFETY:` comment explaining the invariants.

### Documentation
- Public items must have doc comments (`///`) in rust format.
- Private items must have comments for the purpose of the item unless its name is descriptive enough.
- Module-level documentation (`//!`) should describe the module purpose.

### INDEX.md
- Place one under each crate directory to describe the purpose of the crate and its dependencies.
- Place one under each directory module within and including src describing a file architecture and public interface.
- Keep them updated after every edit to files in related directory and crate.
- Use them to guide the navigation to functionalities.

### Testing
- Write unit tests in `#[cfg(test)]` modules.
- Integration tests go in `tests/` directories.
- Name test harness/mock types with prefix Mock, helper functions with prefix assist_, and cases with prefix test_.
- Unit tests must cover both defined success and failure path, but must not cover explicitly undefined code path.
- Unit tests must only test one variance at once in a single test case, and must do precise state or result inspection.
- Unit tests cases must assume all other non-tested variances to be correct.
- Unit tests cases must not have redundant coverage.
- Integration tests must assume the implementation is fully correct.
- Integration tests must test only end-to-end integration of the crate public interface or serve as integration example.
- Regression prevention integration tests must be put under the regression target.

### Formatting and Linting
- Run `cargo fmt` before committing.
- Run `cargo clippy` and address warnings.

### Version management
- Commit should be made on every unit of progression.
- Commit name should be brief description of progression.
- All commits should pass compilation.

## Dependency Management

### Workspace Dependencies
- All shared dependencies **must** be declared in the workspace `Cargo.toml` under `[workspace.dependencies]`.
- Member crates reference workspace dependencies with `{ workspace = true }`.
- This ensures consistent versions across the entire workspace.

### Adding Dependencies
- Prefer well-maintained, widely-used crates.
- Pin major versions (e.g., `"1"` not `"*"`).
- Check for known advisories before adding a new dependency.
- Keep the dependency tree small — avoid pulling in large transitive dependencies for minor functionality.

### Crate Organization
- ref for workspace reference, doc for workspace documents. these are not components.
- Each workspace member is its own crate with a clear, singular purpose.
- Use `lib.rs` as the crate root; keep it focused on re-exports and module declarations.

## Agentic Work (for agents only)
- Always delegate planning work to subagents.
- Always delegate exploring work to subagents.
- Always delegate to subagents when reading mass amount of files to find specific information.
- Always delegate to subagents when dealing with repetitive tool calls for trivial task.
- When in doubt or having multiple pathways to choose, ask the user. use ask_human tool if present.
- Do not perform git commits unless you are explicitly requested to do so by the user.
- You are always allowed to revert current changes using checkout.
- Check the know issues document list first to see if any issue you found is not to fix now for what reason.
- If the user asks to postpone an issue fix, file it under the known issues document.