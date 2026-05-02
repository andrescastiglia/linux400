/// Truncate text to a display-width budget using an ASCII ellipsis.
pub fn ellipsize(value: impl AsRef<str>, max_chars: usize) -> String {
    let value = value.as_ref();
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    format!(
        "{}...",
        value
            .chars()
            .take(max_chars.saturating_sub(3))
            .collect::<String>()
    )
}

pub fn sanitize_runtime_message(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            !lower.contains("stack backtrace") && !lower.starts_with("   ")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipsize_uses_ascii_ellipsis() {
        assert_eq!(ellipsize("ABCDEFGHIJ", 6), "ABC...");
        assert_eq!(ellipsize("ABC", 6), "ABC");
    }

    #[test]
    fn runtime_message_sanitizer_removes_backtrace_lines() {
        let message = "CPF9898 failed\nstack backtrace:\n   0: frame";
        assert_eq!(sanitize_runtime_message(message), "CPF9898 failed");
    }
}
