use crate::common::make_lexer;
use nyx_token::TokenKind;

fn snapshot_tokens(source: &str) -> String {
    let (mut lexer, source_file) = make_lexer(source);
    let mut results = Vec::new();

    while let Some(token_result) = lexer.next() {
        let token = token_result.expect("lexer error");
        let location = source_file.location(token.span.start);

        let kind = match &token.kind {
            TokenKind::Identifier(symbol) => {
                format!("Identifier({:?})", lexer.symbol_registry.resolve(*symbol))
            }

            TokenKind::IntLiteral(symbol) => {
                format!("IntLiteral({:?})", lexer.symbol_registry.resolve(*symbol))
            }

            TokenKind::FloatLiteral(symbol) => {
                format!("FloatLiteral({:?})", lexer.symbol_registry.resolve(*symbol))
            }

            other => format!("{other:?}"),
        };

        results.push(format!(
            "kind: {kind}\npos:  {}:{}\nrange: [{}..{}]\ntext:  {:?}\n",
            location.line,
            location.column,
            token.span.start,
            token.span.end,
            &source[token.span.range()],
        ));
    }

    results.join("\n")
}

#[test]
fn test_entire_file_tokenization() {
    let source = r#"
/* This multiline comment should be skipped
from the tokens... */
let x = 10;
const PI = 3.14;

// This is a test
fn main() {
    let y = [1, 2, 3];
    /* Trying to get 1 out of y
        * to display it to the screen
        */
    print(x + y[0]);
}
    "#;

    insta::assert_snapshot!(snapshot_tokens(source));
}

#[test]
fn test_real_world_operator_usage() {
    let source = r#"if (x != 0 && y >= 10.5 || x <= 10) {
    z = x + y * 2;
} else {
    z = -1;
}
"#;

    insta::assert_snapshot!(snapshot_tokens(source));
}
