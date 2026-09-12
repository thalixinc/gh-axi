# Intent: rust-conversion
Author: manateeit. Status: draft.
Issue: #8. Epic: #8.

## Problem
gh-axi is the only thalixinc-owned AXI tool still on Node/npm (package.json, `runAxiCli` from `axi-sdk-js`). Every sibling (cf, cf-queue, cmux-axi, sdlc-axi, brainstorm-axi, sbox-axi, herdr-axi) is Rust/cargo. Two runtimes, two install paths, two version feeds — the drift tax is real and growing.

## Proposed outcome
One Rust crate, `gh-axi`, distributed via `cargo install --git https://github.com/thalixinc/gh-axi`. Identical CLI surface (17 commands, flags, TOON output), the shared `version [--yes]` update convention (#368), and the Node/axi-sdk-js dependency dropped.

## Affected users and systems
- Every agent/operator that runs `gh-axi` (the `gh` wrapper surface they depend on is preserved byte-for-byte).
- The release path: npm publish (release-please) → cargo release/git install.
- CI workflows that build/test the CLI (Node toolchain → cargo).
- The installable skill (`skills/gh-axi`) — regenerated to point at the cargo install path.

## Constraints
- Observable behavior preserved 1:1 — no new verbs/behavior beyond a faithful port (non-goal).
- TOON output format, exit codes (0/1/2), and the `-R`/`--hostname`/`GH_HOST` context resolution must be identical.
- Secret values must never appear in argv or stdout (carry over the stdin-only `secret set` contract).
- `version --yes` must degrade to the bare version line on a fetch failure, never hard-error (cf #378).

## Open questions
- Ticket says "port to clap"; the AXI family (cf, cf-queue, cmux-axi, sdlc-axi, brainstorm-axi, herdr-axi) hand-rolls its parser. Which is authoritative? (Leans hand-rolled: "identical CLI UX" is only achievable outside clap's help/error renderer.)
- Version feed: gh-axi has no GitHub Releases today (only npm tags `gh-axi-v0.1.35`). When is the first cargo release cut, and what tag shape (`v0.1.35` vs `gh-axi-v0.1.35`)?
- Crate version: start at `0.1.35` (continuity with npm) or reset to `0.1.0`?
- npm package retirement timeline (name mapping is explicitly a follow-on distribution question).
