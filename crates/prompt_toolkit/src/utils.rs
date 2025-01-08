// Utility functions for prompt processing
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref VARIABLE_REGEX: Regex = Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap();
}

pub fn extract_variables(template: &str) -> Vec<String> {
    VARIABLE_REGEX
        .captures_iter(template)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variables() {
        let template = "Hello {{name}}, today is {{day}}!";
        let vars = extract_variables(template);
        assert_eq!(vars, vec!["name", "day"]);
    }
}
