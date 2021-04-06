use crate::ast::{Expression, Variable, MathType, Function};
use crate::parser::{TokenQueue, token_queue};
use crate::lexer::{LexedToken, Token};
use std::collections::HashMap;
use std::any::Any;
use std::fmt::{Debug, Formatter};
use crate::expression_parser::Precedence::Prefix;
use num_bigint::BigInt;

pub fn parse_expression(queue: &mut TokenQueue, variables: &Vec<Variable>, functions: &Vec<Function>) -> Expression {
    actual_parse_expression(parse_expression_part(queue, Precedence::None), variables, functions)
}

pub fn parse_expression_part(queue: &mut TokenQueue, precedence: Precedence) -> PartExpression {
    if queue.is_empty() {
        panic!("Not sure what exactly you want");
    }

    let mut next = queue.peek();
    let prefix_parser = prefix_parser(next.token_type().clone());
    let mut left = prefix_parser.runner_prefix()(queue, next);

    loop {
        if queue.is_empty() {
            break;
        }

        next = queue.get().clone();

        let infix_parser = infix_parser(next.token_type().clone());

        if precedence.order() >= infix_parser.precedence_infix().order() {
            break;
        }

        queue.remove();

        left = infix_parser.runner_infix()(queue, left, next, infix_parser.precedence_infix().clone())
    }

    left
}

enum Parser {
    Infix {
        token: Token,
        runner: fn(&mut TokenQueue, PartExpression, LexedToken, Precedence) -> PartExpression,
        precedence: Precedence
    },
    Prefix {
        token: Token,
        runner: fn(&mut TokenQueue, LexedToken) -> PartExpression
    }
}

impl Parser {
    fn runner_infix(&self) -> fn(&mut TokenQueue, PartExpression, LexedToken, Precedence) -> PartExpression {
        match *self {
            Parser::Infix { runner, .. } => runner,
            Parser::Prefix { .. } => panic!("Not supported")
        }
    }

    fn precedence_infix(&self) -> &Precedence {
        match self {
            Parser::Infix { precedence, .. } => precedence,
            Parser::Prefix { .. } => panic!("Not supported")
        }
    }

    fn runner_prefix(&self) -> fn(&mut TokenQueue, LexedToken) -> PartExpression {
        match *self {
            Parser::Prefix { runner, .. } => runner,
            Parser::Infix { .. } => panic!("Not supported")
        }
    }

    fn token(&self) -> Token {
        match *self {
            Parser::Infix { token, .. } | Parser::Prefix { token, .. } => token,
        }
    }
}

fn default_parse_infix(queue: &mut TokenQueue, left: PartExpression, token: LexedToken, precedence: Precedence) -> PartExpression {
    PartExpression::InfixOperator {
        operator: token.content().to_owned(),
        left: Box::new(left),
        right: Box::new(parse_expression_part(queue, precedence.one_less().clone())),
        token
    }
}

fn infix_parser(token: Token) -> Parser {
    match token.id() {
        "PLUS" | "MINUS" => Parser::Infix {
            token,
            runner: default_parse_infix,
            precedence: Precedence::Sum
        },
        "MULTIPLY" | "DIVIDE" => Parser::Infix {
            token,
            runner: default_parse_infix,
            precedence: Precedence::Product
        },
        "EQUALS" | "NOT_EQUALS" | "BIGGER_OR_EQUALS" | "BIGGER" | "SMALLER_OR_EQUALS" | "ASSIGN" => Parser::Infix {
            token,
            runner: default_parse_infix,
            precedence: Precedence::Conditional
        },
        "ASSIGN" => Parser::Infix {
            token,
            runner: default_parse_infix,
            precedence: Precedence::Assignment
        },
        "OPEN_PARENTHESIS" => Parser::Infix { // I literally hate myself. I made so much typos initally writing this.... (I wrote "OPEN_PARANTHESIS") THIS TOOK ME 1 F*CKING HOUR TO DEBUG
            token,
            runner: |queue, left, token, precedence| -> PartExpression {
                match left {
                    PartExpression::Identifier { .. } => {},
                    _ => token.err("Identifier expected")
                }

                let mut arguments = Vec::<PartExpression>::new();
                let mut first = true;

                while queue.is_not_empty() { // maybe also loop here
                    let next = queue.peek();

                    if first {
                        first ^= true;

                        if next.token_type().id().eq("CLOSE_PARENTHESIS") {
                            break;
                        }

                        queue.back();
                        arguments.push(parse_expression_part(queue, Precedence::None));
                    } else {
                        match next.token_type().id() {
                            "CLOSE_PARENTHESIS" => break,
                            "COMMA" => arguments.push(parse_expression_part(queue, Precedence::None)),
                            _ => next.err("CLOSE_PARENTHESIS or COMMA expected")
                        }
                    }
                }

                PartExpression::FunctionInvocation {
                    val: Box::new(left),
                    arguments,
                    token
                }
            },
            precedence: Precedence::FunctionInvocation
        },
        _ => Parser::Infix {
            token,
            runner: |queue, left, token, precedence | -> PartExpression {
                token.err(&format!("Unknown infix ('{}')", token.token_type().id()))
            },
            precedence: Precedence::None
        }
    }
}

