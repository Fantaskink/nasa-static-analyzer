use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct RulesConfig {
    pub rule_set: RuleSet,
}

#[derive(Deserialize)]
pub struct RuleSet {
    //pub fixed_loop_bounds: bool,
    pub restrict_goto: bool,
    pub restrict_setjmp: bool,
    pub restrict_longjmp: bool,
    pub restrict_recursion: bool,
}

pub fn load_ruleset(file_path: &str) -> RuleSet {
    let file_content = fs::read_to_string(file_path).expect("Failed to read config file");
    let config: RulesConfig = toml::from_str(&file_content).expect("Failed to parse config file");
    config.rule_set
}
