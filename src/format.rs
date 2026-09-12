//! Shared count-line formatting, mirroring `src/format.ts`.

pub struct CountLineOptions {
    pub count: usize,
    pub limit: Option<usize>,
    pub total_count: Option<usize>,
    pub api_limit_hit: bool,
    pub display_limit: Option<usize>,
}

pub fn format_count_line(opts: &CountLineOptions) -> String {
    if opts.api_limit_hit {
        return format!("count: {}+ (GitHub search API limit reached)", opts.count);
    }
    if let Some(total) = opts.total_count {
        if total >= opts.count {
            return format!("count: {} of {} total", opts.count, total);
        }
    }
    if let Some(display) = opts.display_limit {
        if opts.count > display {
            return format!("count: {} (showing first {})", opts.count, display);
        }
    }
    if let Some(limit) = opts.limit {
        if opts.count == limit && opts.count > 0 {
            return format!("count: {} (showing first {})", opts.count, opts.count);
        }
    }
    format!("count: {}", opts.count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(count: usize) -> CountLineOptions {
        CountLineOptions {
            count,
            limit: None,
            total_count: None,
            api_limit_hit: false,
            display_limit: None,
        }
    }

    #[test]
    fn simple_count() {
        assert_eq!(format_count_line(&opts(3)), "count: 3");
    }

    #[test]
    fn count_at_limit_shows_truncation() {
        let mut o = opts(500);
        o.limit = Some(500);
        assert_eq!(format_count_line(&o), "count: 500 (showing first 500)");
    }

    #[test]
    fn count_with_total() {
        let mut o = opts(3);
        o.total_count = Some(10);
        assert_eq!(format_count_line(&o), "count: 3 of 10 total");
    }

    #[test]
    fn api_limit_hit() {
        let mut o = opts(1000);
        o.api_limit_hit = true;
        assert_eq!(
            format_count_line(&o),
            "count: 1000+ (GitHub search API limit reached)"
        );
    }
}
