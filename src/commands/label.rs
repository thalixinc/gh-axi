//! `gh-axi label` — port of `src/commands/label.ts`.

use crate::context::RepoContext;
use crate::error::{map_clap_error, AxiError, Result};
use crate::format::{format_count_line, CountLineOptions};
use crate::gh::{gh_exec, gh_json};
use crate::suggestions::{get_suggestions, SuggestionContext};
use crate::toon::{
    field, render_error_multiline, render_help, render_kv, render_list, render_output,
};
use clap::{Parser, Subcommand};

pub const LABEL_HELP: &str = r#"usage: gh-axi label <subcommand> [flags]
subcommands[4]:
  list, create, edit <name>, delete <name>
flags{list}:
  --limit <n> (default 500)
flags{create}:
  --name <text> (required), --color <hex> (required, without #), --description <text>
flags{edit}:
  --name, --color, --description
examples:
  gh-axi label list
  gh-axi label create --name "priority:high" --color ff0000 --description "High priority"
  gh-axi label delete "priority:low""#;

#[derive(Parser)]
struct LabelCli {
    #[command(subcommand)]
    command: LabelCommand,
}

#[derive(Subcommand)]
enum LabelCommand {
    /// List labels.
    List {
        #[arg(long)]
        limit: Option<String>,
    },
    /// Create a label.
    Create {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Edit a label.
    Edit {
        name: Option<String>,
        #[arg(long = "name")]
        rename: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a label.
    Delete { name: Option<String> },
}

fn suggestions(action: &str, is_empty: Option<bool>, ctx: Option<&RepoContext>) -> Vec<String> {
    get_suggestions(&SuggestionContext {
        domain: "label",
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
        return Ok(LABEL_HELP.to_string());
    }
    const SUBS: [&str; 4] = ["list", "create", "edit", "delete"];
    if !SUBS.contains(&sub) {
        return Ok(render_error_multiline(
            &format!("Unknown subcommand: {sub}"),
            "VALIDATION_ERROR",
            &["Available subcommands: list, create, edit, delete".to_string()],
        ));
    }

    let argv = std::iter::once("gh-axi".to_string()).chain(args.iter().cloned());
    let cli = LabelCli::try_parse_from(argv).map_err(|e| map_clap_error(e, "label", sub))?;
    match cli.command {
        LabelCommand::List { limit } => list(limit.as_deref(), ctx),
        LabelCommand::Create {
            name,
            color,
            description,
        } => create(
            name.as_deref(),
            color.as_deref(),
            description.as_deref(),
            ctx,
        ),
        LabelCommand::Edit {
            name,
            rename,
            color,
            description,
        } => edit(
            name.as_deref(),
            rename.as_deref(),
            color.as_deref(),
            description.as_deref(),
            ctx,
        ),
        LabelCommand::Delete { name } => delete(name.as_deref(), ctx),
    }
}

fn list(limit_arg: Option<&str>, ctx: Option<&RepoContext>) -> Result<String> {
    let limit = limit_arg.unwrap_or("500");
    let labels: Vec<serde_json::Value> =
        gh_json(&["label", "list", "--json", "name", "--limit", limit], ctx)?;
    let is_empty = labels.is_empty();
    let limit_num: usize = limit.parse().unwrap_or(500);
    let count_line = format_count_line(&CountLineOptions {
        count: labels.len(),
        limit: Some(limit_num),
        total_count: None,
        api_limit_hit: false,
        display_limit: None,
    });
    let sg = suggestions("list", Some(is_empty), ctx);
    Ok(render_output(&[
        count_line,
        render_list("labels", &labels, &[field("name")]),
        render_help(&sg),
    ]))
}

fn create(
    name: Option<&str>,
    color: Option<&str>,
    description: Option<&str>,
    ctx: Option<&RepoContext>,
) -> Result<String> {
    let name = name.ok_or_else(|| {
        AxiError::usage("--name is required: gh-axi label create --name \"...\" --color \"...\"")
    })?;
    let color = color.ok_or_else(|| {
        AxiError::usage("--color is required: gh-axi label create --name \"...\" --color \"...\"")
    })?;

    let existing: Vec<serde_json::Value> = gh_json(&["label", "list", "--json", "name"], ctx)?;
    if let Some(found) = existing.iter().find(|l| {
        l.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.to_lowercase() == name.to_lowercase())
            .unwrap_or(false)
    }) {
        let sg = suggestions("create", None, ctx);
        let found_name = found.get("name").and_then(|n| n.as_str()).unwrap_or(name);
        return Ok(render_output(&[
            render_kv(&[("create", "already_exists"), ("label", found_name)]),
            render_help(&sg),
        ]));
    }

    let mut gh_args: Vec<&str> = vec!["label", "create", name, "--color", color];
    if let Some(d) = description {
        gh_args.push("--description");
        gh_args.push(d);
    }
    gh_exec(&gh_args, ctx)?;

    let sg = suggestions("create", None, ctx);
    Ok(render_output(&[
        render_kv(&[("created", "ok"), ("label", name)]),
        render_help(&sg),
    ]))
}

fn edit(
    name: Option<&str>,
    rename: Option<&str>,
    color: Option<&str>,
    description: Option<&str>,
    ctx: Option<&RepoContext>,
) -> Result<String> {
    let label_name =
        name.ok_or_else(|| AxiError::usage("Label name is required: gh-axi label edit <name>"))?;

    let mut gh_args: Vec<&str> = vec!["label", "edit", label_name];
    if let Some(n) = rename {
        gh_args.push("--name");
        gh_args.push(n);
    }
    if let Some(c) = color {
        gh_args.push("--color");
        gh_args.push(c);
    }
    if let Some(d) = description {
        gh_args.push("--description");
        gh_args.push(d);
    }
    gh_exec(&gh_args, ctx)?;

    let sg = suggestions("edit", None, ctx);
    Ok(render_output(&[
        render_kv(&[("edit", "ok"), ("label", rename.unwrap_or(label_name))]),
        render_help(&sg),
    ]))
}

fn delete(name: Option<&str>, ctx: Option<&RepoContext>) -> Result<String> {
    let name =
        name.ok_or_else(|| AxiError::usage("Label name is required: gh-axi label delete <name>"))?;

    gh_exec(&["label", "delete", name, "--yes"], ctx)?;

    let sg = suggestions("delete", None, ctx);
    Ok(render_output(&[
        render_kv(&[("delete", "ok"), ("label", name)]),
        render_help(&sg),
    ]))
}
