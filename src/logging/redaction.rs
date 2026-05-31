pub fn snippet(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push('…');
    }
    out
}

#[cfg(test)]
pub fn redact_secret(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        "***".to_string()
    } else {
        let start = chars.iter().take(4).collect::<String>();
        let end = chars.iter().skip(chars.len() - 4).collect::<String>();
        format!("{start}***{end}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_limits_chars() {
        assert_eq!(snippet("abcdef", 3), "abc…");
        assert_eq!(snippet("abc", 3), "abc");
    }

    #[test]
    fn redacts_short_and_long_secrets() {
        assert_eq!(redact_secret("short"), "***");
        assert_eq!(redact_secret("abcdefghijkl"), "abcd***ijkl");
    }
}
