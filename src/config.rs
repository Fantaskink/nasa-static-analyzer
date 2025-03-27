use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct RulesConfig {
    pub rule_set: RuleSet,
}

#[derive(Debug, Deserialize)]
pub struct RuleSet {
    // Avoid complex flow constructs
    pub restrict_goto: bool,
    pub restrict_setjmp: bool,
    pub restrict_longjmp: bool,
    pub restrict_recursion: bool,

    // Enforce loop bounds
    pub fixed_loop_bounds: bool,

    // Restrict heap allocation, e.g. malloc
    pub restrict_heap_allocation: bool,

    // Restrict function size
    pub restrict_function_size: bool,

    // Check return value of functions
    pub check_return_value: bool,
}

pub fn load_ruleset(file_path: &str) -> RuleSet {
    let file_content = fs::read_to_string(file_path).expect("Failed to read config file");
    let config: RulesConfig = toml::from_str(&file_content).expect("Failed to parse config file");
    config.rule_set
}
