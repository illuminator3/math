use crate::ast::{AST, Expression, MathType, Function, Variable};
use std::env::var;
use num_bigint::BigInt;
use std::ops::{Add, Sub, Mul, Div};
use std::collections::HashMap;
use std::ptr::eq;

pub fn interpret(ast: AST, external_functions: Vec<ExternalRuntimeFunction>) {
    let exprs = ast.loose_expressions.clone().into_iter().map(RuntimeExpression::from).collect::<Vec<RuntimeExpression>>();
    let mut runtime = RuntimeAST::create(ast, external_functions);

    for expr in exprs {
        expr.execute(&mut runtime);
    }
}

#[derive(Clone)]
pub struct RuntimeAST {
    variables: Vec<RuntimeVariable>,
    functions: Vec<RuntimeFunction>,
    external_functions: Vec<ExternalRuntimeFunction>
}

impl RuntimeAST {
    pub fn create(ast: AST, external_functions: Vec<ExternalRuntimeFunction>) -> Self {
        RuntimeAST {
            variables: ast.variables.into_iter().map(|v| RuntimeVariable::from(v)).collect::<Vec<RuntimeVariable>>(),
            functions: ast.functions.into_iter().filter(|f| Expression::External != f.definition).map(|f| RuntimeFunction::from(f)).collect::<Vec<RuntimeFunction>>(),
            external_functions
        }
    }

    pub fn function_ast(orig: RuntimeAST, func: RuntimeFunction, args: Vec<BigInt>) -> RuntimeAST {
        let mut vars = orig.variables.into_iter().filter(|v| !v.function_argument).collect::<Vec<RuntimeVariable>>().clone();
        let mut ptr = 0;

        for param in func.parameters {
            vars.push(RuntimeVariable {
                name: param,
                definition: RuntimeExpression {
                    orig: Expression::NumberValue {
                        value: args.get(ptr).unwrap().clone()
                    }
                },
                function_argument: true
            });

            ptr += 1;
        }

        RuntimeAST {
            variables: vars,
            functions: orig.functions.clone(),
            external_functions: orig.external_functions
        }
    }

    pub fn get_functions(&self) -> &Vec<RuntimeFunction> {
        &self.functions
    }

    pub fn get_variables(&self) -> &Vec<RuntimeVariable> {
        &self.variables
    }

    pub fn delete_function(&mut self, name: &str, params: usize) {
        self.functions.retain(|f| f.name.ne(name) && f.parameters.len() != params);
    }

    pub fn delete_variable(&mut self, name: &str) {
        self.variables.retain(|v| v.name.ne(name));
    }

    pub fn lookup_variable(&self, name: &str) -> RuntimeVariable {
        self.variables.clone().into_iter().find(|v| v.name.eq(name)).unwrap()
    }

    pub fn lookup_function(&self, name: &str, params: usize) -> RuntimeFunction {
        self.functions.clone().into_iter().find(|f| f.name.eq(name) && f.parameters.len() == params).unwrap()
    }

    pub fn lookup_external_function(&self, name: &str, params: usize) -> ExternalRuntimeFunction {
        self.external_functions.clone().into_iter().find(|f| f.name.eq(name) && f.parameters == params).unwrap()
    }

    pub fn function_exists(&self, name: &str, params: usize) -> bool {
        self.functions.clone().into_iter().find(|f| f.name.eq(name) && f.parameters.len() == params).is_some()
    }

    pub fn external_function_exists(&self, name: &str, params: usize) -> bool {
        self.external_functions.clone().into_iter().find(|f| f.name.eq(name) && f.parameters == params).is_some()
    }

    pub fn invoke_function(&mut self, name: &str, args: Vec<RuntimeExpression>) -> BigInt {
        return if self.function_exists(name, args.len()) {
            let mut fun = self.lookup_function(name, args.len());

            let result = fun.invoke(args.clone().into_iter().map(|expr| expr.execute(self)).collect::<Vec<BigInt>>(), self);
            let cache = fun.cache;

            self.functions = self.functions.clone().into_iter().map(|mut f| if f.name.eq(name) && f.parameters.len() == args.len() {
                f.cache = cache.clone();

                f
            } else {
                f
            }).collect::<Vec<RuntimeFunction>>();

            result
        } else if self.external_function_exists(name, args.len()) {
            (self.lookup_external_function(name, args.len()).invoke)(args, self)
        } else {
            panic!("Something went wrong (FUNCTION NOT FOUND)")
        }
    }

