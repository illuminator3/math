use std::path::Path;
use crate::lexer::{data, token, full_lex};
use std::fs::read_to_string;
use crate::parser::parse;
use crate::interpreter::{interpret, runtime::ExternalRuntimeFunction};
use std::panic::set_hook;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::io::{stdin, Write};
use num_bigint::BigInt;
use std::thread;
use std::io::stdout;

pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod parser;

macro_rules! external {
    ($name: expr, $parameters: expr, $invoke: expr) => {
        ExternalRuntimeFunction::create(
            $name,
            $parameters,
            $invoke
        )
    };
}

const DEV: bool = false;

fn main() {
    if DEV {
        fake_main(Path::new("test.math"));
    } else {
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

        fake_main(path);
    }
}

fn fake_main(file: &Path) {
    let start = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
    let data = data(vec![
        token(
            "LET",
            "let",
            false
        ),
        token(
            "CONST",
            "const",
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
            "CACHE",
            "cache",
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
        token(
            "POW",
            "^",
            false
        ),
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
    let t = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
    let content = read_to_string(file).expect("Error while reading file");
    let r = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
    let lex_result = full_lex(content.to_owned(), file.file_name().unwrap().to_str().unwrap().to_owned(), "#".to_owned(), data);
    let l = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
    let external_functions = vec![
        external!( // println(output)
            "println",
            1,
            |args, ast| {
                println!("{}", args.get(0).unwrap().execute(ast));

                BigInt::from(0)
            }
        ),
        external!( // print(output)
            "print",
            1,
            |args, ast| {
                print!("{}", args.get(0).unwrap().execute(ast));

                stdout().flush().unwrap(); // flush so it gets printed

                BigInt::from(0)
            }
        ),
        external!( // if(condition, true, false)
            "if",
            3,
            |args, ast| {
                return if args.get(0).unwrap().execute(ast) == BigInt::from(1) {
                    args.get(1).unwrap().execute(ast)
                } else {
                    args.get(2).unwrap().execute(ast)
                }
            }
        ),
        external!( // input()
            "input",
            0,
            |_, _| {
                let mut input = String::new();

                stdin().read_line(&mut input).ok().expect("Failed to read line");

                let result = input.replace("\r\n", "").replace("\n", "").parse::<isize>();

                if result.is_err() {
                    panic!("Input must be a number");
                }

                BigInt::from(result.unwrap())
            }
        ),
        external!( // sleep(millis)
            "sleep",
            1,
            |args, ast| {
                thread::sleep(Duration::from_millis(*args.get(0).unwrap().execute(ast).to_u64_digits().1.get(0).unwrap()));

                BigInt::from(0)
            }
        ),
        external!( // newline()
            "newline",
            0,
            |_, _| {
                println!();

                BigInt::from(0)
            }
        ),
        external!( // empty()
            "empty",
            0,
            |_, _| {
                print!(" ");

                stdout().flush().unwrap(); // flush so it gets printed

                BigInt::from(0)
            }
        )
    ];
    let parse_result = parse(lex_result, external_functions.clone());
    let p = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();

    interpret(parse_result, external_functions);

    let i = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_micros();
    let token_t = t - start;
    let read_t = r - t;
    let lex_t = l - r;
    let parse_t = p - l;
    let interpret_t = i - p;
    let total_t = i - start;
    let t_stuff = |i: u128| -> String {
        let m = i / 1000;

        return if m != 0 {
            format!("{}ms", m)
        } else {
            format!("{}µs", i)
        }
    };

    println!("Finished in {} (T: {}, R: {} L: {} P: {} I: {})", t_stuff(total_t), t_stuff(token_t), t_stuff(read_t), t_stuff(lex_t), t_stuff(parse_t), t_stuff(interpret_t));
}
