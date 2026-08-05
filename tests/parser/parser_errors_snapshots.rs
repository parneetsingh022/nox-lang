use crate::common::{make_lexer, render_diagnostic};
use nyx_parser::Parser;

fn snapshot_parser_errors(source: &str) -> String {
    let (mut lexer, source_file) = make_lexer(source);

    // We panic here if lexing fails to ensure parser tests only deal with valid tokens.
    let tokens: Vec<_> = lexer.by_ref().collect::<Result<Vec<_>, _>>().expect(
        "expected lexing to succeed in parser error tests, but it encountered a fatal error",
    );

    assert!(
        lexer.take_errors().is_empty(),
        "expected no non-fatal lexer errors in parser tests"
    );

    let mut parser = Parser::new(&tokens, &lexer.symbol_registry, source_file);

    let parse_result = parser.parse_expr();

    let error = match parse_result {
        Ok(_) => panic!("expected a parser error, but parsing succeeded"),
        Err(err) => err,
    };

    render_diagnostic(&error)
}

#[test]
fn report_unexpected_token() {
    // Two operators in a row
    let source = "10 + * 5";
    insta::assert_snapshot!(snapshot_parser_errors(source));
}

#[test]
fn report_unclosed_parenthesis() {
    // Missing closing parenthesis
    let source = "(10 + 20";
    insta::assert_snapshot!(snapshot_parser_errors(source));
}

#[test]
fn report_unclosed_parenthesis_with_multiple() {
    // Missing closing parenthesis
    let source = "(10 + 20 + 32 * (75 - 28 * (8-2))";
    insta::assert_snapshot!(snapshot_parser_errors(source));
}

#[test]
fn report_unexpected_eof() {
    // Trailing operator with no right-hand operand
    let source = "100 +";
    insta::assert_snapshot!(snapshot_parser_errors(source));
}

#[test]
fn report_missing_comma_in_function_call() {
    let source = "my_func(10, name, age 11)";
    insta::assert_snapshot!(snapshot_parser_errors(source));
}

#[test]
fn report_missing_parenthesis_in_function_call() {
    let source = "my_func(10, name, age";
    insta::assert_snapshot!(snapshot_parser_errors(source));
}
