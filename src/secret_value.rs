//! Secret/variable value resolution, mirroring `src/secretValue.ts`.

use crate::error::{AxiError, Result};
use crate::stdin::{is_stdin_tty, read_stdin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Noun {
    Secret,
    Variable,
}

fn value_required_error(noun: Noun) -> AxiError {
    match noun {
        Noun::Secret => AxiError::usage("secret value is required: pipe the value via stdin")
            .with_suggestions(vec!["echo -n \"<value>\" | gh-axi secret set <name>".into()]),
        Noun::Variable => AxiError::usage(
            "variable value is required: pass --body <value> or pipe the value via stdin",
        )
        .with_suggestions(vec![
            "gh-axi variable set <name> --body <value>".into(),
            "echo -n \"<value>\" | gh-axi variable set <name>".into(),
        ]),
    }
}

/// Resolve a secret/variable value from an already-extracted flag value, or from
/// piped stdin. Secret callers pass no flag value so secrets are stdin-only.
/// Never accepts an interactive TTY prompt.
pub fn resolve_value(flag_value: Option<&str>, noun: Noun) -> Result<String> {
    if let Some(v) = flag_value {
        if noun == Noun::Secret {
            return Err(AxiError::usage(
                "Secret values must be piped via stdin; --body/-b is not accepted for secrets",
            )
            .with_suggestions(vec!["echo -n \"<value>\" | gh-axi secret set <name>".into()]));
        }
        if v.is_empty() {
            return Err(
                AxiError::usage("--body requires a value").with_suggestions(vec![format!(
                    "gh-axi {} set <name> --body <value>",
                    match noun {
                        Noun::Secret => "secret",
                        Noun::Variable => "variable",
                    }
                )]),
            );
        }
        return Ok(v.to_string());
    }

    if is_stdin_tty() {
        return Err(value_required_error(noun));
    }

    let value = read_stdin();
    if value.is_empty() {
        return Err(value_required_error(noun));
    }
    Ok(value)
}
