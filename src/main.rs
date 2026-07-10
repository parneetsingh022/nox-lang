pub mod diagnostic;
pub mod tokenizer;

use tokenizer::lexer::Lexer;

fn main() {
    let str = "let const ident ident2 349 5450 544.220 39.33";

    let lexer = Lexer::new(str.as_bytes());

    for token in lexer.into_iter() {
        if token.kind == tokenizer::TokenKind::Eof {
            break;
        }
        println!("{:?}", token);
    }
}
