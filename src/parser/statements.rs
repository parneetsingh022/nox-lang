use crate::{
    diagnostic::Span,
    lexer::{Keyword, Token, TokenKind},
    parser::{Parser, ast::Statement},
};

impl<'a> Parser<'a> {
    pub fn parse_statement(&mut self) -> Statement {
        let token = self.expect_keyword("Expected a statement");

        let (keyword, span) = match &token {
            Token {
                kind: TokenKind::Keyword(keyword),
                span,
            } => (*keyword, *span),
            _ => unreachable!(),
        };

        match keyword {
            Keyword::Let => self.parse_let_statement(span),
            _ => panic!("Unexpected statement!"),
        }
    }

    fn parse_let_statement(&mut self, start: Span) -> Statement {
        let symbol = self.expect_identifier("Expected an identifier after let");
        self.expect(
            TokenKind::Eq,
            "Expected an equal after identifier in let statement",
        );

        let expr = self.parse_expr();
        self.expect(TokenKind::Semi, "Expected a semicolon after let statement");
        let span = start.to(expr.span());
        Statement::Let {
            name: symbol,
            expr,
            span,
        }
    }
}
