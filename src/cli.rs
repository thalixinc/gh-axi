//! Command-first dispatch, mirroring the Node CLI's `runAxiCli` surface.

use crate::error::{AxiError, Result};
use crate::toon;
use crate::version;
use std::io::IsTerminal;

pub const TOP_HELP: &str = r#"usage: gh-axi [command] [args] [flags]
commands[17]:
  (none)=dashboard, issue, pr, stack, run, workflow, release, repo, label, gist, project, secret, variable, search, api, setup, version
flags[4]:
  -R/--repo <OWNER/NAME> (after command), --hostname <host> (after command) or GH_HOST env, both flags accept space or equals form, --help, -v/-V/--version
requires:
  gh >= 2.99.0 for --attach on issue/pr create, edit, and comment (set GH_BIN to override the gh binary)
examples:
  gh-axi
  gh-axi issue list --state open
  gh-axi issue list -R owner/name
  gh-axi issue list --repo=owner/name
  gh-axi issue list --hostname git.example.com
  gh-axi pr view 42
  gh-axi stack view
  gh-axi secret list
  gh-axi setup hooks
"built-in":
  update: Upgrade `gh-axi` to the latest published release
  "update --check": Report current vs latest without installing
"#;

pub const VERSION_HELP: &str = r#"usage: gh-axi version [--yes]
Print the installed version and report an available update (from the GitHub releases feed).

flags:
  --yes   upgrade now when a newer version is published (cargo install --git https://github.com/thalixinc/gh-axi)

examples:
  gh-axi version
  gh-axi version --yes
"#;

pub const UPDATE_HELP: &str = r#"command: update
description: Upgrade `gh-axi` to the latest published release
flags:
  "--check": Report current vs latest and exit without installing
examples[2]: gh-axi update,gh-axi update --check
"#;

/// The 16 named verbs (plus the `(none)` dashboard); `update` is the built-in.
const KNOWN_COMMANDS: [&str; 16] = [
    "issue", "pr", "stack", "run", "workflow", "release", "repo", "label", "gist", "project",
    "secret", "variable", "search", "api", "setup", "version",
];

/// The result of a dispatch: the full stdout and the process exit code.
pub struct Outcome {
    pub output: String,
    pub exit_code: i32,
}

impl Outcome {
    fn ok(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            exit_code: 0,
        }
    }

    fn err(err: AxiError) -> Self {
        let exit_code = err.exit_code();
        Self {
            output: format!(
                "{}\n",
                toon::render_error(&err.message, err.code, &err.suggestions)
            ),
            exit_code,
        }
    }
}

/// Run the CLI against the real release feed.
pub fn run(argv: &[String]) -> Outcome {
    run_with(argv, version::fetch_latest_release)
}

/// Run the CLI, injecting the release-feed fetch (so tests avoid the network).
pub fn run_with<F>(argv: &[String], fetch_latest: F) -> Outcome
where
    F: Fn() -> Result<Option<String>>,
{
    // Top-level --help (the SDK special-cases exactly this one flag).
    if argv.len() == 1 && argv[0] == "--help" {
        return Outcome::ok(TOP_HELP.to_string());
    }
    // Bare version flag: `gh-axi <version>` (#368 convention).
    if argv.len() == 1 && matches!(argv[0].as_str(), "-v" | "-V" | "--version") {
        return Outcome::ok(format!("gh-axi {}\n", version::VERSION));
    }
    // No command -> dashboard (home).
    let Some(command) = argv.first().map(String::as_str) else {
        return Outcome::err(not_ported("(dashboard)"));
    };
    // Leading flags are rejected before any command resolves.
    if command.starts_with('-') {
        return Outcome::err(leading_flag_error(command));
    }
    let args = &argv[1..];

    // Built-in `update`.
    if command == "update" {
        return update_outcome(args, fetch_latest);
    }

    // `<command> --help`.
    if args.iter().any(|a| a == "--help") {
        if let Some(help) = command_help(command) {
            return Outcome::ok(help.to_string());
        }
    }

    if !KNOWN_COMMANDS.contains(&command) {
        return Outcome::err(unknown_command(command));
    }

    match command {
        "version" => version_outcome(args, fetch_latest),
        other => Outcome::err(not_ported(other)),
    }
}

fn command_help(command: &str) -> Option<&'static str> {
    match command {
        "version" => Some(VERSION_HELP),
        _ => None, // remaining verb help lands with each ported verb
    }
}

fn version_outcome<F>(args: &[String], fetch_latest: F) -> Outcome
where
    F: Fn() -> Result<Option<String>>,
{
    let yes = args.iter().any(|a| a == "--yes");
    let latest = fetch_latest().unwrap_or(None);
    let tty = std::io::stdin().is_terminal();
    match version::run_version(yes, latest.as_deref(), tty) {
        Ok(output) => Outcome::ok(output),
        Err(err) => Outcome::err(err),
    }
}

fn update_outcome<F>(args: &[String], fetch_latest: F) -> Outcome
where
    F: Fn() -> Result<Option<String>>,
{
    if args.len() == 1 && args[0] == "--help" {
        return Outcome::ok(UPDATE_HELP.to_string());
    }
    let check = args.iter().any(|a| a == "--check");
    match fetch_latest() {
        Ok(latest) => match version::run_update(check, latest.as_deref()) {
            Ok(output) => Outcome::ok(output),
            Err(err) => Outcome::err(err),
        },
        Err(err) => Outcome::err(err),
    }
}

fn leading_flag_error(flag: &str) -> AxiError {
    AxiError::usage("Flags must come after the command").with_suggestions(vec![
        "Run `gh-axi <command> [args] [flags]`".to_string(),
        format!("Move `{flag}` after the command instead of before it"),
    ])
}

fn unknown_command(command: &str) -> AxiError {
    AxiError::usage(format!("Unknown command: {command}"))
        .with_suggestions(vec!["Run `--help` to see available commands".to_string()])
}

/// Honest in-progress signal for verbs that land in later port PRs.
fn not_ported(command: &str) -> AxiError {
    AxiError::operational(
        format!(
            "`gh-axi {command}` is not yet available in the Rust port (epic #8 in progress); the Node CLI still provides it"
        ),
        "NOT_PORTED",
    )
}
