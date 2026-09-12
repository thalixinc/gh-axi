//! GitHub host resolution, mirroring `src/host.ts`.

pub const DEFAULT_HOST: &str = "github.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSource {
    Flag,
    Env,
    Default,
}

#[derive(Debug, Clone)]
pub struct HostContext {
    pub value: String,
    pub source: HostSource,
}

/// Resolve the effective GitHub host. Priority: `--hostname` > `GH_HOST` > github.com.
pub fn resolve_host(flag_value: Option<&str>) -> String {
    if let Some(v) = flag_value {
        return v.to_string();
    }
    if let Ok(v) = std::env::var("GH_HOST") {
        if !v.is_empty() {
            return v;
        }
    }
    DEFAULT_HOST.to_string()
}

/// Escape a host so it can be embedded literally in a regex.
pub fn escape_reg_exp(value: &str) -> String {
    regex::escape(value)
}
