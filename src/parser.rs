use crate::ast::{AST, Function, Variable, Expression};
use crate::parser::expression::{PartExpression, actual_parse_expression, Precedence, parse_expression_part};
use crate::lexer::LexedToken;
use crate::interpreter::runtime::ExternalRuntimeFunction;

pub mod expression;

pub fn parse(tokens: Vec<LexedToken>, external_functions: Vec<ExternalRuntimeFunction>) -> AST {
    let mut queue = token_queue(tokens);
    let mut variables = Vec::<Variable>::new();
    let mut functions = external_functions.into_iter().map(map_function).collect::<Vec<Function>>();
    let mut loose_expressions_pre = Vec::<PartExpression>::new();

    queue.purge_all("WHITESPACE");

    // pre parse

    while queue.is_not_empty() {
        let next = queue.peek();

        match next.token_type().id() {
            "LET" => variables.push(pre_parse_variable(&mut queue)),
            "CONST" => {
                let mut var = pre_parse_variable(&mut queue);

                var.constant = true;

                variables.push(var);
            }
            "DEFINE" => functions.push(pre_parse_function(&mut queue)),
            "NEW_LINE" => {}, // do nothing
            _ => {
                queue.back();

                loose_expressions_pre.push(pre_parse_loose_expression(&mut queue));
            }
        }
    }

    // post parse

    let variables_clone = variables.clone();
    let functions_clone = functions.clone();

    variables.iter_mut().for_each(|v| post_parse_variable(v, &variables_clone, &functions_clone));
    functions.iter_mut().filter(|f| Expression::External != f.definition).for_each(|f| post_parse_function(f, &variables_clone, &functions_clone));

    let mut loose_expressions = Vec::<Expression>::new();

    for expr in loose_expressions_pre {
        if PartExpression::Comment == expr { // filter out comments
            continue;
        }

        loose_expressions.push(actual_parse_expression(expr, &variables, &functions));
    }

    AST {
        functions,
        variables,
        loose_expressions
    }
}

fn map_function(f: ExternalRuntimeFunction) -> Function {
    Function {
        name: f.name().to_owned(),
        definition: Expression::External,
        parameters: (0..*f.parameters()).map(|i| format!("p{}", i)).collect::<Vec<String>>(),
        pre_definition: PartExpression::None,
        cached: false
    }
}

fn pre_parse_loose_expression(queue: &mut TokenQueue) -> PartExpression {
    let mut lines_left = 1;
    let mut actual_tokens = Vec::<LexedToken>::new();

    while lines_left > 0 && queue.is_not_empty() {
        let next = queue.peek();

        match next.token_type().id() {
            "PIPE" => lines_left += 1,
            "NEW_LINE" => lines_left -= 1,
            _ => actual_tokens.push(next)
        }
    }

    if actual_tokens.is_empty() { // probably a comment
        return PartExpression::Comment;
    }

    parse_expression_part(&mut token_queue(actual_tokens), Precedence::None)
}

fn post_parse_variable(var: &mut Variable, variables: &Vec<Variable>, functions: &Vec<Function>) {
    var.definition = actual_parse_expression(var.pre_definition.clone(), variables, functions);

    for expr in &var.pre_wherepart {
        var.wherepart.push(actual_parse_expression(expr.clone(), variables, functions));
    }

    // clear pre definition/wherepart

    var.pre_definition = PartExpression::None;
    var.pre_wherepart.clear();
}

fn pre_parse_variable(queue: &mut TokenQueue) -> Variable {
    let mut name = String::new();
    let mut definition = PartExpression::None;
    let /* mut */ wherepart = Vec::<PartExpression>::new();
    let mut lines_left = 1;

    while lines_left > 0 && queue.is_not_empty() {
        let next = queue.peek();

        match next.token_type().id() {
            "PIPE" => lines_left += 1,
            "NEW_LINE" => lines_left -= 1,
            "ASSIGN" => {
                if name.is_empty() {
                    next.err("Expected identifier");
                } else if PartExpression::None != definition {
                    next.err("Invalid token");
                }

                let mut expr_queue_vec = Vec::<LexedToken>::new();

                loop {
                    let get = queue.peek();
                    let id = get.token_type().id();

                    if id.eq("NEW_LINE") {
                        lines_left -= 1;

                        break;
                    } else if id.eq("PIPE") {
                        lines_left += 1;

                        break;
                    }

                    if id.eq("WHERE") || lines_left == 0 {
                        queue.back();

                        break;
                    }

                    expr_queue_vec.push(get.clone());
                }

                let mut expr_queue = token_queue(expr_queue_vec);

                definition = parse_expression_part(&mut expr_queue, Precedence::None);
            },
            "IDENTIFIER" => {
                if !name.is_empty() {
                    next.err(&format!("Invalid token ('{}')", next.content()));
                }

                name = next.content().to_owned();
            },
            "WHERE" => {
                if name.is_empty() {
                    next.err("Expected identifier");
                } else if PartExpression::None == definition {
                    next.err("Expected definition");
                }

                // wherepart = read_where(queue);

                todo!("where part");
            },
            _ => {
                if !name.is_empty() {
                    next.err("Expected =");
                }

                next.err("Expected identifier");
            }
        }
    }

    Variable {
        name,
        definition: Expression::None, // do in post parse so that we can do lookahead variable parsing etc...
        wherepart: vec![],
        pre_definition: definition,
        pre_wherepart: wherepart,
        constant: false
    }
}

