extern crate lang_c;

use lang_c::driver::{parse, Config};
use lang_c::loc::get_location_for_offset;
use lang_c::print::Printer;
use lang_c::span::Span;
use lang_c::visit::Visit;
use lang_c::visit::{
    visit_call_expression, visit_function_definition, visit_statement, visit_while_statement,
};

mod config;
use config::load_ruleset;
use config::RuleSet;

struct StaticAnalyzer {
    rule_set: RuleSet,                // Configuration for the static analyzer
    source: String,                   // Source code of the program being analyzed
    current_function: Option<String>, // Name of the current function being analyzed for recursion
    analyzing_loop: bool, // Flag to indicate if the analyzer is currently analyzing a loop for bounds
}

impl StaticAnalyzer {
    fn new(rule_set: RuleSet, source: String) -> Self {
        StaticAnalyzer {
            rule_set,
            source,
            current_function: None,
            analyzing_loop: false,
        }
    }

    fn check_goto(&self, statement: &lang_c::ast::Statement, span: &Span) {
        if let lang_c::ast::Statement::Goto(_) = statement {
            println!("Error: goto statement found at {:?}", span);
        }
    }

    fn check_setjmp(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            if identifier.node.name == "setjmp" {
                println!("Error: setjmp found at {:?}", span);
            }
        }
    }

    fn check_longjmp(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            if identifier.node.name == "longjmp" {
                println!("Error: longjmp found at {:?}", span);
            }
        }
    }

    fn set_current_function(&mut self, function_definition: &lang_c::ast::FunctionDefinition) {
        let declarator = &function_definition.declarator.node.kind.node;
        if let lang_c::ast::DeclaratorKind::Identifier(identifier) = declarator {
            self.current_function = Some(identifier.node.name.clone());
        }
    }

    fn check_recursion(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            if let Some(current_function) = &self.current_function {
                if identifier.node.name == *current_function {
                    println!("Error: Recursive call found at {:?}", span);
                }
            }
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
            self.set_current_function(function_definition);
        }

        if self.rule_set.restrict_function_size {
            let start_line = get_location_for_offset(&self.source, span.start).0.line;
            let end_line = get_location_for_offset(&self.source, span.end).0.line;
            let size = end_line - start_line + 1;

            if size > 50 {
                println!("Error: Function size exceeds 50 lines at {:?}", span);
            }
        }

        visit_function_definition(self, function_definition, span);
    }

    fn visit_call_expression(
        &mut self,
        call_expression: &'ast lang_c::ast::CallExpression,
        span: &'ast Span,
    ) {
        if self.rule_set.restrict_recursion {
            self.check_recursion(call_expression, span);
        }

        if self.rule_set.restrict_setjmp {
            self.check_setjmp(call_expression, span);
        }

        if self.rule_set.restrict_longjmp {
            self.check_longjmp(call_expression, span);
        }

        visit_call_expression(self, call_expression, span);
    }

    fn visit_while_statement(
        &mut self,
        while_statement: &'ast lang_c::ast::WhileStatement,
        span: &'ast Span,
    ) {
        if self.rule_set.fixed_loop_bounds {
            self.analyzing_loop = true;
            todo!("Implement loop bounds checking");
        }

        self.analyzing_loop = false;

        visit_while_statement(self, while_statement, span);
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

    let rule_set = load_ruleset("ruleset.toml");
    let source = ast.source;

    let mut analyzer = StaticAnalyzer::new(rule_set, source);
    analyzer.visit_translation_unit(&ast.unit);
}
