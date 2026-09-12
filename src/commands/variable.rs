//! `gh-axi variable` — port of `src/commands/variable.ts`.

use crate::context::RepoContext;
use crate::error::{map_clap_error, AxiError, Result};
use crate::gh::{gh_exec, gh_exec_with_stdin, gh_json};
use crate::secret_value::{resolve_value, Noun};
use crate::suggestions::{get_suggestions, SuggestionContext};
use crate::toon::{
    field, relative_time, render_error_multiline, render_help, render_kv, render_list,
    render_output,
};
use clap::{Parser, Subcommand};

pub const VARIABLE_HELP: &str = r#"usage: gh-axi variable <subcommand> [flags]
subcommands[3]:
  list, set <name>, delete <name>
flags{set}:
  --body/-b <value> (reads from stdin if omitted)
examples:
  gh-axi variable list
  gh-axi variable set NODE_ENV --body production
  echo -n "production" | gh-axi variable set NODE_ENV
  gh-axi variable delete NODE_ENV"#;

#[derive(Parser)]
struct VariableCli {
    #[command(subcommand)]
    command: VariableCommand,
}

#[derive(Subcommand)]
enum VariableCommand {
    /// List variables.
    List,
    /// Set a variable (value from --body or stdin).
    Set {
        name: Option<String>,
        #[arg(long = "body", short = 'b')]
        body: Option<String>,
    },
    /// Delete a variable.
    Delete { name: Option<String> },
}

fn suggestions(action: &str, is_empty: Option<bool>, ctx: Option<&RepoContext>) -> Vec<String> {
    get_suggestions(&SuggestionContext {
        domain: "variable",
        action,
        state: None,
        is_empty,
        id: None,
        repo: ctx,
        host: None,
        owner: None,
    })
}

pub fn run(args: &[String], ctx: Option<&RepoContext>) -> Result<String> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    if sub == "--help" || sub.is_empty() {
        return Ok(VARIABLE_HELP.to_string());
    }
    const SUBS: [&str; 3] = ["list", "set", "delete"];
    if !SUBS.contains(&sub) {
        return Ok(render_error_multiline(
            &format!("Unknown subcommand: {sub}"),
            "VALIDATION_ERROR",
            &["Available subcommands: list, set, delete".to_string()],
        ));
    }

    let argv = std::iter::once("gh-axi".to_string()).chain(args.iter().cloned());
    let cli = VariableCli::try_parse_from(argv).map_err(|e| map_clap_error(e, "variable", sub))?;
    match cli.command {
        VariableCommand::List => list(ctx),
        VariableCommand::Set { name, body } => set(name.as_deref(), body.as_deref(), ctx),
        VariableCommand::Delete { name } => delete(name.as_deref(), ctx),
    }
}

fn list(ctx: Option<&RepoContext>) -> Result<String> {
    let variables: Vec<serde_json::Value> =
        gh_json(&["variable", "list", "--json", "name,value,updatedAt"], ctx)?;
    let is_empty = variables.is_empty();
    let sg = suggestions("list", Some(is_empty), ctx);
    Ok(render_output(&[
        format!("count: {}", variables.len()),
        render_list(
            "variables",
            &variables,
            &[
                field("name"),
                field("value"),
                relative_time("updatedAt", "updated"),
            ],
        ),
        render_help(&sg),
    ]))
}

fn set(name: Option<&str>, body: Option<&str>, ctx: Option<&RepoContext>) -> Result<String> {
    let name = name
        .ok_or_else(|| AxiError::usage("Variable name is required: gh-axi variable set <name>"))?;
    let value = resolve_value(body, Noun::Variable)?;
    gh_exec_with_stdin(&["variable", "set", name], &value, ctx)?;

    let sg = suggestions("set", None, ctx);
    Ok(render_output(&[
        render_kv(&[("set", "ok"), ("variable", name)]),
        render_help(&sg),
    ]))
}

fn delete(name: Option<&str>, ctx: Option<&RepoContext>) -> Result<String> {
    let name = name.ok_or_else(|| {
        AxiError::usage("Variable name is required: gh-axi variable delete <name>")
    })?;
    gh_exec(&["variable", "delete", name], ctx)?;

    let sg = suggestions("delete", None, ctx);
    Ok(render_output(&[
        render_kv(&[("delete", "ok"), ("variable", name)]),
        render_help(&sg),
    ]))
}
