mod common;

use common::make_lexer;
use nox_lang::lexer::TokenKind;

fn snapshot_tokens(source: &str) -> String {
    let mut lexer = make_lexer(source);
    let mut results = Vec::new();

    // Use a loop to avoid iterator lifetime issues
    while let Some(token_result) = lexer.next() {
        let t = token_result.expect("Lexer error");

        let kind_str = match &t.kind {
            TokenKind::Identifier(sym) => {
                format!("Identifier({:?})", lexer.symbol_registry.resolve(*sym))
            }
            TokenKind::IntLiteral(sym) => {
                format!("IntLiteral({:?})", lexer.symbol_registry.resolve(*sym))
            }
            TokenKind::FloatLiteral(sym) => {
                format!("FloatLiteral({:?})", lexer.symbol_registry.resolve(*sym))
            }
            other => format!("{:?}", other),
        };

        results.push(format!(
            "kind: {}\npos:  {}:{}\nrange: [{}..{}]\ntext:  {:?}\n",
            kind_str,
            t.span.line,
            t.span.column,
            t.span.start,
            t.span.end,
            &source[t.span.start..t.span.end]
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
