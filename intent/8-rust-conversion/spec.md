# Spec: rust-conversion

From: intent.md. Author: manateeit. Status: draft.
Issue: #8. Epic: #8.

## Requirements
- **R1 — CLI surface 1:1.** The 17 commands (`(none)` dashboard, issue, pr, stack, run, workflow, release, repo, label, gist, project, secret, variable, search, api, setup, version) with every subcommand, flag, repeatable flag, and TOON output shape, exactly as `gh-axi --help` renders today. `gh-axi --help` is the spec.
- **R2 — version/--yes (#368).** `gh-axi version [--yes]` prints `gh-axi <version>`, reports `update available` from the release feed, auto-updates on `--yes`, prompts `[y/N]` on a TTY, and never blocks/reports-only on a non-TTY. Fetch failure degrades to the bare version line (exit 0).
- **R3 — cargo distribution.** Replace npm install/publish with `cargo build` + `cargo install --git`/release. Crate name/binary `gh-axi`.
- **R4 — context resolution.** `-R`/`--repo`, `--hostname`/`GH_HOST`, and the gh-child `GH_HOST` inheritance behave identically (after-command flags, space/equals forms, stack's cwd-bound rejection).
- **R5 — secret contract.** `secret set` is stdin-only; values never in argv/stdout.

## Design
- **Crate layout** mirrors the family: `src/lib.rs` (public modules) + `src/bin/gh-axi.rs` (dispatch), `src/toon.rs` (TOON renderer), `src/error.rs` (structured `AxiError` + 0/1/2 exit codes), `src/version.rs` (#368), plus per-verb modules under `src/commands/`.
- **Parser:** hand-rolled arg walk (the `Parsed` pattern in `sdlc-axi`/`cf`), NOT clap — see Areas of concern. This preserves the exact TOON help/error strings.
- **Version feed:** `gh api repos/thalixinc/gh-axi/releases/latest --jq .tag_name` (canonical #368, releases/latest never the tags feed); update via `cargo install --git https://github.com/thalixinc/gh-axi --tag v<latest> --force`.
- **Deps:** `serde`/`serde_json` only (family convention); `gh`/`cargo`/`curl` invoked via `std::process::Command`.
- **Coexistence:** the Rust crate lands alongside the Node source; Node remains until verbs are ported and CI flips.

## Areas of concern
- **clap vs hand-rolled parser.** R1 requires "identical CLI UX"; clap cannot render the Node custom TOON help/error. All AXI siblings hand-roll. Resolution: hand-rolled (family convention); the ticket's "clap" is treated as shorthand for "Rust CLI" pending originator confirmation.
- **Empty release feed.** No GitHub Releases exist yet; `version --yes` degrades gracefully (no update available) until the first cargo release — release config is a separate distribution task.
- **Crate version + tag shape.** Version/tag naming is unresolved (see intent Open questions); first increment starts at `0.1.35` for output continuity.