fn prefix_parser(token: Token) -> Parser {
    Parser::Prefix {
        token,
        runner: match token.id() {
            "MINUS" => |queue, t| -> PartExpression {
                PartExpression::PrefixOperator {
                    prefix: t.content().to_owned(),
                    expression: Box::new(parse_expression_part(queue, Precedence::Prefix)),
                    token: t
                }
            },
            "NUMBER" => |queue, t| -> PartExpression {
                PartExpression::Number {
                    val: t.content().parse::<BigInt>().unwrap(),
                    token: t
                }
            },
            "IDENTIFIER" => |queue, t| -> PartExpression {
                PartExpression::Identifier {
                    val: t.content().to_owned(),
                    token: t
                }
            },
            "OPEN_PARENTHESIS" => |queue, t| -> PartExpression {
                // let expr = parse_expression_part(queue, Precedence::None);
                //
                // // TODO not working :(
                //
                // let peek = queue.peek();
                //
                // println!("{}", &peek.token_type().id());
                //
                // peek.check_id("CLOSE_PARENTHESIS", "Expected close paranthesis".to_owned());
                //
                // expr

                let mut expr_queue_vec = Vec::<LexedToken>::new();
                let mut paras = 1;

                while queue.is_not_empty() {
                    let next = queue.peek(); // maybe change to #get
                    let id = next.token_type().id();

                    if id.eq("OPEN_PARENTHESIS") {
                        paras += 1;
                    } else if id.eq("CLOSE_PARENTHESIS") {
                        paras -= 1;
                    }

                    if paras < 0 {
                        next.err("Too many OPEN_PARENTHESIS");
                    } else if paras == 0 {
                        // queue.remove();

                        if expr_queue_vec.is_empty() {
                            next.err("Empty block");
                        }

                        let mut expr_queue = token_queue(expr_queue_vec);

                        return parse_expression_part(&mut expr_queue, Precedence::None);
                    }

                    expr_queue_vec.push(next);
                }

                t.err_offset("Missing CLOSING_PARENTHESIS", 1);
            },
            _ => | _, t| -> PartExpression {
                t.err(&format!("Unknown prefix ('{}')", t.token_type().id()));
            }
        }
    }
}

pub fn actual_parse_expression(expr: PartExpression, variables: &Vec<Variable>, functions: &Vec<Function>) -> Expression {
    return match expr {
        PartExpression::Number { val, token } => {
            Expression::NumberValue {
                value: BigInt::from(val)
            }
        },
        PartExpression::Identifier { val, token } => {
            if variables.into_iter().any(|var| var.name.eq(&val)) {
                return Expression::VariableAccess {
                    variable: val
                };
            }

            token.err("Variable not found");
        },
        PartExpression::PrefixOperator { prefix, expression, token } => {
            match prefix.as_str() {
                "-" => {
                    let expression = actual_parse_expression(*expression.clone(), &variables.clone(), &functions.clone());

                    // match expression {
                    //     Expression::NumberValue { .. } => {
                    Expression::Math {
                        var1: Box::new(expression.clone()),
                        var2: Box::new(Expression::Math {
                            var1: Box::new(expression),
                            var2: Box::new(Expression::NumberValue {
                                value: BigInt::from(2)
                            }),
                            math: MathType::Multiply
                        }),
                        math: MathType::Subtract
                    }
                        // }
                        // _ => token.err(&format!("Can't apply {} prefix to this", prefix))
                    // }
                }
                _ => token.err("Unknown prefix")
            }
        },
        PartExpression::InfixOperator { operator, left, right, token } => {
            match operator.as_str() {
                "+" | "-" | "*" | "/" | "==" | "<" | ">" | "=!" | "<=" | ">=" => {
                    Expression::Math {
                        var1: Box::new(actual_parse_expression(*left.clone(), &variables.clone(), &functions.clone())),
                        var2: Box::new(actual_parse_expression(*right.clone(), &variables.clone(), &functions.clone())),
                        math: MathType::of(operator)
                    }
                },
                "=" => {
                    // Expression::VariableAssignment {
                    //     var:
                    // }
                    let actual_left = actual_parse_expression(*left.clone(), &variables.clone(), &functions.clone());

                    match actual_left {
                        Expression::VariableAccess { .. } => {},
                        _ => token.err("Expected variable access on left side of infix operator")
                    }

                    // Expression::Math {
                    //     var1: Box::new(actual_parse_expression(*left.clone(), &variables.clone())),
                    //     var2: Box::new(actual_parse_expression(*right.clone(), &variables.clone())),
                    //     math: MathType::Equals
                    // }
                    Expression::VariableAssignment {
                        variable: actual_left.variable_acess_variable().to_owned(),
                        value: Box::new(actual_parse_expression(*right.clone(), &variables.clone(), &functions.clone()))
                    }
                },
                _ => token.err("Unknown infix")
            }
        },
        PartExpression::FunctionInvocation { val, arguments, token } => {
            let name = match *val {
                PartExpression::Identifier { val, .. } => val,
                _ => panic!("Internal error")
            };
            let args = arguments.into_iter().map(|a| actual_parse_expression(a, variables, functions)).collect::<Vec<Expression>>();

            if functions.into_iter().find(|f| f.name.eq(&name) && f.parameters.len() == args.len()).is_none() {
                token.err_neg_offset("Function not found", name.len() as isize);
            }

            Expression::FunctionInvocation {
                function: name,
                arguments: args
            }
        },
        PartExpression::None | PartExpression::Comment => panic!("Can't parse PartExpression::None | PartExpression::Comment")
    };
}

