use nox_lang::{diagnostic::SourceFile, lexer::Lexer};

pub fn make_lexer(code: &str) -> Lexer {
    let source_file: SourceFile = SourceFile::new("main.nox", code);
    Lexer::new(source_file)
}
