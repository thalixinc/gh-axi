//! Faithful subset of the `@toon-format/toon` v2 encoder the Node CLI emits, so
//! Rust output stays byte-identical: scalars, flat key-value objects, inline
//! arrays of primitive strings, and tabular lists (`label[N]{fields}:` + rows).

use serde_json::Value;

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

/// Encode a JSON scalar as a TOON primitive.
pub fn encode_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => encode_string(s),
        other => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

/// A field extractor for transforming gh JSON into flat TOON cells.
#[derive(Debug, Clone, Copy)]
pub enum FieldDef {
    Field {
        key: &'static str,
        as_: &'static str,
    },
    RelativeTime {
        key: &'static str,
        as_: &'static str,
    },
}

impl FieldDef {
    fn as_name(&self) -> &'static str {
        match self {
            FieldDef::Field { as_, .. } | FieldDef::RelativeTime { as_, .. } => as_,
        }
    }
}

pub fn field(key: &'static str) -> FieldDef {
    FieldDef::Field { key, as_: key }
}

pub fn relative_time(key: &'static str, as_: &'static str) -> FieldDef {
    FieldDef::RelativeTime { key, as_ }
}

fn extract_value(item: &Value, def: &FieldDef) -> Value {
    match def {
        FieldDef::Field { key, .. } => item.get(*key).cloned().unwrap_or(Value::Null),
        FieldDef::RelativeTime { key, .. } => Value::String(format_relative_time(
            item.get(*key).and_then(|v| v.as_str()),
        )),
    }
}

/// `label[N]{f1,f2}:` header + one CSV row per item.
pub fn render_list(label: &str, items: &[Value], schema: &[FieldDef]) -> String {
    if items.is_empty() {
        return format!("{}[0]:", encode_key(label));
    }
    let fields: Vec<String> = schema.iter().map(|f| encode_key(f.as_name())).collect();
    let mut out = format!(
        "{}[{}]{{{}}}:",
        encode_key(label),
        items.len(),
        fields.join(",")
    );
    for item in items {
        let cells: Vec<String> = schema
            .iter()
            .map(|f| encode_value(&extract_value(item, f)))
            .collect();
        out.push_str("\n  ");
        out.push_str(&cells.join(","));
    }
    out
}

/// A flat object of string scalars: `key: value` per pair.
pub fn render_kv(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}: {}", encode_key(k), encode_string(v)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render the `help[N]:` next-step suggestion block (multiline, no dash).
pub fn render_help(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let indented = lines
        .iter()
        .map(|l| format!("  {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("help[{}]:\n{}", lines.len(), indented)
}

/// Combine already-rendered TOON blocks, dropping empty ones.
pub fn render_output(blocks: &[String]) -> String {
    blocks
        .iter()
        .filter(|b| !b.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
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

/// Structured error as TOON with a *multiline* `help[N]:` block (the shape
/// `src/toon.ts renderError` emits for returned errors, vs the inline shape the
/// thrown-error formatter uses).
pub fn render_error_multiline(message: &str, code: &str, suggestions: &[String]) -> String {
    let head = format!(
        "{}: {}\n{}: {}",
        encode_key("error"),
        encode_string(message),
        encode_key("code"),
        encode_string(code)
    );
    if suggestions.is_empty() {
        return head;
    }
    format!("{head}\n{}", render_help(suggestions))
}

fn format_relative_time(iso: Option<&str>) -> String {
    let Some(iso) = iso else {
        return "unknown".to_string();
    };
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return "unknown".to_string();
    };
    let now = chrono::Utc::now();
    let secs = now
        .signed_duration_since(dt.with_timezone(&chrono::Utc))
        .num_seconds();
    if secs < 60 {
        return "just now".to_string();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hrs = mins / 60;
    if hrs < 24 {
        return format!("{hrs}h ago");
    }
    let days = hrs / 24;
    if days < 30 {
        return format!("{days}d ago");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo ago");
    }
    let years = months / 12;
    format!("{years}y ago")
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
        assert_eq!(encode_string("null"), "\"null\"");
    }

    #[test]
    fn leaves_plain_strings_unquoted() {
        assert_eq!(encode_string("VALIDATION_ERROR"), "VALIDATION_ERROR");
        assert_eq!(encode_string("0.1.35"), "0.1.35");
        assert_eq!(encode_string("gh-axi"), "gh-axi");
    }

    #[test]
    fn key_quoting() {
        assert_eq!(encode_key("error"), "error");
        assert_eq!(encode_key("built-in"), "\"built-in\"");
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
    fn tabular_list_shape() {
        let items = vec![
            serde_json::json!({ "name": "a,b" }),
            serde_json::json!({ "name": "c" }),
        ];
        assert_eq!(
            render_list("labels", &items, &[field("name")]),
            "labels[2]{name}:\n  \"a,b\"\n  c"
        );
    }

    #[test]
    fn empty_list_shape() {
        assert_eq!(render_list("labels", &[], &[field("name")]), "labels[0]:");
    }

    #[test]
    fn kv_shape() {
        assert_eq!(
            render_kv(&[("set", "ok"), ("variable", "NODE_ENV")]),
            "set: ok\nvariable: NODE_ENV"
        );
    }

    #[test]
    fn help_block_shape() {
        assert_eq!(
            render_help(&["line1".to_string(), "line2".to_string()]),
            "help[2]:\n  line1\n  line2"
        );
    }
}
