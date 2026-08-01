use nox_diagnostic::ParserError;
use nox_source::Span;
use nox_token::{Keyword, Token, TokenKind};

use crate::parser::{
    Parser,
    ast::{Stmt, StmtKind},
    expression::is_expr_start,
};

impl<'a> Parser<'a> {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
        let token = self.peek().ok_or_else(|| self.unexpected_eof_error())?;

        match token.kind {
            TokenKind::Keyword(_) => self.parse_keyword_stmt(token),
            _ if is_expr_start(token) => self.parse_expr_stmt(),
            _ => Err(self.expected_statement_error(token)),
        }
    }

    fn parse_keyword_stmt(&mut self, token: Token) -> Result<Stmt, ParserError> {
        match token.kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_let_stmt(),
            TokenKind::Keyword(_) => Err(self.expected_statement_error(token)),
            _ => unreachable!("non-keyword token passed to parse_keyword_stmt"),
        }
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, ParserError> {
        let expr = self.parse_expr()?;
        let span = expr.span();

        self.ensure_no_adjacent_expression(expr.span())?;
        self.expect_semicolon()?;

        if !expr.is_valid_expr_statement() {
            return Err(ParserError::InvalidExpressionStatement {
                at: span.into(),
                src: self.source_file.clone(),
            });
        }

        Ok(Stmt::new(StmtKind::ExprStmt { expr }, span))
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParserError> {
        // Consume the keyword
        let start = self.expect(TokenKind::Keyword(Keyword::Let))?.span;
        let identifier = self.expect_identifier()?;
        self.expect(TokenKind::Eq)?;

        let expr = self.parse_expr()?;

        self.ensure_no_adjacent_expression(expr.span())?;

        let semi = self.expect_semicolon()?;

        let span = Span::from_bounds(start, semi.span);

        Ok(Stmt::new(
            StmtKind::Let {
                name: identifier,
                expr,
            },
            span,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nox_token::SymbolRegistry;
    use rstest::rstest;

    use crate::{
        lexer::make_lexer,
        parser::{
            ast::{BinaryOp, Expr, ExprKind, SpannedIdentifier, UnaryOp},
            expression::tests::{binary, boolean, float, identifier, int, unary},
        },
    };

    /// This assumes a statement to have [`StatementKind::Let`] otherwise
    /// it panics
    fn as_let(stmt: &Stmt) -> (&SpannedIdentifier, &Expr) {
        #[allow(irrefutable_let_patterns)]
        let StmtKind::Let { name, expr } = stmt.kind() else {
            panic!("Expected let found : {:?}", stmt);
        };
        (name, expr)
    }

    fn try_parse_statement(source: &str) -> Result<(Stmt, SymbolRegistry), ParserError> {
        let (mut lexer, source_file) = make_lexer(source);

        let tokens = lexer
            .by_ref()
            .collect::<Result<Vec<_>, _>>()
            .expect("expected lexing to succeed");

        let errors = lexer.take_errors();
        assert!(errors.is_empty(), "unexpected lexer errors: {errors:#?}");

        let symbol_registry = lexer.take_registry();
        let mut parser = Parser::new(&tokens, &symbol_registry, source_file);

        parser
            .parse_stmt()
            .map(|statement| (statement, symbol_registry))
    }

    fn parse_statement(source: &str) -> (Stmt, SymbolRegistry) {
        try_parse_statement(source).expect("expected statement to parse")
    }

    #[rstest]
    #[case("let x = 5 + 6;")]
    #[case("let x = foo();")]
    #[case("let x = foo() + bar();")]
    #[case("let x = foo()(1);")]
    #[case("foo();")]
    #[case("foo(bar(), baz());")]
    #[case("x = 5;")]
    fn does_not_report_missing_operator_for_valid_expressions(#[case] source: &str) {
        try_parse_statement(source).unwrap_or_else(|error| {
            panic!("expected `{source}` to parse successfully, found: {error:?}")
        });
    }

    #[rstest]
    #[case("let x = 5 \n + 6;")] // Line break before operator
    #[case("let x = 10 + \n 20;")] // Line break after operator
    #[case("let x = \n foo();")] // Line break after assignment
    #[case("foo(\n bar(), \n baz() \n);")] // Arguments on separate lines
    #[case("let x = \n ( \n 5 + 6 \n );")] // Line breaks inside grouping parentheses
    #[case("x \n = \n 5;")] // Highly fragmented assignment
    #[case("let x = foo() \n + \n bar();")] // Line breaks surrounding an operator
    fn does_not_report_missing_operator_for_multiline_expressions(#[case] source: &str) {
        try_parse_statement(source).unwrap_or_else(|error| {
            panic!(
                "expected multiline expression `{source}` to parse successfully, found: {error:?}"
            )
        });
    }
    #[rstest]
    #[case("x", "42", int(42))]
    #[case("_underscore_variable", "204.2101", float(204.2101))]
    #[case("___boolean_true", "true", boolean(true))]
    #[case("___boolean_false___293", "false", boolean(false))]
    #[case("unary_variable", "-39", unary(UnaryOp::Minus, int(39)))]
    #[case("unary_bool___", "!false", unary(UnaryOp::Not, boolean(false)))]
    fn parse_let_statement(
        #[case] variable_name: &str,
        #[case] value: &str,
        #[case] expected: Expr,
    ) {
        let source = format!("let {variable_name} = {value};");
        let (stmt, symbol_registry) = parse_statement(source.as_str());

        let (name, expr) = as_let(&stmt);

        assert_eq!(variable_name, symbol_registry.resolve(name.symbol()));
        assert_eq!(expr, &expected);
    }

    #[rstest]
    #[case("x", "y")]
    #[case("result", "_input")]
    #[case("same", "same")]
    fn parse_let_statement_with_identifier_value(#[case] variable_name: &str, #[case] value: &str) {
        let source = format!("let {variable_name} = {value};");
        let (stmt, symbol_registry) = parse_statement(&source);
        let (name, expr) = as_let(&stmt);

        assert_eq!(variable_name, symbol_registry.resolve(name.symbol()));

        let ExprKind::Identifier(symbol) = expr.kind() else {
            panic!("expected identifier, found: {:?}", expr.kind());
        };

        assert_eq!(value, symbol_registry.resolve(*symbol));
    }

    #[test]
    fn parses_mixed_expression_in_let_statement() {
        let source = "let result = -(value + 42) * !false;";
        let (stmt, mut symbol_registry) = parse_statement(source);

        let value_symbol = symbol_registry.store("value");

        let expected = binary(
            unary(
                UnaryOp::Minus,
                binary(identifier(value_symbol), BinaryOp::Plus, int(42)),
            ),
            BinaryOp::Multiply,
            unary(UnaryOp::Not, boolean(false)),
        );

        let (name, expr) = as_let(&stmt);
        assert_eq!("result", symbol_registry.resolve(name.symbol()));
        assert_eq!(&expected, expr);
    }

    #[rstest]
    #[case("let = 42;")]
    #[case("let 42 = 42;")]
    #[case("let x 42;")]
    #[case("let x = ;")]
    #[case("let x = 42")]
    fn rejects_incomplete_let_statements(#[case] source: &str) {
        assert!(
            try_parse_statement(source).is_err(),
            "expected parsing to fail for `{source}`"
        );
    }

    #[test]
    fn rejects_empty_input() {
        let error = try_parse_statement("").err().unwrap();

        assert!(
            matches!(error, ParserError::UnexpectedEof { .. }),
            "expected unexpected EOF, found: {error:?}"
        );
    }

    #[rstest]
    #[case("foo();")]
    #[case("foo(1, 2);")]
    #[case("x = 42;")]
    #[case("x = y = 10;")]
    fn parses_valid_expression_statements(#[case] source: &str) {
        let (stmt, reg) = try_parse_statement(source).unwrap_or_else(|err| {
            panic!("expected valid statement for `{source}`, found error: {err:?}")
        });

        assert!(
            matches!(stmt.kind(), StmtKind::ExprStmt { .. }),
            "expected ExprStmt for `{source}`, found: {:?}",
            stmt.debug_with(&reg)
        );
    }

    #[test]
    fn let_statement_span_covers_entire_statement() {
        let source = "let result = 42;";
        let (stmt, _) = parse_statement(source);

        assert_eq!(stmt.span(), Span::single_line(0, source.len(), 1, 1, 17));
    }

    #[test]
    fn let_statement_span_starts_at_let_keyword() {
        let source = "\n  let result = 42;";
        let (stmt, _) = parse_statement(source);

        assert_eq!(stmt.span(), Span::single_line(3, source.len(), 2, 3, 19));
    }

    #[test]
    fn let_declaration_identifier_has_correct_span() {
        let source = "let result = 42;";
        let (stmt, _) = parse_statement(source);

        let (name, _) = as_let(&stmt);

        assert_eq!(name.span(), Span::single_line(4, 10, 1, 5, 11));
    }

    #[test]
    fn let_declaration_identifier_tracks_line_and_column() {
        let source = "\n  let long_name = 42;";
        let (stmt, _) = parse_statement(source);

        let (name, _) = as_let(&stmt);

        assert_eq!(name.span(), Span::single_line(7, 16, 2, 7, 16));
    }

    #[test]
    fn declaration_and_expression_identifiers_have_independent_spans() {
        let source = "let destination = source;";
        let (stmt, symbol_registry) = parse_statement(source);

        let (name, expr) = as_let(&stmt);

        assert_eq!("destination", symbol_registry.resolve(name.symbol()));
        assert_eq!(name.span(), Span::single_line(4, 15, 1, 5, 16));

        let ExprKind::Identifier(symbol) = expr.kind() else {
            panic!("expected identifier expression, found: {:?}", expr.kind());
        };

        assert_eq!("source", symbol_registry.resolve(*symbol));
        assert_eq!(expr.span(), Span::single_line(18, 24, 1, 19, 25));
    }

    #[rstest]
    // Mid-file missing semicolon (followed by a newline and next statement)
    #[case("let x = 10\nlet y = 20;")]
    #[case("foo(92, 49, age)\nlet y = 20;")]
    #[case("foo()\nlet y = 20;")]
    #[case("x = 10\nlet y = 20;")]
    // End-of-file missing semicolon
    #[case("let x = 10")]
    #[case("foo()")]
    #[case("foo(92, 49, age)")]
    #[case("x = 10")]
    #[case("x = 10 * 39 - 19 + 29")]
    fn reports_missing_semicolon_error(#[case] source: &str) {
        let error = try_parse_statement(source)
            .err()
            .unwrap_or_else(|| panic!("expected parsing to fail for `{source}`, but it succeeded"));

        assert!(
            matches!(error, ParserError::ExpectedSemicolon { .. }),
            "expected ExpectedSemicolonError for `{source}`, found: {error:?}"
        );
    }

    #[rstest]
    #[case("let x = 5 6;")]
    #[case("let x = value other;")]
    #[case("let x = foo() bar();")]
    #[case("let x = (1 + 2) 3;")]
    #[case("let x = 5 true;")]
    #[case("let x = -5 value;")]
    fn reports_missing_operator_between_adjacent_initializer_expressions(#[case] source: &str) {
        let error = try_parse_statement(source).err().unwrap_or_else(|| {
            panic!("expected adjacent expressions to be rejected for `{source}`")
        });

        assert!(
            matches!(error, ParserError::MissingOperator { .. }),
            "expected MissingOperatorError for `{source}`, found: {error:?}"
        );
    }

    #[rstest]
    #[case("foo() bar();")]
    #[case("x = 5 6;")]
    #[case("x = value other;")]
    #[case("x = foo() bar();")]
    #[case("y = (14 * 25) \n - 900 105;")]
    #[case("y = (14 * 25) - \n 900 105;")]
    fn reports_missing_operator_in_expression_statements(#[case] source: &str) {
        let error = try_parse_statement(source).err().unwrap_or_else(|| {
            panic!("expected adjacent expressions to be rejected for `{source}`")
        });

        assert!(
            matches!(error, ParserError::MissingOperator { .. }),
            "expected MissingOperatorError for `{source}`, found: {error:?}"
        );
    }

    #[rstest]
    #[case("let x = 5\nfoo();")]
    #[case("let x = value\nother();")]
    #[case("let x = foo()\nbar();")]
    #[case("let x = (1 + 2)\nfoo();")]
    #[case("foo()\nbar();")]
    #[case("x = 5\nfoo();")]
    fn reports_missing_semicolon_before_expression_on_next_line(#[case] source: &str) {
        let error = try_parse_statement(source)
            .err()
            .unwrap_or_else(|| panic!("expected a missing semicolon error for `{source}`"));

        assert!(
            matches!(error, ParserError::ExpectedSemicolon { .. }),
            "expected ExpectedSemicolonError for `{source}`, found: {error:?}"
        );
    }

    #[rstest]
    #[case("let x = 5\n\nfoo();")]
    #[case("foo()\n\nbar();")]
    #[case("x = 5\n\n\nfoo();")]
    fn reports_missing_semicolon_across_blank_lines(#[case] source: &str) {
        let error = try_parse_statement(source)
            .err()
            .unwrap_or_else(|| panic!("expected a missing semicolon error for `{source}`"));

        assert!(
            matches!(error, ParserError::ExpectedSemicolon { .. }),
            "expected ExpectedSemicolonError for `{source}`, found: {error:?}"
        );
    }

    #[rstest]
    #[case("let x = 5    6;")]
    #[case("let x = foo()\tbar();")]
    fn whitespace_does_not_hide_adjacent_expressions(#[case] source: &str) {
        let error = try_parse_statement(source).err().unwrap_or_else(|| {
            panic!("expected adjacent expressions to be rejected for `{source}`")
        });

        assert!(
            matches!(error, ParserError::MissingOperator { .. }),
            "expected MissingOperatorError for `{source}`, found: {error:?}"
        );
    }
}
