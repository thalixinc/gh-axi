//! Version + self-update for gh-axi (GitHub releases), mirroring cf's `version.rs`
//! (#368): read `releases/latest` (never the tags feed), so a tag-only release is
//! invisible to the update feed.

use crate::error::{AxiError, Result};
use std::cmp::Ordering;
use std::io::Write;
use std::process::Command;

pub const REPO: &str = "https://github.com/thalixinc/gh-axi";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LATEST_RELEASE_ENDPOINT: &str = "repos/thalixinc/gh-axi/releases/latest";

/// Compare two dotted semvers; missing components count as 0. A prerelease suffix
/// (e.g. `-rc1`) is dropped by the simple numeric parse, matching the family.
pub fn semver_cmp(a: &str, b: &str) -> Ordering {
    let pa: Vec<u64> = a.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    let pb: Vec<u64> = b.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    for i in 0..pa.len().max(pb.len()) {
        match pa
            .get(i)
            .copied()
            .unwrap_or(0)
            .cmp(&pb.get(i).copied().unwrap_or(0))
        {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// Whether `latest` is a strictly newer release than the current build.
pub fn update_available(latest: &str) -> bool {
    semver_cmp(latest, VERSION) == Ordering::Greater
}

/// Fetch the latest release tag (leading `v` stripped). `Ok(None)` = no release published.
pub fn fetch_latest_release() -> Result<Option<String>> {
    let out = Command::new("gh")
        .args(["api", LATEST_RELEASE_ENDPOINT, "--jq", ".tag_name"])
        .output()
        .map_err(|e| AxiError::operational(format!("`gh` not available: {e}"), "UPDATE_CHECK"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("404") || stderr.contains("No releases found") {
            return Ok(None); // no releases published yet
        }
        return Err(AxiError::operational(
            "could not fetch the release feed (check `gh` auth and network)",
            "UPDATE_CHECK",
        )
        .with_suggestions(vec![
            "Run `gh auth login` to authenticate with GitHub.".into()
        ]));
    }

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(None); // no releases published yet
    }
    Ok(Some(stdout.trim_start_matches('v').to_string()))
}

/// Run `cargo install --git <REPO> --tag v<latest> --force` (cf #368).
pub fn do_update(latest: &str) -> Result<()> {
    let tag = format!("v{latest}");
    let status = Command::new("cargo")
        .args(["install", "--git", REPO, "--tag", &tag, "--force"])
        .status()
        .map_err(|e| AxiError::operational(format!("`cargo` not available: {e}"), "UPDATE"))?;
    if !status.success() {
        return Err(
            AxiError::operational("cargo install failed", "UPDATE").with_suggestions(vec![
                format!("Run `cargo install --git {REPO} --tag {tag} --force` manually."),
            ]),
        );
    }
    Ok(())
}

/// `gh-axi version [--yes]` — report the current version, and when a newer release is
/// published, report `update available` (non-tty), prompt `[y/N]` (tty), or install on
/// `--yes`. The version line always comes first; a missing/empty feed degrades to it.
pub fn run_version(yes: bool, latest: Option<&str>, tty: bool) -> Result<String> {
    let version_line = format!("gh-axi {VERSION}");
    let Some(latest) = latest.filter(|v| update_available(v)) else {
        return Ok(version_line);
    };

    if yes {
        do_update(latest)?;
        return Ok(format!(
            "{version_line}\nupdate: gh-axi upgraded {VERSION} -> {latest}"
        ));
    }

    if !tty {
        return Ok(format!(
            "{version_line}\nupdate available: {latest} — run `gh-axi update`, or `gh-axi version --yes` to update now"
        ));
    }

    // Interactive: draw everything directly so the version line precedes the prompt.
    println!("{version_line}");
    print!("update available: {latest} — run gh-axi update, or --yes to update now [y/N] ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    if matches!(line.trim(), "y" | "Y" | "yes" | "YES") {
        do_update(latest)?;
        println!("update: gh-axi upgraded {VERSION} -> {latest}");
    } else {
        println!("(kept at {VERSION})");
    }
    Ok(String::new())
}

/// `gh-axi update [--check]` — install the latest release, or (with `--check`)
/// report current vs latest.
pub fn run_update(check: bool, latest: Option<&str>) -> Result<String> {
    match latest {
        None => {
            if check {
                Ok(format!(
                    "update:\n  package: gh-axi\n  current: {VERSION}\n  latest: (none published)\n  available: false"
                ))
            } else {
                Ok(format!(
                    "ok: gh-axi already at latest ({VERSION}) — no published release"
                ))
            }
        }
        Some(latest) => {
            let available = semver_cmp(latest, VERSION) == Ordering::Greater;
            if check {
                Ok(format!(
                    "update:\n  package: gh-axi\n  current: {VERSION}\n  latest: {latest}\n  available: {available}"
                ))
            } else if !available {
                Ok(format!("ok: gh-axi already at latest ({VERSION})"))
            } else {
                do_update(latest)?;
                Ok(format!("update: gh-axi upgraded {VERSION} -> {latest}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_ordering() {
        assert_eq!(semver_cmp("0.2.0", "0.1.0"), Ordering::Greater);
        assert_eq!(semver_cmp("0.1.0", "0.2.0"), Ordering::Less);
        assert_eq!(semver_cmp("0.1.0", "0.1.0"), Ordering::Equal);
        assert_eq!(semver_cmp("0.1.0", "0.1.0-rc1"), Ordering::Equal);
        assert_eq!(semver_cmp("1.0.0", "1.0"), Ordering::Equal);
    }

    #[test]
    fn update_available_gates_on_newer() {
        assert!(update_available("0.2.0"));
        assert!(!update_available("0.1.35"));
        assert!(!update_available("0.1.34"));
    }

    // #339 regression guard: the update feed keys off `releases/latest`, never the
    // git-tags feed — a tag-only release must be invisible to this path.
    #[test]
    fn update_reads_releases_latest_not_tags() {
        assert_eq!(
            LATEST_RELEASE_ENDPOINT,
            "repos/thalixinc/gh-axi/releases/latest"
        );
        assert!(LATEST_RELEASE_ENDPOINT.contains("releases/latest"));
    }

    #[test]
    fn version_continuity_is_pinned() {
        assert_eq!(VERSION, "0.1.35");
    }

    #[test]
    fn version_on_latest_or_no_release_is_just_the_line() {
        assert_eq!(
            run_version(false, None, false).unwrap(),
            format!("gh-axi {VERSION}")
        );
        assert_eq!(
            run_version(false, Some("0.1.35"), false).unwrap(),
            format!("gh-axi {VERSION}")
        );
    }

    #[test]
    fn version_reports_update_when_newer_and_non_tty() {
        let out = run_version(false, Some("0.2.0"), false).unwrap();
        assert_eq!(
            out,
            format!(
                "gh-axi {VERSION}\nupdate available: 0.2.0 — run `gh-axi update`, or `gh-axi version --yes` to update now"
            )
        );
    }

    #[test]
    fn update_check_reports_no_release() {
        let out = run_update(true, None).unwrap();
        assert_eq!(
            out,
            format!(
                "update:\n  package: gh-axi\n  current: {VERSION}\n  latest: (none published)\n  available: false"
            )
        );
    }
}
