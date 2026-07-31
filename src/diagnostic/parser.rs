use miette::SourceSpan;

use crate::{diagnostic::SourceFile, lexer::TokenKind};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ParserError {
    #[error("unexpected end of file")]
    #[diagnostic(
        code(nox::parser::unexpected_eof),
        help("check for unclosed delimiters, incomplete expressions, or trailing operators")
    )]
    UnexpectedEof {
        #[label("unexpected end of file here")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("expected `{expected}`, found `{found}`")]
    #[diagnostic(
        code(nox::parser::expected_token),
        help("insert the missing token or remove the unexpected one")
    )]
    ExpectedToken {
        expected: TokenKind,
        found: TokenKind,

        #[label("expected `{expected}` after this")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("expected an identifier, found `{found}`")]
    #[diagnostic(
        code(nox::parser::expected_identifier),
        help("provide a valid identifier")
    )]
    ExpectedIdentifier {
        found: TokenKind,

        #[label("expected an identifier here")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("expected an expression, found `{found}`")]
    #[diagnostic(
        code(nox::parser::expected_expression),
        help("remove the unexpected token or provide the missing expression")
    )]
    ExpectedExpression {
        found: TokenKind,

        #[label("expected an expression here")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("expected a statement, found `{found}`")]
    #[diagnostic(
        code(nox::parser::expected_statement),
        help("remove the unexpected token or begin a valid statement")
    )]
    ExpectedStatement {
        found: TokenKind,

        #[label("expected a statement here")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("expected ';'")]
    #[diagnostic(
        code(nox::parser::expected_semicolon),
        help("add a semicolon `;` to terminate the statement")
    )]
    ExpectedSemicolon {
        #[label("expected ';' here")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("unclosed delimiter")]
    #[diagnostic(
        code(nox::parser::unclosed_delimiter),
        help("insert a `{expected}` to close this group")
    )]
    UnclosedDelimiter {
        expected: TokenKind,

        #[label("expected `{expected}` to close this")]
        opened_at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    #[error("missing operator between expressions")]
    #[diagnostic(
        code(nox::parser::missing_operator),
        help("use an operator (like `*`, `+`, etc.) between these values.")
    )]
    MissingOperator {
        #[label("this expression...")]
        left_span: SourceSpan,

        #[label("...is followed directly by this, but needs an operator between them")]
        right_span: SourceSpan,

        #[source_code]
        src: SourceFile,
    },

    /// Raised when an expression is used as a statement even though its result
    /// is not meaningful when ignored.
    #[error("expression result is unused")]
    #[diagnostic(
        code(nox::parser::invalid_expression_statement),
        help("use the expression as part of another expression, assign its result, or remove it")
    )]
    InvalidExpressionStatement {
        #[label("the result of this expression is ignored")]
        at: SourceSpan,

        #[source_code]
        src: SourceFile,
    },
}
