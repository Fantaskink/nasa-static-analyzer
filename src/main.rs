extern crate lang_c;

use lang_c::driver::{parse, Config};
use lang_c::print::Printer;
use lang_c::span::Span;
use lang_c::visit::Visit;
use lang_c::visit::{visit_call_expression, visit_function_definition, visit_statement};

mod config;
use config::load_ruleset;
use config::RuleSet;

struct StaticAnalyzer {
    rule_set: RuleSet,
    current_function: Option<String>, // Name of the current function being analyzed
}

impl StaticAnalyzer {
    fn new(rule_set: RuleSet) -> Self {
        StaticAnalyzer {
            rule_set,
            current_function: None,
        }
    }

    fn check_goto(&self, statement: &lang_c::ast::Statement, span: &Span) {
        if let lang_c::ast::Statement::Goto(_) = statement {
            println!("Error: goto statement found at {:?}", span);
        }
    }
}

impl<'ast> Visit<'ast> for StaticAnalyzer {
    fn visit_statement(&mut self, statement: &'ast lang_c::ast::Statement, span: &'ast Span) {
        if self.rule_set.restrict_goto {
            self.check_goto(statement, span);
        }

        visit_statement(self, statement, span);
    }

    fn visit_function_definition(
        &mut self,
        function_definition: &'ast lang_c::ast::FunctionDefinition,
        span: &'ast Span,
    ) {
        if self.rule_set.restrict_recursion {
            let declarator = &function_definition.declarator.node.kind.node;
            if let lang_c::ast::DeclaratorKind::Identifier(identifier) = declarator {
                self.current_function = Some(identifier.node.name.clone());
            }
            println!(
                "Analyzing function: {}",
                self.current_function.as_ref().unwrap()
            );
        }

        visit_function_definition(self, function_definition, span);
    }

    fn visit_call_expression(
        &mut self,
        call_expression: &'ast lang_c::ast::CallExpression,
        span: &'ast Span,
    ) {

        if self.rule_set.restrict_recursion {
            if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
                if let Some(current_function) = &self.current_function {
                    if identifier.node.name == *current_function {
                        println!("Error: Recursive call found at {:?}", span);
                    }
                }
            }
        }

        if self.rule_set.restrict_setjmp {
            if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
                if identifier.node.name == "setjmp" {
                    println!("Error: setjmp found at {:?}", span);
                }
            }
        }

        if self.rule_set.restrict_longjmp {
            if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
                if identifier.node.name == "longjmp" {
                    println!("Error: longjmp found at {:?}", span);
                }
            }
        }

        visit_call_expression(self, call_expression, span);
    }
}

fn main() {
    let config = Config::default();
    let Ok(ast) = parse(&config, "example.c") else {
        panic!("Failed to parse the input file");
    };

    let mut buf = String::new();
    let mut printer = Printer::new(&mut buf);
    printer.visit_translation_unit(&ast.unit);

    println!("{}", buf);

    let mut analyzer = StaticAnalyzer::new(load_ruleset("ruleset.toml"));
    analyzer.visit_translation_unit(&ast.unit);
}
