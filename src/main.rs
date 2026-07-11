pub mod diagnostic;
pub mod tokenizer;

use tokenizer::lexer::Lexer;

fn main() {
    let string = "let const ident ident2 349 5450 544.220 39.33";

    let lexer = Lexer::new(string);

    for token in lexer.into_iter() {
        println!("{:?}", token);
    }
}
