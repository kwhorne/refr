/// Parses one-based page ranges ("1, 3-5", "all" or empty) into ordered zero-based indices.
/// Duplicates are removed.
pub fn parse(value: &str, page_count: usize) -> Result<Vec<usize>, String> {
    if page_count == 0 {
        return Err("The document has no pages.".into());
    }
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("all") {
        return Ok((0..page_count).collect());
    }
    let mut result = Vec::new();
    for token in value.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let mut parts = token.split('-').map(str::trim);
        let number = |part: Option<&str>| -> Result<usize, String> {
            part.filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|p| p.parse().ok())
                .ok_or_else(|| "Use page numbers and ranges, for example 1, 3-5.".to_string())
        };
        let start = number(parts.next())?;
        let end = match parts.next() {
            Some(part) => number(Some(part))?,
            None => start,
        };
        if parts.next().is_some() {
            return Err("Use page numbers and ranges, for example 1, 3-5.".into());
        }
        if start < 1 || end > page_count || end < start {
            return Err(format!("Pages must be between 1 and {page_count}; ranges must be ascending."));
        }
        for i in start..=end {
            if !result.contains(&(i - 1)) {
                result.push(i - 1);
            }
        }
    }
    if result.is_empty() {
        return Err("Select at least one page.".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_ranges() {
        assert_eq!(parse("1, 3-4, 3", 5).unwrap(), vec![0, 2, 3]);
        assert_eq!(parse("all", 3).unwrap(), vec![0, 1, 2]);
        assert!(parse("4-2", 5).is_err());
        assert!(parse("9", 5).is_err());
        assert!(parse("x", 5).is_err());
    }
}
