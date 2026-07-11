pub mod diagnostic;
pub mod tokenizer;

use tokenizer::lexer::Lexer;

use crate::tokenizer::Token;

fn main() {
    let string = "what is your name 3495.\r\nthis is new line 339 439.24 454abc";

    let mut lexer = Lexer::new(string, "main.nox");
    let tokens: Vec<Token<'_>> = lexer.by_ref().collect();
    let errors = lexer.take_errors();

    for e in errors {
        println!("{:?}", e);
    }

    for tok in tokens {
        println!("{:?}", tok);
    }
}
