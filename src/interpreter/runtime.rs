use num_bigint::BigInt;
use crate::ast::Expression;

#[derive(Clone)]
pub struct RuntimeAST {
    pub variables: Vec<RuntimeVariable>,
    pub functions: Vec<RuntimeFunction>,
    pub external_functions: Vec<ExternalRuntimeFunction>
}

#[derive(Clone)]
pub struct ExternalRuntimeFunction {
    pub name: String,
    pub parameters: usize,
    pub invoke: fn(Vec<RuntimeExpression>, &mut RuntimeAST) -> BigInt
}

#[derive(Clone, Debug)]
pub struct RuntimeVariable {
    pub name: String,
    pub definition: RuntimeExpression,
    pub function_argument: bool
}

#[derive(Clone)]
pub struct RuntimeFunction {
    pub name: String,
    pub definition: RuntimeExpression,
    pub parameters: Vec<String>,
    pub cached: bool,
    pub cache: Vec<Tuple<Vec<RuntimeExpression>, BigInt>>
}

#[derive(Clone, Debug)]
pub struct Tuple<A: PartialEq, B: PartialEq> {
    pub a: A,
    pub b: B
}

#[derive(Clone, Debug)]
pub struct RuntimeExpression {
    pub orig: Expression,
    pub is_pointer: bool,
    pub pointer_to: Box<Option<RuntimeVariable>>
}