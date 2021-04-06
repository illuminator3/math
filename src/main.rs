use std::path::Path;
use crate::lexer::{data, token, full_lex};
use std::fs::{read_to_string, read};
use crate::parser::parse;
use crate::interpreter::{interpret, ExternalRuntimeFunction, RuntimeExpression};
use std::panic::{catch_unwind, set_hook};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::stdin;

mod lexer;
mod parser;
mod ast;
mod interpreter;
mod expression_parser;

macro_rules! external {
    ($name: expr, $parameters: expr, $invoke: expr) => {
        ExternalRuntimeFunction::create(
            $name,
            $parameters,
            $invoke
        )
    };
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    args.remove(0);

    if args.len() != 1 {
        println!("Usage: math <file>");

        return;
    }

    let file = args.get(0).expect("uh");
    let path = Path::new(file);

    if !path.exists() {
        println!("File not found");

        return;
    }

    set_hook(Box::new(|info| { // "suppress" panics so that only their message will be shown
        let mut s = format!("{}", info);

        s = s.replace("panicked at '", "");
        s = s[..s.rfind("', src\\").expect("Malformed string")].to_owned();

        println!("{}", s);
    }));

    // fake_main(Path::new("test.math"));
    fake_main(path);
}

fn fake_main(file: &Path) {
    let start = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let data = data(vec![
        token(
            "LET",
            "let",
            false
        ),
        token(
            "DEFINE",
            "define",
            false
        ),
        token(
            "WHERE",
            "where",
            false
        ),
        token(
            "EXTERNAL",
            "external",
            false
        ),
        token(
            "COMMA",
            ",",
            false
        ),
        token(
            "PIPE",
            "|",
            false
        ),
        token(
            "OPEN_PARENTHESIS",
            "(",
            false
        ),
        token(
            "CLOSE_PARENTHESIS",
            ")",
            false
        ),
        token(
            "EQUALS",
            "==",
            false
        ),
        token(
            "NOT_EQUALS",
            "=!",
            false
        ),
        token(
            "BIGGER_OR_EQUALS",
            ">=",
            false
        ),
        token(
            "BIGGER",
            ">",
            false
        ),
        token(
            "SMALLER_OR_EQUALS",
            "<=",
            false
        ),
        token(
            "SMALLER",
            "<",
            false
        ),
        token(
            "ASSIGN",
            "=",
            false
        ),
        token(
            "PLUS",
            "+",
            false
        ),
        token(
            "MINUS",
            "-",
            false
        ),
        token(
            "DIVIDE",
            "/",
            false
        ),
        token(
            "MULTIPLY",
            "*",
            false
        ),
        // token(
        //     "DOT",
        //     ".",
        //     false
        // ), we probably don't need this for now
        token(
            "NUMBER",
            "([0-9_.]+)",
            true
        ),
        token(
            "WHITESPACE",
            "\\s+",
            true
        ),
        token(
            "IDENTIFIER",
            "[a-zA-Z][A-Za-z0-9_]*(\\*|)",
            true
        )
    ]);
    let content = read_to_string(file).expect("Error while reading file");
    let r = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let lex_result = full_lex(content.to_owned(), "test2.math".to_owned(), "#".to_owned(), data);
    let l = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let external_functions = vec![
        external!( // println(output)
            "println",
            1,
            |args, ast| {
                println!("{}", args.get(0).unwrap().execute(ast));

                0
            }
        ),
        external!( // print(output)
            "print",
            1,
            |args, ast| {
                print!("{}", args.get(0).unwrap().execute(ast));

                0
            }
        ),
        external!( // if(condition, true, false)
            "if",
            3,
            |args, ast| {
                // println!("IF {:?}", RuntimeExpression::expr_to_string(args.get(0).unwrap().orig()));

                return if args.get(0).unwrap().execute(ast.clone()) == 1 {
                    // println!("true");

                    args.get(1).unwrap().execute(ast)
                } else {
                    // println!("false");

                    args.get(2).unwrap().execute(ast)
                }
            }
        ),
        external!( // input()
            "input",
            0,
            |args, ast| {
                let mut input = String::new();

                stdin().read_line(&mut input).ok().expect("Failed to read line");

                let result = input.replace("\r\n", "").replace("\n", "").parse::<isize>();

                if result.is_err() {
                    panic!("Input must be a number");
                }

                result.unwrap()
            }
        )
    ];
    let parse_result = parse(lex_result, external_functions.clone());
    let p = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();

    interpret(parse_result, external_functions);

    let i = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();
    let read_t = r - start;
    let lex_t = l - r;
    let parse_t = p - l;
    let interpret_t = i - p;
    let total_t = i - start;

    println!("Finished in {}ms (R: {}ms L: {}ms P: {}ms I: {}ms)", total_t, read_t, lex_t, parse_t, interpret_t);
}