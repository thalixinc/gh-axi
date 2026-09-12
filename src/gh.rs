//! gh subprocess execution, mirroring `src/gh.ts`.

use crate::context::{RepoContext, RepoSource};
use crate::error::{gh_not_installed_error, map_gh_error, AxiError, Result};
use serde::de::DeserializeOwned;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Append `--repo <nwo>` for flag/env sources (git remote is auto-detected by gh).
fn build_args(args: &[&str], ctx: Option<&RepoContext>) -> Vec<String> {
    let mut out: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    if let Some(ctx) = ctx {
        if ctx.source != RepoSource::Git {
            out.push("--repo".to_string());
            out.push(ctx.nwo.clone());
        }
    }
    out
}

/// Override the wrapped `gh` binary. Unset or blank keeps PATH lookup (`gh`).
pub fn resolve_gh_bin() -> String {
    std::env::var("GH_BIN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "gh".to_string())
}

fn missing_gh_error() -> AxiError {
    let overridden = std::env::var("GH_BIN").ok().map(|s| s.trim().to_string());
    if let Some(bin) = overridden.filter(|s| !s.is_empty()) {
        AxiError::operational(
            format!("GH_BIN is not an executable gh binary: {bin}"),
            "GH_NOT_INSTALLED",
        )
    } else {
        gh_not_installed_error()
    }
}

fn run(args: &[String]) -> ExecResult {
    run_with_input(args, None)
}

fn run_with_input(args: &[String], input: Option<&str>) -> ExecResult {
    let bin = resolve_gh_bin();
    let mut cmd = Command::new(&bin);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return ExecResult {
                stdout: String::new(),
                stderr: "ENOENT".to_string(),
                exit_code: 127,
            };
        }
        Err(e) => {
            return ExecResult {
                stdout: String::new(),
                stderr: e.to_string(),
                exit_code: 1,
            };
        }
    };

    if let Some(input) = input {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
    }

    match child.wait_with_output() {
        Ok(out) => ExecResult {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            exit_code: out.status.code().unwrap_or(1),
        },
        Err(e) => ExecResult {
            stdout: String::new(),
            stderr: e.to_string(),
            exit_code: 1,
        },
    }
}

/// Execute gh and return parsed JSON.
pub fn gh_json<T: DeserializeOwned>(args: &[&str], ctx: Option<&RepoContext>) -> Result<T> {
    let result = run(&build_args(args, ctx));
    if result.stderr == "ENOENT" {
        return Err(missing_gh_error());
    }
    if result.exit_code != 0 {
        return Err(map_gh_error(&result.stderr, result.exit_code));
    }
    serde_json::from_str(&result.stdout).map_err(|_| {
        AxiError::operational(
            format!(
                "Unexpected gh output: {}",
                result.stdout.chars().take(200).collect::<String>()
            ),
            "UNKNOWN",
        )
    })
}

/// Execute gh and return raw stdout.
pub fn gh_exec(args: &[&str], ctx: Option<&RepoContext>) -> Result<String> {
    let result = run(&build_args(args, ctx));
    if result.stderr == "ENOENT" {
        return Err(missing_gh_error());
    }
    if result.exit_code != 0 {
        return Err(map_gh_error(&result.stderr, result.exit_code));
    }
    Ok(result.stdout)
}

/// Execute gh, returning stdout + stderr without throwing on non-zero exit.
pub fn gh_raw(args: &[&str], ctx: Option<&RepoContext>) -> Result<ExecResult> {
    let result = run(&build_args(args, ctx));
    if result.stderr == "ENOENT" {
        return Err(missing_gh_error());
    }
    Ok(result)
}

/// Execute gh, writing `input` to the child's stdin instead of a CLI flag.
/// Keeps sensitive values (secret/variable bodies) out of the argv gh receives.
pub fn gh_exec_with_stdin(args: &[&str], input: &str, ctx: Option<&RepoContext>) -> Result<String> {
    let result = run_with_input(&build_args(args, ctx), Some(input));
    if result.stderr == "ENOENT" {
        return Err(missing_gh_error());
    }
    if result.exit_code != 0 {
        return Err(map_gh_error(&result.stderr, result.exit_code));
    }
    Ok(result.stdout)
}
