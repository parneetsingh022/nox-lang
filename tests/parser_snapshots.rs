mod common;

use common::make_lexer;
use nox_lang::parser::Parser;

fn parse_expression_debug(source: &str) -> String {
    let mut lexer = make_lexer(source);
    let tokens = lexer.by_ref().map(|res| res.unwrap()).collect::<Vec<_>>();

    let registry = lexer.take_registry();

    let mut parser = Parser::new(&tokens, &registry);
    let expression = parser.parse_expr();

    format!("{:#?}", expression.debug_with(&registry))
}

// Helper function to consolidate the boilerplate
fn snapshot_parse(input: &str, name: &str) {
    let ast = parse_expression_debug(input);
    insta::assert_snapshot!(name, ast);
}

#[test]
fn test_literals() {
    snapshot_parse("42", "int_literal");
    snapshot_parse("3.14159", "float_literal");
    snapshot_parse("variable_name", "identifier");
}

#[test]
fn test_function_calls() {
    snapshot_parse("foo()", "simple_call");
    snapshot_parse("calculate(10, 20.5)", "call_with_args");
    snapshot_parse("nested(foo(), bar(1, 2))", "nested_calls");
}

#[test]
fn test_complex_expressions() {
    // Testing order of operations/precedence if your parser supports it
    snapshot_parse("a + b * c", "precedence_math");
    snapshot_parse("foo(x) + bar(y) * 10", "mixed_expression");
}
