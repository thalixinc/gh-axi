//! Repository context resolution, mirroring `src/context.ts`.

use crate::host::{resolve_host, HostContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoSource {
    Flag,
    Env,
    Git,
}

#[derive(Debug, Clone)]
pub struct RepoContext {
    pub owner: String,
    pub name: String,
    /// Full "OWNER/NAME" string.
    pub nwo: String,
    /// How the repo was resolved — determines whether to append `--repo` to gh calls.
    pub source: RepoSource,
    pub host: Option<HostContext>,
}

/// Resolve the target repository. Priority: `--repo` flag > `GH_REPO` env > git remote origin.
pub fn resolve_repo(flag_value: Option<&str>) -> Option<RepoContext> {
    if let Some(v) = flag_value {
        return parse_nwo(v, RepoSource::Flag);
    }
    if let Ok(env_repo) = std::env::var("GH_REPO") {
        if !env_repo.is_empty() {
            return parse_nwo(&env_repo, RepoSource::Env);
        }
    }
    let url = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;
    parse_remote_url(&url)
}

fn parse_nwo(nwo: &str, source: RepoSource) -> Option<RepoContext> {
    let parts: Vec<&str> = nwo.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some(RepoContext {
        owner: parts[0].to_string(),
        name: parts[1].to_string(),
        nwo: nwo.to_string(),
        source,
        host: None,
    })
}

fn parse_remote_url(url: &str) -> Option<RepoContext> {
    let host = resolve_host(None);
    let escaped = regex::escape(&host);

    // SSH: git@<host>:OWNER/NAME.git (or ssh://git@<host>/OWNER/NAME.git)
    let ssh = regex::Regex::new(&format!(
        r"(?:^|@|/){escaped}[:/]([^/]+)/([^/]+?)(?:\.git)?$"
    ))
    .ok()?;
    if let Some(caps) = ssh.captures(url) {
        return Some(repo_from_parts(
            caps.get(1)?.as_str(),
            caps.get(2)?.as_str(),
        ));
    }

    // HTTPS: https://<host>/OWNER/NAME.git
    let https =
        regex::Regex::new(&format!(r"(?:^|@|/){escaped}/([^/]+)/([^/]+?)(?:\.git)?$")).ok()?;
    if let Some(caps) = https.captures(url) {
        return Some(repo_from_parts(
            caps.get(1)?.as_str(),
            caps.get(2)?.as_str(),
        ));
    }

    None
}

fn repo_from_parts(owner: &str, name: &str) -> RepoContext {
    RepoContext {
        owner: owner.to_string(),
        name: name.to_string(),
        nwo: format!("{owner}/{name}"),
        source: RepoSource::Git,
        host: None,
    }
}
