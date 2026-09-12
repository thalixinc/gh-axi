//! Structured error type + gh-stderr classification (AXI family convention),
//! mirroring `src/errors.ts`.
//!
//! Exit codes: 0 = success, 1 = operational failure, 2 = usage / validation.

use std::fmt;

/// A structured CLI error carrying a message, `code`, and optional `help` suggestions.
#[derive(Debug, Clone)]
pub struct AxiError {
    pub message: String,
    pub code: &'static str,
    pub suggestions: Vec<String>,
}

impl AxiError {
    /// Operational failure (exit 1 unless the code is `VALIDATION_ERROR`).
    pub fn operational(message: impl Into<String>, code: &'static str) -> Self {
        Self {
            message: message.into(),
            code,
            suggestions: Vec::new(),
        }
    }

    /// Usage / validation failure (exit 2).
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: "VALIDATION_ERROR",
            suggestions: Vec::new(),
        }
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// 2 for validation/usage, 1 otherwise.
    pub fn exit_code(&self) -> i32 {
        if self.code == "VALIDATION_ERROR" {
            2
        } else {
            1
        }
    }
}

impl fmt::Display for AxiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AxiError {}

pub type Result<T, E = AxiError> = std::result::Result<T, E>;

fn first_error_line(stderr: &str) -> String {
    stderr.trim().split('\n').next().unwrap_or("").to_string()
}

/// `gh CLI is not installed` — the default missing-binary error.
pub fn gh_not_installed_error() -> AxiError {
    AxiError::operational(
        "gh CLI is not installed — see https://cli.github.com",
        "GH_NOT_INSTALLED",
    )
}

/// Match a regex (compiled on demand; gh errors are rare) and return captures.
fn cap<'h>(re: &str, haystack: &'h str) -> Option<regex::Captures<'h>> {
    regex::Regex::new(re).ok()?.captures(haystack)
}

