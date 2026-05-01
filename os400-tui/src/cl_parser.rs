//! Shared CL command tokenizer and argument extraction.
//!
//! This module provides a single source of truth for parsing CL command strings
//! into tokens and extracting keyword arguments. Previously this logic was
//! duplicated between `cmd_line.rs` and `admin_views.rs`.

/// - keyword values in parentheses: `OBJ(QGPL/DEMO)` stays as one token
/// - quoted strings with spaces: `TEXT('My text')` stays as one token
/// - nested parentheses and mixed quoting
pub fn tokenize_cl_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    for ch in command.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ch if ch.is_whitespace() && depth == 0 && !in_single && !in_double => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

/// Extract the value of a keyword argument from a list of CL tokens.
///
/// Given tokens like `["OBJ(QGPL/DEMO)", "TEXT('hello')"]` and key `"OBJ"`,
/// returns `Some("QGPL/DEMO")`.
pub fn extract_command_arg(tokens: &[String], key: &str) -> Option<String> {
    tokens.iter().find_map(|token| {
        let token = token.trim();
        if !token.to_uppercase().starts_with(&format!("{key}(")) || !token.ends_with(')') {
            return None;
        }
        Some(token[key.len() + 1..token.len() - 1].trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_command_tokenizes() {
        let tokens = tokenize_cl_command("WRKOBJ");
        assert_eq!(tokens, vec!["WRKOBJ"]);
    }

    #[test]
    fn keyword_values_are_preserved() {
        let tokens = tokenize_cl_command("CHGOBJD OBJ(QGPL/DEMO) TEXT('Demo object')");
        assert_eq!(
            tokens,
            vec![
                "CHGOBJD".to_string(),
                "OBJ(QGPL/DEMO)".to_string(),
                "TEXT('Demo object')".to_string()
            ]
        );
    }

    #[test]
    fn extract_arg_finds_keyword() {
        let tokens = tokenize_cl_command("DSPOBJD OBJ(QGPL/TEST)");
        assert_eq!(
            extract_command_arg(&tokens[1..], "OBJ").as_deref(),
            Some("QGPL/TEST")
        );
    }

    #[test]
    fn extract_arg_returns_none_for_missing_key() {
        let tokens = tokenize_cl_command("WRKOBJ");
        assert_eq!(extract_command_arg(&tokens, "OBJ"), None);
    }

    #[test]
    fn nested_parens_are_preserved() {
        let tokens = tokenize_cl_command("SBMJOB CMD(CALL PGM(HELLO)) JOB(TEST)");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1], "CMD(CALL PGM(HELLO))");
    }
}
