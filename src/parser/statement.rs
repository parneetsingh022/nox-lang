use nyx_diagnostic::ParserError;
use nyx_source::Span;
use nyx_token::{Keyword, Token, TokenKind};

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
            TokenKind::OpenBrace => self.parse_block_stmt(),
            _ if is_expr_start(token) => self.parse_expr_stmt(),
            _ => Err(self.expected_statement_error(token)),
        }
    }

    fn parse_keyword_stmt(&mut self, token: Token) -> Result<Stmt, ParserError> {
        match token.kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_let_stmt(),
            TokenKind::Keyword(Keyword::If) => self.parse_if_stmt(),
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

    fn parse_block_stmt(&mut self) -> Result<Stmt, ParserError> {
        let start = self.expect(TokenKind::OpenBrace)?.span;
        let mut stmts = Vec::new();

        while let Some(token) = self.peek() {
            if token.kind == TokenKind::CloseBrace {
                break;
            }

            stmts.push(self.parse_stmt()?);
        }

        let end = self.expect_closing(TokenKind::CloseBrace, start)?.span;

        let span = Span::from_bounds(start, end);

        Ok(Stmt::new(StmtKind::Block { stmts }, span))
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParserError> {
        let start = self.expect(TokenKind::Keyword(Keyword::If))?.span;
        let cond = self.parse_expr()?;

        let block_stmt = self.parse_block_stmt()?;
        let mut end = block_stmt.span();

        let then_branch = Box::new(block_stmt);
        let mut else_branch = None;

        // Possible `else` or `else if` branch
        if self.eat(TokenKind::Keyword(Keyword::Else)) {
            let token = self.peek().ok_or_else(|| self.unexpected_eof_error())?;

            let branch = if token.kind == TokenKind::OpenBrace {
                self.parse_block_stmt()?
            } else if token.kind == TokenKind::Keyword(Keyword::If) {
                self.parse_if_stmt()?
            } else {
                return Err(ParserError::ExpectedToken {
                    expected: TokenKind::OpenBrace,
                    found: token.kind,
                    at: token.span.into(),
                    src: self.source_file.clone(),
                });
            };

            end = branch.span();
            else_branch = Some(Box::new(branch));
        }

        let span = Span::from_bounds(start, end);

        Ok(Stmt::new(
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            },
            span,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nyx_lexer::Lexer;
    use nyx_source::SourceFile;
    use nyx_token::SymbolRegistry;
    use rstest::rstest;

    use crate::parser::{
        ast::{BinaryOp, Expr, ExprKind, SpannedIdentifier, UnaryOp},
        expression::tests::{binary, boolean, float, identifier, int, unary},
    };

    #[cfg(test)]
    pub fn make_lexer(code: &str) -> (Lexer, SourceFile) {
        let source_file: SourceFile = SourceFile::new("main.nyx", code);
        (Lexer::new(source_file.clone()), source_file)
    }

    /// This assumes a statement to have [`StatementKind::Let`] otherwise
    /// it panics
    fn as_let(stmt: &Stmt) -> (&SpannedIdentifier, &Expr) {
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

        let value_symbol = symbol_registry.intern("value");

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

    // =========================================================================
    // Block Statement Tests
    // =========================================================================

    /// This assumes a statement to have [`StmtKind::Block`] otherwise
    /// it panics
    fn as_block(stmt: &Stmt) -> &Vec<Stmt> {
        let StmtKind::Block { stmts } = stmt.kind() else {
            panic!("Expected block found: {:?}", stmt);
        };
        stmts
    }

    #[test]
    fn parses_empty_block() {
        let source = "{}";
        let (stmt, _) = parse_statement(source);

        let stmts = as_block(&stmt);
        assert!(
            stmts.is_empty(),
            "expected empty block to contain no statements"
        );
    }

    #[test]
    fn parses_single_statement_block() {
        let source = "{ let x = 42; }";
        let (stmt, _) = parse_statement(source);

        let stmts = as_block(&stmt);
        assert_eq!(stmts.len(), 1, "expected exactly 1 statement in the block");
        assert!(
            matches!(stmts[0].kind(), StmtKind::Let { .. }),
            "expected let statement inside block"
        );
    }

    #[test]
    fn parses_multiple_statements_block() {
        let source = "{ let x = 42; foo(); x = 10; }";
        let (stmt, _) = parse_statement(source);

        let stmts = as_block(&stmt);
        assert_eq!(stmts.len(), 3, "expected exactly 3 statements in the block");

        assert!(matches!(stmts[0].kind(), StmtKind::Let { .. }));
        assert!(matches!(stmts[1].kind(), StmtKind::ExprStmt { .. }));
        assert!(matches!(stmts[2].kind(), StmtKind::ExprStmt { .. }));
    }

    #[test]
    fn parses_nested_blocks() {
        let source = "{ { let x = 5; } foo(); }";
        let (stmt, _) = parse_statement(source);

        let stmts = as_block(&stmt);
        assert_eq!(stmts.len(), 2, "expected 2 statements in the outer block");

        // Check inner block
        let inner_block = as_block(&stmts[0]);
        assert_eq!(
            inner_block.len(),
            1,
            "expected 1 statement in the inner block"
        );
        assert!(matches!(inner_block[0].kind(), StmtKind::Let { .. }));

        // Check outer expression statement
        assert!(matches!(stmts[1].kind(), StmtKind::ExprStmt { .. }));
    }

    #[test]
    fn block_span_covers_opening_and_closing_braces() {
        let source = "{ let x = 5; }";
        let (stmt, _) = parse_statement(source);

        // Assuming Span::single_line takes (start_byte, end_byte, line, start_col, end_col)
        assert_eq!(stmt.span(), Span::single_line(0, source.len(), 1, 1, 15));
    }

    #[test]
    fn multiline_block_span_is_tracked_correctly() {
        let source = "{\n    let x = 5;\n}";
        let (stmt, _) = parse_statement(source);

        let span = stmt.span();
        assert_eq!(span.start, 0, "Span should start at the opening brace");
        assert_eq!(
            span.end,
            source.len(),
            "Span should end after the closing brace"
        );
    }

    #[rstest]
    #[case("{")]
    #[case("{ let x = 5;")]
    #[case("{ foo(); { let y = 10; }")]
    fn reports_error_for_unclosed_block(#[case] source: &str) {
        let error = try_parse_statement(source)
            .err()
            .unwrap_or_else(|| panic!("expected unclosed block `{source}` to fail parsing"));

        // Depending on how `expect_closing` works, this might be a specific ExpectedClosing error
        // or an UnexpectedEof error if it runs out of tokens first.
        assert!(
            matches!(
                error,
                ParserError::UnclosedDelimiter { .. } | ParserError::UnexpectedEof { .. }
            ),
            "expected expected closing brace or unexpected EOF for `{source}`, found: {error:?}"
        );
    }

    // =========================================================================
    // If Statement Tests
    // =========================================================================

    /// Helper to extract the components of an `If` statement, otherwise panics.
    fn as_if(stmt: &Stmt) -> (&Expr, &Stmt, Option<&Stmt>) {
        let StmtKind::If {
            cond,
            then_branch,
            else_branch,
        } = stmt.kind()
        else {
            panic!("Expected if statement found: {:?}", stmt);
        };
        (cond, then_branch.as_ref(), else_branch.as_deref())
    }

    #[test]
    fn parses_simple_if_statement() {
        let source = "if true { foo(); }";
        let (stmt, _) = parse_statement(source);

        let (cond, then_branch, else_branch) = as_if(&stmt);

        assert_eq!(cond, &boolean(true), "expected boolean true condition");
        assert!(else_branch.is_none(), "expected no else branch");

        let then_stmts = as_block(then_branch);
        assert_eq!(then_stmts.len(), 1, "expected 1 statement in then block");
        assert!(matches!(then_stmts[0].kind(), StmtKind::ExprStmt { .. }));
    }

    #[test]
    fn parses_if_else_statement() {
        let source = "if true { foo(); } else { bar(); }";
        let (stmt, _) = parse_statement(source);

        let (_, then_branch, else_branch) = as_if(&stmt);

        let then_stmts = as_block(then_branch);
        assert_eq!(then_stmts.len(), 1, "expected 1 statement in then block");

        let else_branch = else_branch.expect("expected an else branch");
        let else_stmts = as_block(else_branch);
        assert_eq!(else_stmts.len(), 1, "expected 1 statement in else block");
    }

    #[test]
    fn parses_if_else_if_chain() {
        let source = "if x { foo(); } else if y { bar(); }";
        let (stmt, symbol_registry) = parse_statement(source);

        let (_, then_branch, else_branch) = as_if(&stmt);

        // Verify the `then` branch is a block
        assert_eq!(as_block(then_branch).len(), 1);

        // Verify the `else` branch is another `if` statement
        let else_stmt = else_branch.expect("expected an else branch");
        let (inner_cond, inner_then, inner_else) = as_if(else_stmt);

        let ExprKind::Identifier(sym) = inner_cond.kind() else {
            panic!("expected identifier in else-if condition");
        };
        assert_eq!("y", symbol_registry.resolve(*sym));

        assert_eq!(as_block(inner_then).len(), 1);
        assert!(inner_else.is_none(), "expected no trailing else branch");
    }

    #[test]
    fn parses_if_else_if_else_chain() {
        let source = "if x { foo(); } else if y { bar(); } else { baz(); }";
        let (stmt, _) = parse_statement(source);

        let (_, _, else_branch) = as_if(&stmt);

        // Extract the chained `else if`
        let else_if_stmt = else_branch.expect("expected first else branch");
        let (_, _, final_else) = as_if(else_if_stmt);

        // Extract the final `else` block
        let final_else_block = final_else.expect("expected final else branch");
        assert_eq!(as_block(final_else_block).len(), 1);
    }

    #[test]
    fn if_statement_span_covers_basic_if() {
        let source = "if true { foo(); }";
        let (stmt, _) = parse_statement(source);

        assert_eq!(stmt.span(), Span::single_line(0, source.len(), 1, 1, 19));
    }

    #[test]
    fn if_statement_span_covers_else_branch() {
        let source = "if true { foo(); } else { bar(); }";
        let (stmt, _) = parse_statement(source);

        assert_eq!(stmt.span(), Span::single_line(0, source.len(), 1, 1, 35));
    }

    #[test]
    fn if_statement_span_covers_long_else_if_chain() {
        let source = "if a { } else if b { } else if c { } else { }";
        let (stmt, _) = parse_statement(source);

        // The overall span of the root AST node should encompass the very first 'if'
        // to the very last '}' in the final else block.
        assert_eq!(stmt.span(), Span::single_line(0, source.len(), 1, 1, 46));
    }

    #[rstest]
    #[case("if true foo();")] // Missing `{` for then branch
    #[case("if true {} else foo();")] // Missing `{` or `if` for else branch
    #[case("if true {} else 5;")] // Unexpected expression after else
    #[case("if true { foo(); ")] // Unclosed then block
    #[case("if true {} else { foo(); ")] // Unclosed else block
    #[case("if true {} else")] // EOF right after `else`
    fn rejects_invalid_if_statements(#[case] source: &str) {
        let error = try_parse_statement(source)
            .err()
            .unwrap_or_else(|| panic!("expected parsing to fail for `{source}`"));

        // We just assert that it produces an error.
        // We don't strictly match the error type here since it varies
        // (ExpectedToken, UnexpectedEof, UnclosedDelimiter, etc.)
        assert!(
            matches!(
                error,
                ParserError::ExpectedToken { .. }
                    | ParserError::UnexpectedEof { .. }
                    | ParserError::UnclosedDelimiter { .. }
                    | ParserError::InvalidExpressionStatement { .. } // depending on how `if {}` fails
            ),
            "expected a syntax error for `{source}`, found: {error:?}"
        );
    }

    #[test]
    fn reports_expected_brace_or_if_after_else() {
        let source = "if true {} else foo();";
        let error = try_parse_statement(source).err().unwrap();

        assert!(
            matches!(
                error,
                ParserError::ExpectedToken {
                    expected: TokenKind::OpenBrace,
                    ..
                }
            ),
            "expected a missing OpenBrace error, found: {:?}",
            error
        );
    }
}
