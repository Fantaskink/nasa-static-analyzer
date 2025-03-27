extern crate lang_c;

use std::collections::HashMap;

use lang_c::driver::{parse, Config};
use lang_c::loc::get_location_for_offset;
use lang_c::print::Printer;
use lang_c::span::Span;
use lang_c::visit::Visit;
use lang_c::visit::{
    visit_call_expression, visit_cast_expression, visit_declaration, visit_expression,
    visit_function_definition, visit_statement, visit_while_statement,
};

mod config;
use config::load_ruleset;
use config::RuleSet;

#[derive(Debug)]
enum SymbolType {
    Function {
        return_type: lang_c::ast::TypeSpecifier,
    },
    Variable,
}

#[derive(Debug)]
struct Symbol {
    name: String,
    symbol_type: SymbolType,
}

#[derive(Debug)]
struct StaticAnalyzer {
    rule_set: RuleSet,                     // Configuration for the static analyzer
    symbol_table: HashMap<String, Symbol>, // Symbol table to store the types of variables
    current_function_type_cast: Option<lang_c::ast::TypeSpecifier>, // Type of the current function being analyzed, is None if not cast
    source: String,                   // Source code of the program being analyzed
    current_function: Option<String>, // Name of the current function being analyzed for recursion
}

impl StaticAnalyzer {
    fn new(rule_set: RuleSet, source: String) -> Self {
        StaticAnalyzer {
            rule_set,
            symbol_table: HashMap::new(),
            current_function_type_cast: None,
            source,
            current_function: None,
        }
    }

    // Helper function to get the line number for a given offset in the source code
    fn get_line_number(&self, span_point: usize) -> usize {
        get_location_for_offset(&self.source, span_point).0.line
    }

    fn get_source_code_from_span(&self, span: &Span) -> String {
        let source_line = &self.source[span.start..span.end];
        let squiggles = "^".repeat(span.end - span.start); // Create squiggles for the span length
        format!("{}\n{}", source_line, squiggles)
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

    fn check_while_loop_bounds(&self, while_statement: &lang_c::ast::WhileStatement, span: &Span) {
        if let lang_c::ast::Expression::BinaryOperator(binary_operator_expression) =
            &while_statement.expression.node
        {
            match binary_operator_expression.node.operator.node {
                lang_c::ast::BinaryOperator::Less
                | lang_c::ast::BinaryOperator::LessOrEqual
                | lang_c::ast::BinaryOperator::Greater
                | lang_c::ast::BinaryOperator::GreaterOrEqual
                | lang_c::ast::BinaryOperator::Equals => {
                    // Check if one side of the condition is a constant
                    if matches!(
                        binary_operator_expression.node.lhs.node,
                        lang_c::ast::Expression::Constant(_)
                    ) || matches!(
                        binary_operator_expression.node.rhs.node,
                        lang_c::ast::Expression::Constant(_)
                    ) {
                        return;
                    }
                }
                _ => {}
            }
        }

        let line_number = self.get_line_number(span.start);
        println!(
            "Error: Loop at line {} does not have fixed bounds",
            line_number
        );
    }

    fn check_heap_usage(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            let heap_functions = ["malloc", "calloc", "realloc", "free"];
            if heap_functions.contains(&identifier.node.name.as_str()) {
                let line_number = self.get_line_number(span.start);
                println!("Error: Heap usage found at line {}", line_number);
                println!("{}", self.get_source_code_from_span(span));
            }
        }
    }

    fn add_function_to_symbol_table(
        &mut self,
        declaration: &lang_c::ast::Declaration,
        span: &Span,
    ) {
        // If DerivedDeclarator KRFunction is present, then it is a function declaration
        let Some(init_declarator) = declaration.declarators.first() else {
            visit_declaration(self, declaration, span);
            return;
        };

        let derived = &init_declarator.node.declarator.node.derived.first();

        if let Some(lang_c::span::Node {
            node: lang_c::ast::DerivedDeclarator::KRFunction(_),
            span: _,
        }) = derived
        {
            if let lang_c::ast::DeclaratorKind::Identifier(identifier) =
                &init_declarator.node.declarator.node.kind.node
            {
                // Extract the return type of the function
                let return_type = match &declaration.specifiers[..] {
                    [lang_c::span::Node {
                        node: lang_c::ast::DeclarationSpecifier::TypeSpecifier(type_specifier),
                        ..
                    }] => {
                        type_specifier.node.clone() // Clone the TypeSpecifier for storage
                    }
                    _ => lang_c::ast::TypeSpecifier::Void, // Default to void if unknown
                };

                // Insert the function into the symbol table with its return type
                self.symbol_table.insert(
                    identifier.node.name.clone(),
                    Symbol {
                        name: identifier.node.name.clone(),
                        symbol_type: SymbolType::Function { return_type },
                    },
                );
            }
        }
    }

