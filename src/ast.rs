use std::collections::hash_map::Entry;
use crate::expression_parser::PartExpression;
use std::env::VarError::NotPresent;
use std::process::exit;
use num_bigint::BigInt;

#[derive(Debug)]
pub struct AST { // AST = Abstract Syntax Tree
    pub functions: Vec<Function>,
    pub variables: Vec<Variable>,
    pub loose_expressions: Vec<Expression>
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub definition: Expression,
    pub parameters: Vec<String>,
    pub pre_definition: PartExpression,
    pub cached: bool
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub definition: Expression,
    pub wherepart: Vec<Expression>,
    pub pre_definition: PartExpression,
    pub pre_wherepart: Vec<PartExpression>
}

#[derive(Debug, Eq, PartialEq)]
pub enum Expression {
    None, // for parsing
    External/* {*/  // for external functions
        /*function: String,
        arguments: Vec<Expression>
    }*/,
    NumberValue {
        value: BigInt
    },
    VariableAccess {
        variable: String
    },
    Math {
        var1: Box<Expression>,
        var2: Box<Expression>,
        math: MathType
    },
    FunctionInvocation {
        function: String,
        arguments: Vec<Expression>
    },
    VariableAssignment {
        variable: String,
        value: Box<Expression>
    }
}

impl Clone for Expression {
    fn clone(&self) -> Self {
        match self {
            Expression::None => Expression::None,
            Expression::External => Expression::External,
            Expression::NumberValue { value } => Expression::NumberValue { value: value.clone() },
            Expression::VariableAccess { variable } => Expression::VariableAccess { variable: variable.to_owned() },
            Expression::Math { var1, var2, math } => Expression::Math { var1: var1.to_owned(), var2: var2.to_owned(), math: math.clone() },
            Expression::FunctionInvocation { function, arguments } => Expression::FunctionInvocation { function: function.to_owned(), arguments: arguments.clone() },
            Expression::VariableAssignment { variable, value } => Expression::VariableAssignment { variable: variable.to_owned(), value: value.to_owned() },
        }
    }
}

impl Expression {
    pub fn variable_acess_variable(&self) -> &String {
        match self {
            Expression::VariableAccess { variable } => variable,
            _ => panic!("Not supported")
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum MathType {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    BiggerOrEquals,
    Bigger,
    SmallerOrEquals,
    Smaller,
    Pow
}

impl MathType {
    pub fn operator(&self) -> String {
        match *self {
            MathType::Add => "+",
            MathType::Subtract => "-",
            MathType::Multiply => "*",
            MathType::Divide => "/",
            MathType::Equals => "==",
            MathType::NotEquals => "=!",
            MathType::BiggerOrEquals => ">=",
            MathType::Bigger => ">",
            MathType::SmallerOrEquals => "<=",
            MathType::Smaller => "<",
            MathType::Pow => "^"
        }.to_owned()
    }

    fn entries() -> Vec<MathType> {
        vec![MathType::Add, MathType::Subtract, MathType::Multiply, MathType::Divide, MathType::Equals, MathType::NotEquals, MathType::BiggerOrEquals, MathType::Bigger, MathType::SmallerOrEquals, MathType::Smaller, MathType::Pow]
    }

    pub fn of(operator: String) -> MathType {
        MathType::entries().into_iter().find(|m| m.operator().eq(&operator)).expect(&format!("Operator not found ('{}')", operator))
    }
}