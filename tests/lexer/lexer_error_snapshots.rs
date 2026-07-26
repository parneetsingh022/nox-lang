use crate::common::{make_lexer, render_diagnostic};

fn snapshot_lexer_errors(source: &str) -> String {
    let (mut lexer, _) = make_lexer(source);
    let mut fatal_error = None;

    while let Some(result) = lexer.by_ref().next() {
        if let Err(error) = result {
            fatal_error = Some(error);
            break;
        }
    }

    let mut rendered_errors: Vec<String> = lexer
        .take_errors()
        .into_iter()
        .map(|error| render_diagnostic(&error))
        .collect();

    if let Some(error) = fatal_error {
        rendered_errors.push(render_diagnostic(&error));
    }

    assert!(
        !rendered_errors.is_empty(),
        "expected at least one lexer error, but lexing succeeded"
    );

    rendered_errors.join("\n")
}

#[test]
fn report_unexpected_tokens() {
    insta::assert_snapshot!(snapshot_lexer_errors("unexpected 😀 error Ñ你 好"));
}

#[test]
fn report_unterminated_block_comment() {
    let source = r#"
let x = 10;

/* This block comment
   continues across multiple lines
   but is never closed

let g = 20.2;
"#;

    insta::assert_snapshot!(snapshot_lexer_errors(source));
}

#[test]
fn report_invalid_numeric_tokens() {
    let source = r#"
123.
294.abc
343abc
"#;

    insta::assert_snapshot!(snapshot_lexer_errors(source));
}
