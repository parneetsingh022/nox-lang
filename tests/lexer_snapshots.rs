use nox_lang::tokenizer::Lexer;
fn snapshot_tokens(source: &str) -> String {
    let lexer = Lexer::new(source, "main.nox");
    lexer
        .map(|t| {
            format!(
                "kind: {:?}\npos:  {}:{}\nrange: [{}..{}]\ntext:  {:?}\n",
                t.kind,
                t.span.line,
                t.span.column,
                t.span.start,
                t.span.end,
                &source[t.span.start..t.span.end]
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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