#[derive(PartialEq, Eq, Debug)]
pub enum PartExpression {
    None, // for parsing
    Comment, // for loose expression parsing to work
    Number {
        val: BigInt,
        token: LexedToken
    },
    Identifier {
        val: String,
        token: LexedToken
    },
    PrefixOperator {
        prefix: String,
        expression: Box<PartExpression>,
        token: LexedToken
    },
    InfixOperator {
        operator: String,
        left: Box<PartExpression>,
        right: Box<PartExpression>,
        token: LexedToken
    },
    FunctionInvocation {
        val: Box<PartExpression>,
        arguments: Vec<PartExpression>,
        token: LexedToken
    }
}

impl Clone for PartExpression {
    fn clone(&self) -> Self {
        match self {
            PartExpression::Number { val, token } => {
                PartExpression::Number {
                    val: val.clone(),
                    token: token.clone()
                }
            }
            PartExpression::Identifier { val, token } => {
                PartExpression::Identifier {
                    val: val.to_owned(),
                    token: token.clone()
                }
            }
            PartExpression::PrefixOperator { prefix, expression, token } => {
                PartExpression::PrefixOperator {
                    prefix: prefix.to_owned(),
                    expression: Box::new(*expression.clone()),
                    token: token.clone()
                }
            }
            PartExpression::InfixOperator { operator, left, right, token } => {
                PartExpression::InfixOperator {
                    operator: operator.to_owned(),
                    left: Box::new(*left.clone()),
                    right: Box::new(*right.clone()),
                    token: token.clone()
                }
            }
            PartExpression::FunctionInvocation { val, arguments, token } => {
                PartExpression::FunctionInvocation {
                    val: Box::new(*val.clone()),
                    arguments: arguments.to_vec(),
                    token: token.clone()
                }
            },
            PartExpression::None => PartExpression::None,
            PartExpression::Comment => PartExpression::Comment
        }
    }
}

// impl PartExpression {
//     fn clone(&self) -> PartExpression {
//
//     }
// }

#[derive(Debug)]
pub enum Precedence {
    None,
    Conditional,
    Sum,
    Product,
    FunctionInvocation,
    Prefix,
    Assignment
}

impl Precedence {
    fn entries(&self) -> HashMap<u8, Precedence> {
        let mut map = HashMap::<u8, Precedence>::new();

        for precedence in vec![Precedence::None, Precedence::Conditional, Precedence::Sum, Precedence::Product, Precedence::FunctionInvocation, Precedence::Prefix] {
            map.insert(precedence.order(), precedence);
        }

        return map;
    }

    fn order(&self) -> u8 {
        match *self {
            Precedence::None => 0,
            Precedence::Conditional => 1,
            Precedence::Sum => 2,
            Precedence::Product => 3,
            Precedence::FunctionInvocation => 4,
            Precedence::Prefix => 5,
            Precedence::Assignment => 6
        }
    }

    fn one_less(&self) -> Precedence {
        let order_less = if self.order() == 0 {
            0
        } else {
            self.order() - 1
        };

        // &self.entries().get(&order_less).expect("hmmmmmmmmmmmmmmmmmm")

        let entries = self.entries();
        let entry = entries.get(&order_less).expect("hmmmmmmmmmmmmmmm");

        entry.clone()
    }

    // hmmmmmmmmmmmmmmmmmm
    // why do we need this
    fn clone(&self) -> Precedence {
        match *self {
            Precedence::None => Precedence::None,
            Precedence::Conditional => Precedence::Conditional,
            Precedence::Sum => Precedence::Sum,
            Precedence::Product => Precedence::Product,
            Precedence::FunctionInvocation => Precedence::FunctionInvocation,
            Precedence::Prefix => Precedence::Prefix,
            Precedence::Assignment => Precedence::Assignment
        }
    }
}