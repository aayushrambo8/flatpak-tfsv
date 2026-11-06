use std::path::Path;

use crate::config::Settings;

fn split_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' | '\'' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

pub fn detect_opener(path: &Path) -> Vec<String> {
    let settings = Settings::load().unwrap_or_default();
    let lower = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let command = if lower.ends_with(".md") || lower.ends_with(".txt") {
        settings.openers.text
    } else if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        settings.openers.image
    } else if lower.ends_with(".pdf") {
        settings.openers.pdf
    } else {
        settings.openers.default
    };

    split_command(&command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_files_use_editor_command() {
        let key = "EDITOR";
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, "code --wait");
        }

        let opener = detect_opener(Path::new("notes.txt"));
        assert_eq!(opener, vec!["code", "--wait"]);

        unsafe {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn split_command_keeps_nested_flags() {
        assert_eq!(split_command("code --wait"), vec!["code", "--wait"]);
    }
}
