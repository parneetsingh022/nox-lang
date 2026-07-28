use crate::{
    diagnostic::{ParserError, Span},
    lexer::{Keyword, TokenKind},
    parser::{
        Parser,
        ast::{Stmt, StmtKind},
    },
};

impl<'a> Parser<'a> {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
        let token = self.peek().ok_or_else(|| self.unexpected_eof_error())?;

        match token.kind {
            TokenKind::Keyword(keyword) => self.parse_keyword_stmt(keyword),
            _ => Err(self.expected_statement_error(token)),
        }
    }

    fn parse_keyword_stmt(&mut self, keyword: Keyword) -> Result<Stmt, ParserError> {
        match keyword {
            Keyword::Let => self.parse_let_stmt(),
            _ => {
                let token = self.peek().ok_or_else(|| self.unexpected_eof_error())?;
                Err(self.expected_statement_error(token))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParserError> {
        // Consume the keyword
        let start = self.expect(TokenKind::Keyword(Keyword::Let))?.span;
        let (symbol, _) = self.expect_identifier()?;

        self.expect(TokenKind::Eq)?;

        let expr = self.parse_expr()?;
        let semi = self.expect(TokenKind::Semi)?;

        let span = Span::from_bounds(start, semi.span);

        Ok(Stmt::new(StmtKind::Let { name: symbol, expr }, span))
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
            ast::{BinaryOp, Expr, ExprKind, UnaryOp},
            expression::{binary, boolean, float, identifier, int, unary},
        },
    };

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

        #[allow(unreachable_patterns)]
        match stmt.kind() {
            StmtKind::Let { name, expr } => {
                assert_eq!(variable_name, symbol_registry.resolve(*name));
                assert_eq!(expr, &expected);
            }
            _ => panic!("Expected let statement found: {:?}", stmt.kind()),
        }
    }

    #[rstest]
    #[case("x", "y")]
    #[case("result", "_input")]
    #[case("same", "same")]
    fn parse_let_statement_with_identifier_value(#[case] variable_name: &str, #[case] value: &str) {
        let source = format!("let {variable_name} = {value};");
        let (stmt, symbol_registry) = parse_statement(&source);

        #[allow(unreachable_patterns)]
        match stmt.kind() {
            StmtKind::Let { name, expr } => {
                assert_eq!(variable_name, symbol_registry.resolve(*name));

                let ExprKind::Identifier(symbol) = expr.kind() else {
                    panic!("expected identifier, found: {:?}", expr.kind());
                };

                assert_eq!(value, symbol_registry.resolve(*symbol));
            }
            unexpected => panic!("expected let statement, found: {unexpected:?}"),
        }
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

        #[allow(irrefutable_let_patterns)]
        let StmtKind::Let { name, expr } = stmt.kind() else {
            panic!("expected let statement, found: {:?}", stmt.kind());
        };

        assert_eq!("result", symbol_registry.resolve(*name));
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
}