    pub fn reassign_variable(&mut self, var: RuntimeVariable, val: BigInt) -> BigInt {
        let name = var.name;

        self.variables.iter_mut().find(|v| v.name.eq(&name)).unwrap().definition = RuntimeExpression {
            orig: Expression::NumberValue {
                value: val.clone()
            }
        };

        val
    }
}

#[derive(Clone)]
pub struct ExternalRuntimeFunction {
    name: String,
    parameters: usize,
    invoke: fn(Vec<RuntimeExpression>, &mut RuntimeAST) -> BigInt
}

impl ExternalRuntimeFunction {
    pub fn create(name: &'static str, parameters: usize, invoke: fn(Vec<RuntimeExpression>, &mut RuntimeAST) -> BigInt) -> ExternalRuntimeFunction {
        ExternalRuntimeFunction {
            name: name.to_owned(),
            parameters,
            invoke
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn parameters(&self) -> &usize {
        &self.parameters
    }

    pub fn invoke(&self) -> &fn(Vec<RuntimeExpression>, &mut RuntimeAST) -> BigInt {
        &self.invoke
    }
}

#[derive(Clone)]
pub struct RuntimeVariable {
    name: String,
    definition: RuntimeExpression,
    function_argument: bool
}

impl RuntimeVariable {
    pub fn from(orig: Variable) -> Self {
        RuntimeVariable {
            name: orig.name,
            definition: RuntimeExpression::from(orig.definition),
            function_argument: false
        }
    }

    pub fn get_value(&self, ast: &mut RuntimeAST) -> BigInt {
        self.definition.execute(ast)
    }
}

#[derive(Clone)]
pub struct RuntimeFunction {
    name: String,
    definition: RuntimeExpression,
    parameters: Vec<String>,
    cached: bool,
    cache: Vec<Tuple<Vec<BigInt>, BigInt>>
}

#[derive(Clone, Debug)]
struct Tuple<A: PartialEq, B: PartialEq> {
    a: A,
    b: B
}

impl<A: PartialEq, B: PartialEq> PartialEq for Tuple<A, B> {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl RuntimeFunction {
    pub fn from(orig: Function) -> Self {
        RuntimeFunction {
            name: orig.name,
            definition: RuntimeExpression::from(orig.definition),
            parameters: orig.parameters,
            cached: orig.cached,
            cache: vec![]
        }
    }

    pub fn invoke(&mut self, args: Vec<BigInt>, ast: &mut RuntimeAST) -> BigInt {
        // return if self.cached {
        //     println!("cached");
        //
        //     let key = &*args;
        //
        //     println!("key {:?}", key);
        //
        //     if self.cache.contains_key(key) {
        //         println!("contains_key");
        //
        //         self.cache.get(key).unwrap().clone()
        //     } else {
        //         println!("recache");
        //
        //         let result = self.definition.execute(RuntimeAST::function_ast(ast.clone(), self.clone(), args.clone()));
        //
        //         println!("result {:?}", result);
        //
        //         self.cache.insert(args.clone(), result.clone());
        //
        //         result
        //     }
        // } else {
        //     self.definition.execute(RuntimeAST::function_ast(ast.clone(), self.clone(), args))
        // }
        return if self.cached {
            let mut found = None;

            for t in self.cache.clone() {
                let a = t.a;
                let r = t.b;

                if a == args {
                    found = Some(r);

                    break;
                }
            }

            if None != found {
                found.unwrap()
            } else {
                let mut ast = RuntimeAST::function_ast(ast.clone(), self.clone(), args.clone());
                let result = self.definition.execute(&mut ast);
                let tuple = Tuple {
                    a: args.clone(),
                    b: result.clone()
                };

                for fun in ast.functions {
                    for t in fun.cache {
                        if !self.cache.contains(&t) {
                            self.cache.push(t);
                        }
                    }
                }

                self.cache.push(tuple);

                result
            }
        } else {
            self.definition.execute(&mut RuntimeAST::function_ast(ast.clone(), self.clone(), args))
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeExpression {
    orig: Expression
}

impl RuntimeExpression {
    pub fn from(orig: Expression) -> Self {
        RuntimeExpression {
            orig
        }
    }

    pub fn orig(&self) -> &Expression {
        &self.orig
    }

    pub fn execute(&self, ast: &mut RuntimeAST) -> BigInt {
        RuntimeExpression::execute_expr(&self.orig, ast)
    }

    pub fn execute_expr(expr: &Expression, ast: &mut RuntimeAST) -> BigInt {
        // println!("execute_expr {:?}", RuntimeExpression::expr_to_string(&expr));

        match expr {
            Expression::NumberValue { value } => value.clone(),
            Expression::VariableAccess { variable } => ast.lookup_variable(&variable.to_owned()).get_value(ast),
            Expression::Math { var1, var2, math } => RuntimeExpression::run_math(math.clone(), RuntimeExpression::from(*var1.clone()), RuntimeExpression::from(*var2.clone()), ast),
            Expression::FunctionInvocation { function, arguments } => ast.invoke_function(&function.to_owned(), arguments.into_iter().map(|expr| RuntimeExpression::from(expr.clone())).collect::<Vec<RuntimeExpression>>()),
            Expression::VariableAssignment { variable, value } => {
                let val = RuntimeExpression::from(*value.clone()).execute(ast);

                ast.reassign_variable(ast.lookup_variable(&variable.to_owned()), val)

                // ast.reassign_variable(ast.lookup_variable(&variable.to_owned()), RuntimeExpression::from(*value.clone()).execute(ast));
            },
            Expression::None | Expression::External => panic!("Can not execute Expression::None | Expression::External")
        }
    }

    pub fn run_math(math: MathType, var1: RuntimeExpression, var2: RuntimeExpression, ast: &mut RuntimeAST) -> BigInt {
        match math {
            MathType::Add               => var1.execute(ast).add(var2.execute(ast)),
            MathType::Subtract          => var1.execute(ast).sub(var2.execute(ast)),
            MathType::Multiply          => var1.execute(ast).mul(var2.execute(ast)),
            MathType::Divide            => var1.execute(ast).div(var2.execute(ast)),
            MathType::Equals            => BigInt::from(if var1.execute(ast) == var2.execute(ast) { 1 } else { 0 }),
            MathType::NotEquals         => BigInt::from(if var1.execute(ast) != var2.execute(ast) { 1 } else { 0 }),
            MathType::BiggerOrEquals    => BigInt::from(if var1.execute(ast) >= var2.execute(ast) { 1 } else { 0 }),
            MathType::Bigger            => BigInt::from(if var1.execute(ast) > var2.execute(ast) { 1 } else { 0 }),
            MathType::SmallerOrEquals   => BigInt::from(if var1.execute(ast) <= var2.execute(ast) { 1 } else { 0 }),
            MathType::Smaller           => BigInt::from(if var1.execute(ast) < var2.execute(ast) { 1 } else { 0 }),
        }
    }

    pub fn expr_to_string(expr: &Expression) -> String {
        match expr {
            Expression::None => "none".to_owned(),
            Expression::External => "external".to_owned(),
            Expression::NumberValue { value } => value.to_string(),
            Expression::VariableAccess { variable } => variable.to_owned(),
            Expression::Math { var1, var2, math } => format!("({}) {} ({})", RuntimeExpression::expr_to_string(var1), math.operator(), RuntimeExpression::expr_to_string(var2)),
            Expression::FunctionInvocation { function, arguments } => format!("{}({})", function, arguments.into_iter().map(|expr| RuntimeExpression::expr_to_string(expr)).collect::<Vec<String>>().join(", ")),
            Expression::VariableAssignment { variable, value } => format!("{} = {}", variable, RuntimeExpression::expr_to_string(value)),
            _ => "".to_owned()
        }
    }
}