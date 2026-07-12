pub mod diagnostic;
pub mod tokenizer;

use tokenizer::lexer::Lexer;

use crate::tokenizer::Token;

fn main() {
    let string = "let x = (30+45 / (45-2));";

    let mut lexer = Lexer::new(string, "main.nox");
    let tokens: Vec<Token<'_>> = lexer.by_ref().collect();
    let errors = lexer.take_errors();

    for err in errors {
        eprintln!("{:?}", miette::Report::new(err));
    }

    for tok in tokens {
        println!("{:?}", tok);
    }
}