/// Classify gh stderr into a structured `AxiError`, mirroring `mapGhError`.
/// Pattern order is the contract: a narrow pattern must sit ahead of any broader
/// one it would otherwise be swallowed by.
pub fn map_gh_error(stderr: &str, exit_code: i32) -> AxiError {
    if let Some(m) = cap(
        r"Could not resolve to a Repository with the name '([^']+)'",
        stderr,
    ) {
        return AxiError::operational(
            format!("Repository \"{}\" not found", &m[1]),
            "REPO_NOT_FOUND",
        )
        .with_suggestions(vec![
            "Run `gh-axi repo list` to see your repositories".into()
        ]);
    }
    if let Some(m) = cap(
        r"Could not resolve to an? .+? with the number of (\d+)",
        stderr,
    ) {
        return AxiError::operational(
            format!("Item #{} does not exist in this repository", &m[1]),
            "NOT_FOUND",
        );
    }
    if let Some(m) = cap(r"(?i)issue (\d+) not found", stderr) {
        return AxiError::operational(format!("Issue #{} does not exist", &m[1]), "NOT_FOUND");
    }
    if let Some(m) = cap(r"(?i)pull request (\d+) not found", stderr) {
        return AxiError::operational(
            format!("Pull request #{} does not exist", &m[1]),
            "NOT_FOUND",
        );
    }
    if let Some(m) = cap(r#"(?i)release with tag "([^"]+)" not found"#, stderr) {
        return AxiError::operational(format!("Release \"{}\" not found", &m[1]), "NOT_FOUND")
            .with_suggestions(vec![
                "Run `gh-axi release list` to see available releases".into()
            ]);
    }
    if let Some(m) = cap(r"(?i)run (\d+) not found", stderr) {
        return AxiError::operational(format!("Run {} not found", &m[1]), "NOT_FOUND")
            .with_suggestions(vec!["Run `gh-axi run list` to see recent runs".into()]);
    }
    // gh tacks a `gh auth login` hint onto repo-resolution failure; this must sit
    // ahead of the generic `gh auth login` pattern below.
    if cap(
        r"(?i)none of the git remotes configured for this repository point to a known GitHub host",
        stderr,
    )
    .is_some()
    {
        return AxiError::usage(
            "Could not determine the target repository from this checkout's git remotes",
        )
        .with_suggestions(vec![
            "Pass the repo explicitly: `-R <owner>/<name>` (after the command)".into(),
            "For a GitHub Enterprise host, add `--hostname <host>` or set GH_HOST".into(),
        ]);
    }
    if cap(r"gh auth login", stderr).is_some() {
        return AxiError::operational(
            "GitHub auth required — run `gh auth login` first",
            "AUTH_REQUIRED",
        );
    }
    if let Some(m) = cap(
        r"(?i)authentication token is missing required scopes \[([^\]]+)\]",
        stderr,
    ) {
        return AxiError::operational(
            format!("GitHub token is missing required scope(s): {}", &m[1]),
            "FORBIDDEN",
        )
        .with_suggestions(vec![
            format!(
                "Run `gh auth refresh -s {}` to grant the required scope",
                &m[1]
            ),
            "Then verify with `gh auth status`".into(),
        ]);
    }
    if cap(r"(?i)secondary rate limit", stderr).is_some() {
        return AxiError::operational(
            "GitHub secondary rate limit hit — wait ~60s and retry",
            "RATE_LIMITED",
        )
        .with_suggestions(vec![
            "Wait 60s before retrying".into(),
            "Use `gh api` (REST) for read-only ops, which has a separate budget".into(),
        ]);
    }
    if cap(r"(?i)API rate limit (?:already )?exceeded", stderr).is_some() {
        return AxiError::operational("GitHub API rate limit exceeded", "RATE_LIMITED")
            .with_suggestions(vec![
                "Wait until the hourly window resets (run `gh api rate_limit` to check)".into(),
                "Use a different identity with `gh auth switch` if available".into(),
            ]);
    }
    if let Some(m) = cap(
        r"(?i)sub-issue is already a sub-issue of issue with number (\d+)",
        stderr,
    ) {
        return AxiError::usage(format!("Issue is already a sub-issue of #{}", &m[1]));
    }
    if cap(r"(?i)sub-?issue.*?(cycle|circular)", stderr).is_some() {
        return AxiError::usage("Cannot add sub-issue: would create a cycle");
    }
    if cap(r"(?i)issue cannot be a sub-?issue of itself", stderr).is_some() {
        return AxiError::usage("An issue cannot be a sub-issue of itself");
    }
    if cap(r"unknown flag: --attach", stderr).is_some() {
        return AxiError::usage(
            "--attach requires gh >= 2.99.0. Upgrade gh, or set GH_BIN to a 2.99.0+ binary",
        )
        .with_suggestions(vec![
            "Install GitHub CLI 2.99.0 or newer from https://github.com/cli/cli/releases".into(),
            "Or point GH_BIN at a 2.99.0+ gh binary".into(),
        ]);
    }
    if cap(r"`--attach` accepts at most 50 values per command", stderr).is_some() {
        return AxiError::usage("--attach accepts at most 50 values per command");
    }
    if let Some(m) = cap(r"`--attach` is not supported when using (`--\S+`)", stderr) {
        return AxiError::usage(format!("--attach cannot be combined with {}", &m[1]));
    }
    if let Some(m) = cap(
        r"(?im)^could not upload ([^\n]+)\nattaching files requires write access to the repository$",
        stderr,
    ) {
        return AxiError::operational(
            format!(
                "Could not upload {}: attaching files requires write access to the repository",
                &m[1]
            ),
            "FORBIDDEN",
        );
    }
    if cap(r"cannot set alt text on video", stderr).is_some() {
        return AxiError::usage("--attach: cannot set alt text on video");
    }
    if let Some(m) = cap(
        r"(?m)^(.+?) is not a supported file type \(supported: ([^)]+)\)$",
        stderr,
    ) {
        return AxiError::usage(format!(
            "--attach {} is not a supported file type (supported: {})",
            &m[1], &m[2]
        ));
    }
    if let Some(m) = cap(r"(?m)^(.+?): images must be at most ([^\n]+)$", stderr) {
        return AxiError::usage(format!(
            "--attach {}: images must be at most {}",
            &m[1],
            m[2].trim()
        ));
    }
    if let Some(m) = cap(r"(?m)^(.+?): videos must be at most ([^\n]+)$", stderr) {
        return AxiError::usage(format!(
            "--attach {}: videos must be at most {}",
            &m[1],
            m[2].trim()
        ));
    }
    if cap(
        r"(?i)attach(?:ment)? uploads? (?:are|is) not (?:supported|available).*(?:Enterprise Server|GHES)",
        stderr,
    )
    .is_some()
    {
        return AxiError::usage(
            "--attach is not supported on GitHub Enterprise Server in this gh release",
        );
    }
    if cap(r"HTTP 403", stderr).is_some() {
        return AxiError::operational("Insufficient permissions for this action", "FORBIDDEN");
    }
    if cap(r"HTTP 422", stderr).is_some() {
        let msg = cap(r#""message"\s*:\s*"([^"]+)""#, stderr)
            .map(|m| m[1].to_string())
            .unwrap_or_else(|| "Validation error".to_string());
        return AxiError::usage(msg);
    }

    // Generic not-found for any 404-like message.
    if cap(r"(?i)not found", stderr).is_some() {
        return AxiError::operational(first_error_line(stderr), "NOT_FOUND");
    }

    let line = first_error_line(stderr);
    AxiError::operational(
        if line.is_empty() {
            format!("gh exited with code {exit_code}")
        } else {
            line
        },
        "UNKNOWN",
    )
}

/// Map a clap parse error to the Node CLI's `rejectUnknownFlags` shape.
/// `sub` is the already-resolved subcommand (the verb's first positional).
pub fn map_clap_error(e: clap::Error, command: &str, sub: &str) -> AxiError {
    let token = extract_single_quoted(&e.to_string()).unwrap_or_default();
    match e.kind() {
        clap::error::ErrorKind::UnknownArgument => {
            let flag = token.split('=').next().unwrap_or(&token).to_string();
            AxiError::usage(format!("unknown flag for gh-axi {command} {sub}: {flag}"))
                .with_suggestions(vec![
                    format!("gh-axi {command} {sub} [flags]"),
                    format!("gh-axi {command} {sub} --help"),
                ])
        }
        _ => AxiError::usage(
            e.to_string()
                .lines()
                .next()
                .unwrap_or("invalid arguments")
                .trim()
                .to_string(),
        ),
    }
}

fn extract_single_quoted(s: &str) -> Option<String> {
    regex::Regex::new(r"'([^']+)'")
        .ok()?
        .captures(s)
        .map(|c| c[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_repo_not_found() {
        let e = map_gh_error(
            "Could not resolve to a Repository with the name 'foo/bar'",
            1,
        );
        assert_eq!(e.code, "REPO_NOT_FOUND");
        assert_eq!(e.message, "Repository \"foo/bar\" not found");
    }

    #[test]
    fn maps_auth_required() {
        let e = map_gh_error("run gh auth login to authenticate", 1);
        assert_eq!(e.code, "AUTH_REQUIRED");
    }

    #[test]
    fn maps_forbidden_scopes() {
        let e = map_gh_error(
            "authentication token is missing required scopes [read:project]",
            1,
        );
        assert_eq!(e.code, "FORBIDDEN");
        assert!(e.message.contains("read:project"));
    }

    #[test]
    fn maps_rate_limited() {
        let e = map_gh_error("secondary rate limit exceeded", 1);
        assert_eq!(e.code, "RATE_LIMITED");
    }

    #[test]
    fn maps_issue_not_found() {
        let e = map_gh_error("issue 42 not found", 1);
        assert_eq!(e.code, "NOT_FOUND");
        assert_eq!(e.message, "Issue #42 does not exist");
    }

    #[test]
    fn falls_back_to_unknown_with_exit_code() {
        let e = map_gh_error("", 3);
        assert_eq!(e.code, "UNKNOWN");
        assert_eq!(e.message, "gh exited with code 3");
    }

    #[test]
    fn validation_error_exits_2() {
        let e = map_gh_error("unknown flag: --attach", 1);
        assert_eq!(e.exit_code(), 2);
    }
}