    fn check_return_value(&self, call_expression: &lang_c::ast::CallExpression, span: &Span) {
        if let lang_c::ast::Expression::Identifier(identifier) = &call_expression.callee.node {
            if let Some(symbol) = self.symbol_table.get(&identifier.node.name) {
                if let SymbolType::Function { return_type } = &symbol.symbol_type {
                    // Check if the return type is not void
                    if *return_type != lang_c::ast::TypeSpecifier::Void {
                        // Ensure that current_function_type_cast is set
                        if self.current_function_type_cast.is_none() {
                            let line_number = self.get_line_number(span.start);
                            println!(
                                "Error: Call to non-void function at line {} does not handle the return value",
                                line_number
                            );
                            println!("{}", self.get_source_code_from_span(span));
                        }
                    }
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for StaticAnalyzer {
    fn visit_expression(&mut self, expression: &'ast lang_c::ast::Expression, span: &'ast Span) {
        visit_expression(self, expression, span);
    }

    fn visit_declaration(&mut self, declaration: &'ast lang_c::ast::Declaration, span: &'ast Span) {
        if self.rule_set.check_return_value {
            self.add_function_to_symbol_table(declaration, span);
        }
        visit_declaration(self, declaration, span);
    }
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
        if let lang_c::ast::DeclaratorKind::Identifier(identifier) =
            &function_definition.declarator.node.kind.node
        {
            // Extract the return type of the function
            let return_type = match &function_definition.specifiers[..] {
                [lang_c::span::Node {
                    node: lang_c::ast::DeclarationSpecifier::TypeSpecifier(type_specifier),
                    ..
                }] => {
                    type_specifier.node.clone() // Clone the TypeSpecifier for storage
                }
                _ => lang_c::ast::TypeSpecifier::Void, // Default to void if unknown
            };

            // Insert the function into the symbol table with its return type
            self.symbol_table.insert(
                identifier.node.name.clone(),
                Symbol {
                    name: identifier.node.name.clone(),
                    symbol_type: SymbolType::Function { return_type },
                },
            );
        }

        if self.rule_set.restrict_recursion {
            self.set_current_function(function_definition);
        }

        if self.rule_set.restrict_function_size {
            let start_line = self.get_line_number(span.start);
            let end_line = self.get_line_number(span.end);
            let size = end_line - start_line + 1;

            // TODO: This should be configurable
            if size > 60 {
                println!(
                    "Error: Function size exceeds 60 lines at line {}",
                    start_line
                );
            }
        }

        visit_function_definition(self, function_definition, span);

        self.current_function = None;
    }

    fn visit_cast_expression(
        &mut self,
        cast_expression: &'ast lang_c::ast::CastExpression,
        span: &'ast Span,
    ) {
        let Some(specifier) = cast_expression.type_name.node.specifiers.first() else {
            return;
        };

        let type_specifier: &lang_c::ast::SpecifierQualifier = &specifier.node;
        if let lang_c::ast::SpecifierQualifier::TypeSpecifier(type_specifier) = type_specifier {
            self.current_function_type_cast = Some(type_specifier.node.clone());
        }
        visit_cast_expression(self, cast_expression, span);
        self.current_function_type_cast = None;
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
            self.check_return_value(call_expression, span);
        }

        visit_call_expression(self, call_expression, span);
    }

    fn visit_while_statement(
        &mut self,
        while_statement: &'ast lang_c::ast::WhileStatement,
        span: &'ast Span,
    ) {
        if self.rule_set.fixed_loop_bounds {
            self.check_while_loop_bounds(while_statement, span);
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

    //println!("{:?}", analyzer.symbol_table);
}
