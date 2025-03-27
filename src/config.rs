use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct RulesConfig {
    pub rules: Rules,
}

#[derive(Deserialize)]
pub struct Rules {
    pub restrict_goto: bool,
}

pub fn load_ruleset(file_path: &str) -> Rules {
    let file_content = fs::read_to_string(file_path).expect("Failed to read config file");
    let config: RulesConfig = toml::from_str(&file_content).expect("Failed to parse config file");
    config.rules
}