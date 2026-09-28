# Physical design: implemented architecture

The module extraction and workspace/crate split are complete. Feature crates live under `crates/`, with app wiring at the top and shared vocabulary below. This document retains the rules behind that restructuring; the historical migration steps are in git history.

Unrelated concerns belong in separate files. Module directories should reflect ownership, and dependencies point downward. Mechanical guards in `tests/physical_design.rs` keep new file-size violations and top-level dependency cycles visible.

## Rules that keep it fixed

1. **File budget: ~1000 lines of non-test code.** Not a hard wall — a
   cohesive file just over it beats two incoherent halves — but crossing
   it in a PR needs a sentence of justification.
2. **Tests move to sibling files once they dominate.** Any `#[cfg(test)]
   mod tests` over ~150 lines becomes `#[cfg(test)] mod tests;` resolving
   to `<module>/tests.rs` (or `tests_<topic>.rs` split by subject). Same
   crate, same visibility, zero test-code changes — pure relocation.
3. **`mod.rs` contains wiring only**: `mod` declarations, the plugin, and
   `pub use` re-exports. Logic in a `mod.rs` is the seed of every god file
   in the codebase.
4. **A file states its contents in a `//!` header** (most already do —
   make it universal). First line = what's in the file; that's what an
   assistant's grep-then-skim lands on.
5. **New concern → new file; second concern in a file → split it then.**
   Boy-scout rule: the person touching a file that violates the budget
   splits it *first*, in its own commit, before the feature change.
6. **Enforce mechanically:** `tests/physical_design.rs` checks the module graph and file budget.
   - **Size** — fails on any file whose non-test line count exceeds the
     budget, with an explicit allowlist for existing offenders. New violations cannot land silently.
   - **Acyclicity** — `no_module_dependency_cycles` builds the top-level
     module graph from every `use crate::X` (comments stripped: several doc
     comments name modules they don't import) and fails on any strongly
     connected component. **No allowlist here, deliberately** — the size
     budget burns down gradually, but the graph started clean and has to
     stay clean, so there is no exemption mechanism to erode.
