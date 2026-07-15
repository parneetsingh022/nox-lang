use std::{env, fs, process};

use nox_lang::tokenizer::Lexer;
use nox_lang::tokenizer::Token;

fn main() {
    // Collect arguments from the command line
    let args: Vec<String> = env::args().collect();

    // Check if the file path argument is provided
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];

    // Read the file, exiting gracefully if the file cannot be read
    let string = fs::read_to_string(file_path).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", file_path, err);
        process::exit(1);
    });

    let mut lexer = Lexer::new(&string, file_path);
    let tokens: Vec<Token<'_>> = match lexer.by_ref().collect::<Result<Vec<_>, _>>() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("{:?}", miette::Report::new(err));
            return;
        }
    };

    let errors = lexer.take_errors();

    if !errors.is_empty() {
        eprintln!("Lexing failed with {} error(s):", errors.len());
        for err in errors {
            eprintln!("{:?}", miette::Report::new(err));
        }
        // Exit early if errors are found
        std::process::exit(1);
    }

    for tok in tokens {
        println!("{:?}", tok);
    }
}
