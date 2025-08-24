use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptType {
    TextInput { 
        prompt: String,
        default: Option<String>,
    },
    Confirmation {
        prompt: String,
        default: Option<bool>,
    },
    MultiSelect {
        prompt: String,
        options: Vec<String>,
        selected: Vec<usize>,
    },
    SingleSelect {
        prompt: String,
        options: Vec<String>,
        default: Option<usize>,
    },
    FilePath {
        prompt: String,
        default: Option<String>,
    },
}

pub struct PromptDetector {
    patterns: Vec<(Regex, fn(&str) -> Option<PromptType>)>,
}

impl PromptDetector {
    pub fn new() -> Self {
        let patterns = vec![
            (
                Regex::new(r"(?i)(enter|input|provide|type).*:[\s]*$").unwrap(),
                detect_text_input as fn(&str) -> Option<PromptType>,
            ),
            (
                Regex::new(r"(?i)\[y/n\]|continue\?|proceed\?|confirm\?").unwrap(),
                detect_confirmation as fn(&str) -> Option<PromptType>,
            ),
            (
                Regex::new(r"(?i)select.*:[\s]*$|choose.*:[\s]*$").unwrap(),
                detect_selection as fn(&str) -> Option<PromptType>,
            ),
            (
                Regex::new(r"(?i)(path|file|directory|folder).*:[\s]*$").unwrap(),
                detect_file_path as fn(&str) -> Option<PromptType>,
            ),
        ];
        
        PromptDetector { patterns }
    }
    
    pub fn detect(&self, output: &str) -> Option<PromptType> {
        let clean_output = strip_ansi_codes(output);
        
        for (pattern, detector) in &self.patterns {
            if pattern.is_match(&clean_output) {
                if let Some(prompt_type) = detector(&clean_output) {
                    return Some(prompt_type);
                }
            }
        }
        
        None
    }
}

fn strip_ansi_codes(text: &str) -> String {
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    ansi_regex.replace_all(text, "").to_string()
}

fn detect_text_input(text: &str) -> Option<PromptType> {
    Some(PromptType::TextInput {
        prompt: text.trim().to_string(),
        default: None,
    })
}

fn detect_confirmation(text: &str) -> Option<PromptType> {
    let default = if text.contains("[Y/n]") {
        Some(true)
    } else if text.contains("[y/N]") {
        Some(false)
    } else {
        None
    };
    
    Some(PromptType::Confirmation {
        prompt: text.trim().to_string(),
        default,
    })
}

fn detect_selection(text: &str) -> Option<PromptType> {
    let lines: Vec<&str> = text.lines().collect();
    let mut options = Vec::new();
    
    for line in lines.iter().rev() {
        if line.trim().starts_with('[') || line.trim().starts_with('•') || 
           line.trim().starts_with('-') || line.trim().starts_with(char::is_numeric) {
            options.push(line.trim().to_string());
        }
    }
    
    if !options.is_empty() {
        options.reverse();
        Some(PromptType::SingleSelect {
            prompt: text.trim().to_string(),
            options,
            default: None,
        })
    } else {
        None
    }
}

fn detect_file_path(text: &str) -> Option<PromptType> {
    Some(PromptType::FilePath {
        prompt: text.trim().to_string(),
        default: None,
    })
}