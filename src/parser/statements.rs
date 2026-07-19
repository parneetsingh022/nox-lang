use crate::{
    lexer::{Keyword, TokenKind},
    parser::{Parser, ast::Statement},
};

impl<'a> Parser<'a> {
    pub fn parse_statement(&mut self) -> Statement {
        let keyword = self.expect_keyword("Expected a statement");

        match keyword {
            Keyword::Let => self.parse_let_statement(),
            _ => panic!("Unexpected statement!"),
        }
    }

    fn parse_let_statement(&mut self) -> Statement {
        let symbol = self.expect_identifier("Expected an identifier after let");
        self.expect(
            TokenKind::Eq,
            "Expected an equal after identifier in let statement",
        );

        let expr = self.parse_expr();
        self.expect(TokenKind::Semi, "Expected a semicolon after let statement");

        Statement::Let { name: symbol, expr }
    }
}
