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
        let token = self.peek()?.clone();

        match token.kind {
            TokenKind::Keyword(keyword) => self.parse_keyword_stmt(keyword),
            _ => Err(self.expected_statement_error(&token)),
        }
    }

    pub fn parse_keyword_stmt(&mut self, keyword: Keyword) -> Result<Stmt, ParserError> {
        match keyword {
            Keyword::Let => self.parse_let_stmt(),
            _ => Err(self.expected_statement_error(&self.peek()?.clone())),
        }
    }

    pub fn parse_let_stmt(&mut self) -> Result<Stmt, ParserError> {
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
