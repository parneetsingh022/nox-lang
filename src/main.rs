use std::{env, fs, process};

use nox_lang::parser::Parser;
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
    let tokens: Vec<Token> = match lexer.by_ref().collect::<Result<Vec<_>, _>>() {
        Ok(tokens) => tokens,
        Err(err) => {
            print_collected_errors(&mut lexer);
            eprintln!("{:?}", miette::Report::new(err));
            std::process::exit(1);
        }
    };

    if print_collected_errors(&mut lexer) {
        std::process::exit(1);
    }

    let mut parser = Parser::new(&tokens, &lexer.symbol_registry);
    let exp = parser.parse_expr();
    println!("{:#?}", exp.debug_with(&lexer.symbol_registry));
}

/// Prints all error collected by lexer.
///
/// Returns true if any errors were printed.
fn print_collected_errors(lexer: &mut Lexer) -> bool {
    let errors = lexer.take_errors();
    if errors.is_empty() {
        return false;
    }

    eprintln!("Lexing failed with {} error(s):", errors.len());
    for err in errors {
        eprintln!("{:?}", miette::Report::new(err));
    }

    true
}
