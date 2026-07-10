pub mod diagnostic;
pub mod tokenizer;

use tokenizer::lexer::Lexer;

fn main() {
    let str = "ident ident2";

    let lexer = Lexer::new(str.as_bytes());

    for token in lexer.into_iter() {
        if token.kind == tokenizer::TokenKind::Eof {
            break;
        }
        println!("{:?}", token);
    }
}
