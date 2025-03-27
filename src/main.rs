extern crate lang_c;

use lang_c::driver::{parse, Config};
use lang_c::loc::get_location_for_offset;
use lang_c::print::Printer;
use lang_c::span::Span;
use lang_c::visit::Visit;
use lang_c::visit::{
    visit_binary_operator_expression, visit_block_item, visit_call_expression,
    visit_function_definition, visit_statement, visit_while_statement,
};

mod config;
use config::load_ruleset;
use config::RuleSet;

#[derive(Debug)]
struct StaticAnalyzer {
    rule_set: RuleSet,                // Configuration for the static analyzer
    source: String,                   // Source code of the program being analyzed
    current_function: Option<String>, // Name of the current function being analyzed for recursion
}

impl StaticAnalyzer {
    fn new(rule_set: RuleSet, source: String) -> Self {
        StaticAnalyzer {
            rule_set,
            source,
            current_function: None,
        }
    }

    // Helper function to get the line number for a given offset in the source code
    fn get_line_number(&self, span_point: usize) -> usize {
        get_location_for_offset(&self.source, span_point).0.line
    }

    fn check_goto(&self, statement: &lang_c::ast::Statement, span: &Span) {
        if let lang_c::ast::Statement::Goto(_) = statement {
            let line_number = self.get_line_number(span.start);
            println!("Error: 'goto' statement found at line {}", line_number);
        }
    }

    fn check_setjmp(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            if identifier.node.name == "setjmp" {
                let line_number = self.get_line_number(span.start);
                println!("Error: 'setjmp' call found at line {}", line_number);
            }
        }
    }

    fn check_longjmp(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            if identifier.node.name == "longjmp" {
                let line_number = self.get_line_number(span.start);
                println!("Error: 'longjmp' call found at line {}", line_number);
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
                    let line_number = self.get_line_number(span.start);
                    println!("Error: Recursion found at line {}", line_number);
                }
            }
        }
    }

    fn check_heap_usage(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        // Check for usage of malloc, calloc, realloc, etc.
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            let heap_functions = ["malloc", "calloc", "realloc", "free"];
            if heap_functions.contains(&identifier.node.name.as_str()) {
                let line_number = self.get_line_number(span.start);
                println!("Error: Heap usage found at line {}", line_number);
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
            let start_line = self.get_line_number(span.start);
            let end_line = self.get_line_number(span.end);
            let size = end_line - start_line + 1;

            if size > 60 {
                println!(
                    "Error: Function size exceeds 60 lines at line {}",
                    start_line
                );
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

        if self.rule_set.restrict_heap_allocation {
            self.check_heap_usage(call_expression, span);
        }

        if self.rule_set.check_return_value {
            /*
            This will require a symbol table to check the return type of the function.
            The parser can seemingly handle #include directives, so we can use that to
            include the header files and get the return type of functions such as printf
            */
            todo!("Implement return value checking");
        }

        visit_call_expression(self, call_expression, span);
    }

    fn visit_while_statement(
        &mut self,
        while_statement: &'ast lang_c::ast::WhileStatement,
        span: &'ast Span,
    ) {
        let mut has_fixed_bound = false;

        if let lang_c::ast::Expression::BinaryOperator(binary_operator_expression) = &while_statement.expression.node {
            if let lang_c::ast::BinaryOperator::Less =
                binary_operator_expression.node.operator.node
            {
                has_fixed_bound = true;
            }
        }

        if self.rule_set.fixed_loop_bounds && !has_fixed_bound {
            let line_number = self.get_line_number(span.start);
            println!("Error: Loop at line {} does not have fixed bounds", line_number);
        }

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
