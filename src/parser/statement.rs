use crate::{
    diagnostic::{ParserError, Span},
    lexer::{Keyword, Token, TokenKind},
    parser::{
        Parser,
        ast::{Stmt, StmtKind},
    },
};

impl<'a> Parser<'a> {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
        let token = self.peek().ok_or_else(|| self.unexpected_eof_error())?;

        match token.kind {
            TokenKind::Keyword(_) => self.parse_keyword_stmt(token),
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

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParserError> {
        // Consume the keyword
        let start = self.expect(TokenKind::Keyword(Keyword::Let))?.span;
        let identifier = self.expect_identifier()?;
        self.expect(TokenKind::Eq)?;

        let expr = self.parse_expr()?;
        dbg!("REACHD HERE");
        let semi = self.expect_semicolon()?;
        dbg!("REACHD HERE");

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
    use rstest::rstest;

    use crate::{
        diagnostic::assert_span,
        lexer::{SymbolRegistry, make_lexer},
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
            matches!(error, ParserError::UnexpectedEof(_)),
            "expected unexpected EOF, found: {error:?}"
        );
    }

    #[rstest]
    #[case("42;")]
    #[case("value;")]
    #[case("(1 + 2);")]
    fn reports_expected_statement_error_for_expression(#[case] source: &str) {
        let error = try_parse_statement(source).err().unwrap();

        assert!(
            matches!(error, ParserError::ExpectedStatement(_)),
            "expected ExpectedStatementError for `{source}`, found: {error:?}"
        );
    }

    #[test]
    fn let_statement_span_covers_entire_statement() {
        let source = "let result = 42;";
        let (stmt, _) = parse_statement(source);

        assert_span(stmt.span(), Span::new(0, source.len(), 1, 1));
    }

    #[test]
    fn let_statement_span_starts_at_let_keyword() {
        let source = "\n  let result = 42;";
        let (stmt, _) = parse_statement(source);

        assert_span(stmt.span(), Span::new(3, source.len(), 2, 3));
    }

    #[test]
    fn let_declaration_identifier_has_correct_span() {
        let source = "let result = 42;";
        let (stmt, _) = parse_statement(source);

        let (name, _) = as_let(&stmt);

        assert_span(name.span(), Span::new(4, 10, 1, 5));
    }

    #[test]
    fn let_declaration_identifier_tracks_line_and_column() {
        let source = "\n  let long_name = 42;";
        let (stmt, _) = parse_statement(source);

        let (name, _) = as_let(&stmt);

        assert_span(name.span(), Span::new(7, 16, 2, 7));
    }

    #[test]
    fn declaration_and_expression_identifiers_have_independent_spans() {
        let source = "let destination = source;";
        let (stmt, symbol_registry) = parse_statement(source);

        let (name, expr) = as_let(&stmt);

        assert_eq!("destination", symbol_registry.resolve(name.symbol()));
        assert_span(name.span(), Span::new(4, 15, 1, 5));

        let ExprKind::Identifier(symbol) = expr.kind() else {
            panic!("expected identifier expression, found: {:?}", expr.kind());
        };

        assert_eq!("source", symbol_registry.resolve(*symbol));
        assert_span(expr.span(), Span::new(18, 24, 1, 19));
    }
}
