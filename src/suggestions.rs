//! Contextual suggestions, mirroring `src/suggestions.ts`.

use crate::context::{RepoContext, RepoSource};
use crate::host::{HostContext, HostSource, DEFAULT_HOST};

pub struct SuggestionContext<'a> {
    pub domain: &'a str,
    pub action: &'a str,
    pub state: Option<&'a str>,
    pub is_empty: Option<bool>,
    pub id: Option<String>,
    pub repo: Option<&'a RepoContext>,
    pub host: Option<&'a HostContext>,
    pub owner: Option<&'a str>,
}

fn repo_flag(ctx: &SuggestionContext) -> String {
    match ctx.repo {
        Some(repo) if repo.source != RepoSource::Git => format!(" -R {}", repo.nwo),
        _ => String::new(),
    }
}

fn owner_flag(ctx: &SuggestionContext) -> String {
    ctx.owner
        .map(|o| format!(" --owner {o}"))
        .unwrap_or_default()
}

fn normalize_repo_flag_line(line: &str) -> String {
    let re = regex::Regex::new(r"`gh-axi -R ([^`\s]+) ([^`]+)`").unwrap();
    re.replace_all(line, "`gh-axi $2 -R $1`").into_owned()
}

fn hostname_flag(ctx: &SuggestionContext) -> String {
    let host = ctx.host.or_else(|| ctx.repo.and_then(|r| r.host.as_ref()));
    match host {
        Some(h) if h.source == HostSource::Flag && h.value != DEFAULT_HOST => {
            format!(" --hostname {}", h.value)
        }
        _ => String::new(),
    }
}

fn append_hostname_flag(line: &str, ctx: &SuggestionContext) -> String {
    let flag = hostname_flag(ctx);
    if flag.is_empty() {
        return line.to_string();
    }
    let re = regex::Regex::new(r"`([^`]*\bgh-axi\b[^`]*)`").unwrap();
    re.replace_all(line, format!("`$1{flag}`")).into_owned()
}