fn post_parse_function(func: &mut Function, variables: &Vec<Variable>, functions: &Vec<Function>) {
    let mut vars = variables.clone();

    for param in &func.parameters {
        vars.push(fake_variable(param.to_owned()));
    }

    func.definition = actual_parse_expression(func.pre_definition.clone(), &vars, functions);

    // clear pre definition

    func.pre_definition = PartExpression::None;
}

fn fake_variable(name: String) -> Variable {
    Variable {
        name,
        definition: Expression::None,
        wherepart: vec![],
        pre_definition: PartExpression::None,
        pre_wherepart: vec![],
        constant: false
    }
}

fn pre_parse_function(queue: &mut TokenQueue) -> Function {
    let mut name = String::new();
    let mut definition = PartExpression::None;
    let mut parameters = Vec::<String>::new();
    let mut lines_left = 1;
    let mut cached = false;

    while lines_left > 0 && queue.is_not_empty() {
        let next = queue.peek();

        match next.token_type().id() {
            "PIPE" => lines_left += 1,
            "NEW_LINE" => lines_left -= 1,
            "OPEN_PARENTHESIS" => {
                if name.is_empty() {
                    next.err("Expected identifier");
                } else if PartExpression::None != definition {
                    next.err("Invalid token");
                }

                let mut expr_queue_vec = Vec::<LexedToken>::new();

                loop {
                    let get = queue.peek();
                    let id = get.token_type().id();

                    if id.eq("NEW_LINE") {
                        lines_left -= 1;

                        break;
                    } else if id.eq("PIPE") {
                        lines_left += 1;

                        break;
                    }

                    if lines_left == 0 || id.eq("ASSIGN") {
                        queue.back();

                        break;
                    }

                    expr_queue_vec.push(get.clone());
                }

                let mut expr_queue = token_queue(expr_queue_vec);
                let mut first = true;

                while expr_queue.is_not_empty() {
                    let next = expr_queue.peek();
                    let token = next.token_type().id().to_owned();

                    if first {
                        first ^= true;

                        if token.eq("CLOSE_PARENTHESIS") {
                            break;
                        }

                        if token.ne("IDENTIFIER") {
                            next.err("Identifier expected");
                        }

                        parameters.push(next.content().to_owned());
                    } else {
                        match token.as_str() {
                            "CLOSE_PARENTHESIS" => break,
                            "COMMA" => parameters.push(expr_queue.peek().check_id("IDENTIFIER", "Identifier expected").content().to_owned()),
                            _ => next.err("CLOSE_PARENTHESIS or COMMA expected")
                        }
                    }
                }
            }
            "ASSIGN" => {
                if name.is_empty() {
                    next.err("Expected identifier");
                } else if PartExpression::None != definition {
                    next.err("Invalid token");
                }

                let mut expr_queue_vec = Vec::<LexedToken>::new();

                loop {
                    let get = queue.peek();
                    let id = get.token_type().id();

                    if id.eq("NEW_LINE") {
                        lines_left -= 1;

                        break;
                    } else if id.eq("PIPE") {
                        lines_left += 1;

                        break;
                    }

                    if lines_left == 0 {
                        queue.back();

                        break;
                    }

                    expr_queue_vec.push(get.clone());
                }

                let mut expr_queue = token_queue(expr_queue_vec);

                definition = parse_expression_part(&mut expr_queue, Precedence::None);
            },
            "IDENTIFIER" => {
                if !name.is_empty() {
                    next.err(&format!("Invalid token ('{}')", next.content()));
                }

                name = next.content().to_owned();
            },
            "CACHE" => cached = true,
            _ => {
                if !name.is_empty() {
                    next.err("Expected =");
                }

                next.err("Expected identifier");
            }
        }
    }

    Function {
        name,
        definition: Expression::None,
        parameters,
        pre_definition: definition,
        cached
    }
}

pub fn token_queue(elements: Vec<LexedToken>) -> TokenQueue {
    TokenQueue {
        elements,
        pointer: 0
    }
}

#[derive(Debug)]
pub struct TokenQueue {
    elements: Vec<LexedToken>,
    pointer: usize
}

impl TokenQueue {
    pub fn peek(&mut self) -> LexedToken {
        let get = self.get().clone();

        self.remove();

        get
    }

    pub fn back(&mut self) {
        self.pointer -= 1;
    }

    pub fn get(&self) -> &LexedToken {
        &self.elements.get(self.pointer).expect("Out of bounds")
    }

    pub fn remove(&mut self) {
        self.pointer += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.elements.len() - self.pointer <= 0
    }

    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }

    pub fn push(&mut self, token: LexedToken) {
        self.elements.push(token);
    }

    pub fn extend(&mut self, other: TokenQueue) {
        self.elements.extend(other.elements);
    }

    pub fn extend_left(&mut self, other: TokenQueue) {
        self.elements.extend(other.elements[other.pointer..].iter().cloned());
    }

    pub fn purge_all(&mut self, id: &'static str) {
        self.elements.retain(|t| t.token_type().id().ne(id))
    }
}