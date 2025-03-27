extern crate lang_c;

use lang_c::driver::{Config, parse}; 
use lang_c::print::Printer;
use lang_c::visit::Visit;
use lang_c::span::Span;
use lang_c::visit::visit_statement;

mod config;
use config::load_ruleset;

struct StaticAnalyzer {
    disallow_goto: bool,
}

impl StaticAnalyzer {
    fn new(disallow_goto: bool) -> Self {
        StaticAnalyzer { disallow_goto }
    }

    fn check_goto(&self, statement: &lang_c::ast::Statement, span: &Span) {
        if let lang_c::ast::Statement::Goto(_) = statement {
            println!("Error: goto statement found at {:?}", span);
        }
    }
}

impl<'ast> Visit<'ast> for StaticAnalyzer {
    fn visit_statement(&mut self, statement: &'ast lang_c::ast::Statement, span: &'ast Span) {

        if self.disallow_goto {
            self.check_goto(statement, span);
        }

        visit_statement(self, statement, span);
    }
}

fn main() {
    let rules = load_ruleset("ruleset.toml");
    println!("Restrict goto: {}", rules.restrict_goto);
    let config = Config::default();
    let Ok(ast) = parse(&config, "example.c") else {
        panic!("Failed to parse the input file");
    };

    let mut buf = String::new();
    let mut printer = Printer::new(&mut buf);
    printer.visit_translation_unit(&ast.unit);

    println!("{}", buf);

    let mut analyzer = StaticAnalyzer::new(true);
    analyzer.visit_translation_unit(&ast.unit);
}