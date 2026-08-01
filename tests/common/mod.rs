use miette::{Diagnostic, GraphicalReportHandler, GraphicalTheme};
use nyx_lexer::Lexer;
use nyx_source::SourceFile;

pub fn make_lexer(code: &str) -> (Lexer, SourceFile) {
    let source_file: SourceFile = SourceFile::new("main.nyx", code);
    (Lexer::new(source_file.clone()), source_file)
}

pub fn render_diagnostic(diagnostic: &dyn Diagnostic) -> String {
    let mut output = String::new();

    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .with_width(80)
        .render_report(&mut output, diagnostic)
        .expect("diagnostic rendering should succeed");

    output
}
