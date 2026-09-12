# Plan: rust-conversion (from intent.md 2026-09-12)

## Files that change
- `Cargo.toml` — new crate `gh-axi` (bin `gh-axi`), serde/serde_json deps, release profile.
- `src/lib.rs` — library root exposing public modules for integration tests.
- `src/bin/gh-axi.rs` — command-first dispatch + top-level help (mirrors `src/cli.ts`).
- `src/toon.rs` — TOON renderer (header/list/kv/help/error/join).
- `src/error.rs` — `AxiError` + 0/1/2 exit-code mapping.
- `src/version.rs` — `version [--yes]` + `update` (#368, releases/latest feed).
- `src/args.rs` — arg-walk helpers (getFlag/getAllFlags/rejectUnknownFlags equivalents).
- `src/commands/*.rs` — one module per verb family (issue, pr, run, workflow, release, repo, label, gist, project, secret, variable, search, api, setup, stack) + `home.rs`.
- `tests/*.rs` — integration + unit parity with `test/*.test.ts`.
- `intent/8-rust-conversion/` — this chain.

## Order of work
1. Open the chain (done) + declare scope/command surface (this spec).
2. Scaffold the crate + shared infra (toon/error/args) + `version [--yes]` (#368).
3. Port verbs in dependency order: context/gh exec → issue/pr (core) → run/workflow/release → repo/label/gist → project/secret/variable → search/api/setup → stack.
4. Port `--help` strings verbatim per command; wire repeatable-flag and conflict guards.
5. Tests parity (help-examples, errors, secretValue, stdin, version-fast-path, cli dispatch).
6. Flip CI to cargo; cut first release (distribution config).

## Risks
- **Behavior drift** — the 1:1 output/exit-code contract is the whole point; every verb must be diffed against the Node CLI's output, not re-derived from memory.
- **gh passthrough subtleties** — `--attach` version floor, `--env` scope rejection, secret stdin-only, stack cwd-bound rejection, repeatable flags (#55/#57/#75) must survive the port.
- **Empty release feed** — `version --yes` cannot be exercised end-to-end until a Rust release exists.

## Proof
- `cargo build` + `cargo test` green.
- `gh-axi --help` and every `<command> --help` render identically to the Node CLI.
- `gh-axi version` prints `gh-axi 0.1.35`; `version --yes`/`update` key off `releases/latest` (regression-guarded like cf #339).
- A representative verb (e.g. `issue list`) produces byte-identical TOON against a live repo.
