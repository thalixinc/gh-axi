//! CLI dispatch integration tests, mirroring the Node `test/cli.test.ts` surface.

use gh_axi::cli::{run_with, TOP_HELP, UPDATE_HELP, VERSION_HELP};

fn no_releases() -> gh_axi::error::Result<Option<String>> {
    Ok(None)
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

#[test]
fn bare_version_flag_prints_version_line() {
    for flag in ["-v", "-V", "--version"] {
        let out = run_with(&argv(&[flag]), no_releases);
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.output, "gh-axi 0.1.35\n");
    }
}

#[test]
fn top_help_matches_spec() {
    let out = run_with(&argv(&["--help"]), no_releases);
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.output, TOP_HELP);
}

#[test]
fn version_command_prints_version_line_only_when_no_release() {
    let out = run_with(&argv(&["version"]), no_releases);
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.output, "gh-axi 0.1.35\n");
}

#[test]
fn version_help_matches_spec() {
    let out = run_with(&argv(&["version", "--help"]), no_releases);
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.output, VERSION_HELP);
}

#[test]
fn update_help_matches_spec() {
    let out = run_with(&argv(&["update", "--help"]), no_releases);
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.output, UPDATE_HELP);
}

#[test]
fn update_check_reports_no_release() {
    let out = run_with(&argv(&["update", "--check"]), no_releases);
    assert_eq!(out.exit_code, 0);
    assert_eq!(
        out.output,
        "update:\n  package: gh-axi\n  current: 0.1.35\n  latest: (none published)\n  available: false\n"
    );
}

#[test]
fn unknown_command_is_validation_error() {
    let out = run_with(&argv(&["bogus"]), no_releases);
    assert_eq!(out.exit_code, 2);
    assert_eq!(
        out.output,
        "error: \"Unknown command: bogus\"\ncode: VALIDATION_ERROR\nhelp[1]: Run `--help` to see available commands\n"
    );
}

#[test]
fn leading_flag_is_rejected() {
    let out = run_with(&argv(&["-h"]), no_releases);
    assert_eq!(out.exit_code, 2);
    assert!(out.output.contains("Flags must come after the command"));
}

#[test]
fn unported_verb_is_honest_operational_error() {
    let out = run_with(&argv(&["issue"]), no_releases);
    assert_eq!(out.exit_code, 1);
    assert!(out.output.contains("not yet available in the Rust port"));
    assert!(out.output.contains("NOT_PORTED"));
}

#[test]
fn dashboard_is_not_yet_ported() {
    let out = run_with(&[], no_releases);
    assert_eq!(out.exit_code, 1);
    assert!(out.output.contains("NOT_PORTED"));
}
