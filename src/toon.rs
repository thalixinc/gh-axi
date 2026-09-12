//! Faithful subset of the `@toon-format/toon` v2 encoder the Node CLI emits, so
//! Rust output stays byte-identical for the shapes gh-axi produces today:
//! scalars, flat key-value objects, and inline arrays of primitive strings.

const DELIMITER: &str = ",";

/// `/^[A-Z_][\w.]*$/i` — a key that needs no quoting.
fn is_valid_unquoted_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Split a leading run of ASCII digits, returning `(count, rest)`.
fn take_digits(s: &str) -> (usize, &str) {
    let n = s.bytes().take_while(|c| c.is_ascii_digit()).count();
    (n, &s[n..])
}

/// `/^-?\d+(?:\.\d+)?(?:e[+-]?\d+)?$/i` or `/^0\d+$/`.
fn is_numeric_like(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() > 1 && b[0] == b'0' && b[1..].iter().all(|c| c.is_ascii_digit()) {
        return true;
    }
    let mut rest = value;
    if let Some(r) = rest.strip_prefix('-') {
        rest = r;
    }
    let (int_len, r) = take_digits(rest);
    if int_len == 0 {
        return false;
    }
    let mut rest = r;
    if let Some(r2) = rest.strip_prefix('.') {
        let (frac_len, r3) = take_digits(r2);
        if frac_len == 0 {
            return false;
        }
        rest = r3;
    }
    if rest.is_empty() {
        return true;
    }
    let r = match rest.strip_prefix('e').or_else(|| rest.strip_prefix('E')) {
        Some(r) => r,
        None => return false,
    };
    let r = r
        .strip_prefix('+')
        .or_else(|| r.strip_prefix('-'))
        .unwrap_or(r);
    let (exp_len, r2) = take_digits(r);
    exp_len > 0 && r2.is_empty()
}

/// Quote a string when it isn't safe to emit unquoted (matches the Node encoder).
fn is_safe_unquoted(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value != value.trim() {
        return false;
    }
    if value == "true" || value == "false" || value == "null" {
        return false;
    }
    if is_numeric_like(value) {
        return false;
    }
    if value.contains(':') || value.contains('"') || value.contains('\\') {
        return false;
    }
    if value
        .chars()
        .any(|c| c == '[' || c == ']' || c == '{' || c == '}')
    {
        return false;
    }
    if value.chars().any(|c| c == '\n' || c == '\r' || c == '\t') {
        return false;
    }
    if value.contains(DELIMITER) {
        return false;
    }
    if value.starts_with('-') {
        return false;
    }
    true
}

fn escape_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// Encode a primitive string value, quoting when necessary.
pub fn encode_string(value: &str) -> String {
    if is_safe_unquoted(value) {
        value.to_string()
    } else {
        format!("\"{}\"", escape_string(value))
    }
}

/// Encode a key, quoting when it isn't a valid unquoted key.
pub fn encode_key(key: &str) -> String {
    if is_valid_unquoted_key(key) {
        key.to_string()
    } else {
        format!("\"{}\"", escape_string(key))
    }
}

/// Inline array of primitive strings: `key[N]: v1,v2`.
pub fn encode_string_array(key: &str, values: &[String]) -> String {
    let joined = values
        .iter()
        .map(|v| encode_string(v))
        .collect::<Vec<_>>()
        .join(DELIMITER);
    format!("{}[{}]: {}", encode_key(key), values.len(), joined)
}

/// Structured error as TOON: `error:` + `code:` + optional inline `help[N]:`.
pub fn render_error(message: &str, code: &str, suggestions: &[String]) -> String {
    let mut lines = vec![
        format!("{}: {}", encode_key("error"), encode_string(message)),
        format!("{}: {}", encode_key("code"), encode_string(code)),
    ];
    if !suggestions.is_empty() {
        lines.push(encode_string_array("help", suggestions));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_colon() {
        assert_eq!(encode_string("a: b"), "\"a: b\"");
    }

    #[test]
    fn quotes_brackets() {
        assert_eq!(encode_string("x [y]"), "\"x [y]\"");
    }

    #[test]
    fn quotes_delimiter() {
        assert_eq!(encode_string("a,b"), "\"a,b\"");
    }

    #[test]
    fn quotes_leading_dash() {
        assert_eq!(encode_string("-item"), "\"-item\"");
    }

    #[test]
    fn quotes_boolean_and_null_literals() {
        assert_eq!(encode_string("true"), "\"true\"");
        assert_eq!(encode_string("false"), "\"false\"");
        assert_eq!(encode_string("null"), "\"null\"");
    }

    #[test]
    fn quotes_numeric_like() {
        assert_eq!(encode_string("123"), "\"123\"");
        assert_eq!(encode_string("0123"), "\"0123\"");
        assert_eq!(encode_string("-1.5e3"), "\"-1.5e3\"");
    }

    #[test]
    fn quotes_whitespace_padded() {
        assert_eq!(encode_string(" x "), "\" x \"");
    }

    #[test]
    fn leaves_plain_strings_unquoted() {
        assert_eq!(encode_string("VALIDATION_ERROR"), "VALIDATION_ERROR");
        assert_eq!(encode_string("0.1.35"), "0.1.35");
        assert_eq!(encode_string("gh-axi"), "gh-axi");
    }

    #[test]
    fn escapes_inner_quotes() {
        assert_eq!(encode_string("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn key_quoting() {
        assert_eq!(encode_key("error"), "error");
        assert_eq!(encode_key("built-in"), "\"built-in\"");
        assert_eq!(encode_key("update --check"), "\"update --check\"");
    }

    #[test]
    fn error_shape_matches_node() {
        let out = render_error(
            "Unknown command: bogus",
            "VALIDATION_ERROR",
            &["Run `--help` to see available commands".to_string()],
        );
        assert_eq!(
            out,
            "error: \"Unknown command: bogus\"\ncode: VALIDATION_ERROR\nhelp[1]: Run `--help` to see available commands"
        );
    }

    #[test]
    fn error_help_array_quotes_each_cell() {
        let out = render_error(
            "unknown flag for gh-axi issue list: --bogus",
            "VALIDATION_ERROR",
            &[
                "gh-axi issue list [flags]".to_string(),
                "gh-axi issue list --help".to_string(),
            ],
        );
        assert_eq!(
            out,
            "error: \"unknown flag for gh-axi issue list: --bogus\"\ncode: VALIDATION_ERROR\nhelp[2]: \"gh-axi issue list [flags]\",gh-axi issue list --help"
        );
    }
}