/// First matching suggestion entry wins; returns `[]` when nothing matches.
pub fn get_suggestions(ctx: &SuggestionContext) -> Vec<String> {
    let finish = |lines: Vec<String>| -> Vec<String> {
        lines
            .into_iter()
            .map(|l| normalize_repo_flag_line(&l))
            .map(|l| append_hostname_flag(&l, ctx))
            .collect()
    };
    let rf = repo_flag(ctx);

    let d = ctx.domain;
    let a = ctx.action;

    // Home
    if d == "home" {
        return finish(vec![
            "Run `gh-axi <command> <subcommand>` — commands: issue, pr, run, release, repo, label, secret, variable".into(),
        ]);
    }

    // Issue
    if d == "issue" {
        let id = ctx.id.as_deref().unwrap_or("");
        match a {
            "list" if !ctx.is_empty.unwrap_or(false) => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue view <number>` to view details"),
                    format!("Run `gh-axi{rf} issue create --title \"...\" --body-file <path>` to create"),
                ]);
            }
            "list" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue create --title \"...\" --body-file <path>` to create an issue"),
                    format!("Run `gh-axi{rf} issue list --state closed` to see closed issues"),
                ]);
            }
            "view" if ctx.state == Some("open") => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue comment {id} --body-file <path>` to comment"),
                    format!("Run `gh-axi{rf} issue close {id}` to close"),
                    format!("Run `gh-axi{rf} issue edit {id} --add-assignee <user>` to assign"),
                    format!(
                        "Run `gh-axi search prs \"{id}\"{}` to find PRs referencing this issue",
                        ctx.repo
                            .map(|r| format!(" --repo {}", r.nwo))
                            .unwrap_or_default()
                    ),
                ]);
            }
            "view" if ctx.state == Some("closed") => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue reopen {id}` to reopen"),
                    format!("Run `gh-axi{rf} issue comment {id} --body-file <path>` to comment"),
                    format!(
                        "Run `gh-axi search prs \"{id}\"{}` to find PRs referencing this issue",
                        ctx.repo
                            .map(|r| format!(" --repo {}", r.nwo))
                            .unwrap_or_default()
                    ),
                ]);
            }
            "create" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue view {id}` to see the full issue"),
                    format!("Run `gh-axi{rf} issue edit {id} --add-label <label>` to label"),
                ]);
            }
            "close" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} issue reopen {id}` to reopen"
                )])
            }
            "reopen" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue close {id}` to close"),
                    format!("Run `gh-axi{rf} issue view {id}` to see details"),
                ]);
            }
            "edit" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} issue view {id}` to see updated issue"
                )])
            }
            "comment" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} issue view {id} --comments` to see all comments"
                )]);
            }
            "delete" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} issue list` to see remaining issues"
                )])
            }
            "lock" | "unlock" | "pin" | "unpin" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} issue view {id}` to see issue details"
                )]);
            }
            "transfer" => return finish(vec![]),
            _ => {}
        }
    }

    // PR
    if d == "pr" {
        let id = ctx.id.as_deref().unwrap_or("");
        match a {
            "list" if !ctx.is_empty.unwrap_or(false) => {
                return finish(vec![
                    format!("Run `gh-axi{rf} pr view <number>` to view details"),
                    format!(
                        "Run `gh-axi{rf} pr create --title \"...\" --body-file <path>` to create"
                    ),
                ]);
            }
            "list" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} pr create --title \"...\" --body-file <path>` to create a PR"),
                    format!("Run `gh-axi{rf} pr list --state closed` to see closed PRs"),
                ]);
            }
            "view" if ctx.state == Some("open") => {
                return finish(vec![
                    format!("Run `gh-axi{rf} pr checks {id}` to see CI status"),
                    format!("Run `gh-axi{rf} pr review {id} --approve` to approve"),
                    format!("Run `gh-axi{rf} pr merge {id}` to merge"),
                ]);
            }
            "view" if ctx.state == Some("closed") => {
                return finish(vec![format!("Run `gh-axi{rf} pr reopen {id}` to reopen")]);
            }
            "view" if ctx.state == Some("merged") => {
                return finish(vec![format!("Run `gh-axi{rf} pr revert {id}` to revert")]);
            }
            "create" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} pr view {id}` to see the full PR"),
                    format!("Run `gh-axi{rf} pr checks {id}` to monitor CI"),
                ]);
            }
            "close" => return finish(vec![format!("Run `gh-axi{rf} pr reopen {id}` to reopen")]),
            "merge" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr revert {id}` to revert if needed"
                )])
            }
            "review" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr view {id}` to see PR details"
                )])
            }
            "checks" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} pr view {id}` to see PR details"),
                    format!("Run `gh-axi{rf} pr merge {id}` to merge when ready"),
                ]);
            }
            "diff" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr review {id} --approve` to approve"
                )])
            }
            "checkout" => return finish(vec![]),
            "ready" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr view {id}` to see PR status"
                )])
            }
            "reopen" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr view {id}` to see PR details"
                )])
            }
            "comment" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr view {id} --comments` to see all comments"
                )]);
            }
            "update-branch" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr checks {id}` to monitor CI after update"
                )]);
            }
            "revert" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} pr view {id}` to see the revert PR"
                )])
            }
            _ => {}
        }
    }

    // Run
    if d == "run" {
        let id = ctx.id.as_deref().unwrap_or("");
        match a {
            "list" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run view <id>` to view details"
                )])
            }
            "view" if ctx.state == Some("completed") => {
                return finish(vec![
                    format!("Run `gh-axi{rf} run rerun {id}` to rerun"),
                    format!("Run `gh-axi{rf} run view {id} --log-failed` to see failure logs"),
                ]);
            }
            "view" if ctx.state == Some("in_progress") => {
                return finish(vec![
                    format!("Run `gh-axi{rf} run watch {id}` to watch until completion"),
                    format!("Run `gh-axi{rf} run cancel {id}` to cancel"),
                ]);
            }
            "view" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run view {id} --log` to see run logs"
                )])
            }
            "rerun" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run watch {id}` to monitor progress"
                )])
            }
            "cancel" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run view {id}` to see final state"
                )])
            }
            "delete" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run list` to see remaining runs"
                )])
            }
            "watch" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run view {id}` to see details"
                )])
            }
            "download" => return finish(vec![]),
            _ => {}
        }
    }

    // Workflow
    if d == "workflow" {
        let id = ctx.id.as_deref().unwrap_or("");
        match a {
            "list" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} workflow view <id>` to view details"),
                    format!("Run `gh-axi{rf} workflow run <id>` to trigger a run"),
                ]);
            }
            "view" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} workflow run {id}` to trigger"),
                    format!("Run `gh-axi{rf} run list --workflow {id}` to see past runs"),
                ]);
            }
            "run" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} run list` to see triggered run"
                )])
            }
            "enable" | "disable" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} workflow list` to see all workflows"
                )]);
            }
            _ => {}
        }
    }

    // Release
    if d == "release" {
        let id = ctx.id.as_deref().unwrap_or("");
        match a {
            "list" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} release view <tag>` to view details"),
                    format!("Run `gh-axi{rf} release create <tag> --body-file <path>` to create a release"),
                ]);
            }
            "view" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} release download {id}` to download assets"),
                    format!("Run `gh-axi{rf} release edit {id} --body-file <path>` to edit notes"),
                ]);
            }
            "create" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} release view {id}` to view the release"),
                    format!("Run `gh-axi{rf} release upload {id} <files...>` to upload assets"),
                ]);
            }
            "edit" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} release view {id}` to see updated release"
                )])
            }
            "delete" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} release list` to see remaining releases"
                )])
            }
            "download" => return finish(vec![]),
            "upload" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} release view {id}` to see all assets"
                )])
            }
            _ => {}
        }
    }

    // Repo
    if d == "repo" {
        match a {
            "view" => {
                return finish(vec![
                    format!("Run `gh-axi{rf} issue list` to see issues"),
                    format!("Run `gh-axi{rf} pr list` to see pull requests"),
                ]);
            }
            "create" => {
                return finish(vec![
                    "Run `gh-axi repo view --repo <owner/name>` to see the new repository".into(),
                ]);
            }
            "list" => {
                return finish(vec![
                    "Run `gh-axi repo view --repo <owner/name>` to view a repository".into(),
                ]);
            }
            "edit" | "clone" | "fork" => return finish(vec![]),
            _ => {}
        }
    }

    // Label
    if d == "label" {
        match a {
            "list" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} label create --name \"...\" --color \"...\"` to create a label"
                )]);
            }
            "create" | "edit" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} label list` to see all labels"
                )])
            }
            "delete" => {
                return finish(vec![format!(
                    "Run `gh-axi{rf} label list` to see remaining labels"
                )])
            }
            _ => {}
        }
    }

    // Project
    if d == "project" {
        let id = ctx.id.as_deref().unwrap_or("");
        let of = owner_flag(ctx);
        match a {
            "list" if !ctx.is_empty.unwrap_or(false) => {
                return finish(vec![
                    format!("Run `gh-axi project view <number>{of}` to view details"),
                    format!("Run `gh-axi project create --title \"...\"{of}` to create a project"),
                ]);
            }
            "list" => {
                return finish(vec![format!(
                    "Run `gh-axi project create --title \"...\"{of}` to create a project"
                )]);
            }
            "create" => {
                return finish(vec![
                    format!("Run `gh-axi project view {id}{of}` to see the new project"),
                    format!("Run `gh-axi project item-add {id} --url <issue-or-pr-url>{of}` to add items"),
                ]);
            }
            "edit" => {
                return finish(vec![format!(
                    "Run `gh-axi project view {id}{of}` to see the updated project"
                )])
            }
            "close" => {
                return finish(vec![format!(
                    "Run `gh-axi project close {id} --undo{of}` to reopen"
                )])
            }
            "copy" => {
                return finish(vec![format!(
                    "Run `gh-axi project view {id}{of}` to see the copied project"
                )])
            }
            "item-list" => {
                return finish(vec![
                    format!("Run `gh-axi project item-add {id} --url <issue-or-pr-url>{of}` to add an item"),
                    format!("Run `gh-axi project field-list {id}{of}` to see project fields"),
                ]);
            }
            "field-list" => {
                return finish(vec!["Run `gh-axi project item-edit --id <item-id> --field-id <field-id> --project-id <project-id> --text \"...\"` to set a field value".into()]);
            }
            "item-add" | "item-create" => {
                return finish(vec![format!(
                    "Run `gh-axi project item-list {id}{of}` to see all items"
                )]);
            }
            "item-edit" => return finish(vec![]),
            "item-archive" | "item-delete" => {
                return finish(vec![format!(
                    "Run `gh-axi project item-list {id}{of}` to see remaining items"
                )]);
            }
            _ => {}
        }
    }

    // Secret
    if d == "secret" {
        match a {
            "list" => {
                return finish(vec![format!(
                    "Run `echo -n \"<value>\" | gh-axi secret set <name>{rf}` to add{} a secret",
                    if ctx.is_empty.unwrap_or(false) {
                        ""
                    } else {
                        " or update"
                    }
                )]);
            }
            "set" => {
                return finish(vec![format!(
                    "Run `gh-axi secret list{rf}` to see all secrets"
                )])
            }
            "delete" => {
                return finish(vec![format!(
                    "Run `gh-axi secret list{rf}` to see remaining secrets"
                )])
            }
            _ => {}
        }
    }

    // Variable
    if d == "variable" {
        match a {
            "list" => {
                return finish(vec![format!(
                    "Run `gh-axi variable set <name> --body <value>{rf}` to add{} a variable",
                    if ctx.is_empty.unwrap_or(false) {
                        ""
                    } else {
                        " or update"
                    }
                )]);
            }
            "set" => {
                return finish(vec![format!(
                    "Run `gh-axi variable list{rf}` to see all variables"
                )])
            }
            "delete" => {
                return finish(vec![format!(
                    "Run `gh-axi variable list{rf}` to see remaining variables"
                )])
            }
            _ => {}
        }
    }

    // Gist
    if d == "gist" {
        let id = ctx.id.as_deref().unwrap_or("");
        match a {
            "list" if !ctx.is_empty.unwrap_or(false) => {
                return finish(vec![
                    "Run `gh-axi gist view <id>` to view a gist's files and metadata".into(),
                    "Run `gh-axi gist list --fields url,owner,created` to add extra fields".into(),
                ]);
            }
            "list" => {
                return finish(vec![
                    "Run `gh-axi api /gists` to see gist data via the raw API".into(),
                ])
            }
            "view" => {
                return finish(vec![
                    format!("Run `gh-axi gist view {id} --files` to list file names only"),
                    "Run `gh-axi gist list` to see all your gists".into(),
                ]);
            }
            "edit" => {
                return finish(vec![
                    "Run `gh-axi gist list` to see all gists".into(),
                    format!("Run `gh-axi gist rename {id} <old> <new>` to rename a file"),
                ]);
            }
            "rename" => {
                return finish(vec![
                    "Run `gh-axi gist list` to see all gists".into(),
                    format!("Run `gh-axi gist edit {id} --filename <name>` to edit file content"),
                ]);
            }
            "create" => return finish(vec!["Run `gh-axi gist list` to see all your gists".into()]),
            "delete" => {
                return finish(vec!["Run `gh-axi gist list` to see remaining gists".into()])
            }
            "clone" => return finish(vec!["Run `gh-axi gist list` to see your gists".into()]),
            _ => {}
        }
    }

    // Search / API
    if d == "search" || d == "api" {
        return finish(vec![]);
    }

    vec![]
}
