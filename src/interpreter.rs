use crate::ast::{AST, Expression, MathType, Function, Variable};
use num_bigint::BigInt;
use std::ops::{Add, Sub, Mul, Div};
use crate::interpreter::runtime::{RuntimeAST, RuntimeExpression, Tuple, RuntimeFunction, RuntimeVariable, ExternalRuntimeFunction};

pub mod runtime;

pub fn interpret(ast: AST, external_functions: Vec<ExternalRuntimeFunction>) {
    let mut runtime = RuntimeAST::create(ast.clone(), external_functions);
    let exprs = ast.loose_expressions.clone().into_iter().map(|expr| RuntimeExpression::from(expr, &runtime)).collect::<Vec<RuntimeExpression>>();

    for expr in exprs {
        expr.execute(&mut runtime);
    }
}

impl RuntimeAST {
    pub fn create(ast: AST, external_functions: Vec<ExternalRuntimeFunction>) -> Self {
        RuntimeAST {
            variables: ast.variables.into_iter().map(|v| RuntimeVariable::from_raw(v)).collect::<Vec<RuntimeVariable>>(),
            functions: ast.functions.into_iter().filter(|f| Expression::External != f.definition).map(|f| RuntimeFunction::from(f)).collect::<Vec<RuntimeFunction>>(),
            external_functions
        }
    }

    pub fn function_ast(mut orig: RuntimeAST, func: RuntimeFunction, args: Vec<RuntimeExpression>) -> RuntimeAST {
        let mut vars = orig.variables.clone().into_iter().filter(|v| !v.function_argument).collect::<Vec<RuntimeVariable>>().clone();
        let mut ptr = 0;

        for param in func.parameters {
            let arg = args.get(ptr).unwrap().clone();

            vars.push(RuntimeVariable {
                name: param,
                definition: RuntimeExpression {
                    orig: if !arg.is_pointer {
                        Expression::NumberValue {
                            value: arg.execute(&mut orig)
                        }
                    } else {
                        Expression::None
                    },
                    is_pointer: arg.is_pointer,
                    pointer_to: arg.pointer_to
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
            let result = fun.invoke(args.clone(), self);
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
        let found = self.variables.clone().into_iter().find(|v| v.name.eq(&name)).unwrap();
        let definition = found.definition.clone();

        if definition.is_pointer {
            self.reassign_variable(definition.pointer_to.clone().unwrap(), val.clone());
        } else {
            self.variables = self.variables.clone().into_iter().map(|v| {
                if v.name.eq(&found.name) {
                    RuntimeVariable {
                        name: v.name,
                        definition: RuntimeExpression {
                            orig: Expression::NumberValue {
                                value: val.clone()
                            },
                            is_pointer: false,
                            pointer_to: Box::new(None)
                        },
                        function_argument: v.function_argument
                    }
                } else {
                    v
                }
            }).collect::<Vec<RuntimeVariable>>();
        }

        val
    }
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

impl RuntimeVariable {
    pub fn from(orig: Variable, ast: &RuntimeAST) -> Self {
        Self {
            name: orig.name,
            definition: RuntimeExpression::from(orig.definition, ast),
            function_argument: false
        }
    }

    pub fn from_raw(orig: Variable) -> Self {
        Self {
            name: orig.name,
            definition: RuntimeExpression {
                orig: orig.definition,
                is_pointer: false,
                pointer_to: Box::new(None)
            },
            function_argument: false
        }
    }

    pub fn get_value(&self, ast: &mut RuntimeAST) -> BigInt {
        self.definition.execute(ast)
    }
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
        Self {
            name: orig.name,
            definition: RuntimeExpression {
                orig: orig.definition,
                is_pointer: false,
                pointer_to: Box::new(None)
            },
            parameters: orig.parameters,
            cached: orig.cached,
            cache: vec![]
        }
    }

    pub fn invoke(&mut self, args: Vec<RuntimeExpression>, ast: &mut RuntimeAST) -> BigInt {
        return if self.cached {
            let ptr = args.clone().into_iter().find(|expr| expr.is_pointer);

            if ptr.is_some() {
                panic!("Cannot invoke cached function with pointer (TODO make this error better)");
            }

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
                let mut ptr = 0;
                let mut vars = Vec::<RuntimeVariable>::new();

                for param in &self.parameters {
                    let arg = args.get(ptr).unwrap().clone();

                    vars.push(RuntimeVariable {
                        name: param.clone(),
                        definition: RuntimeExpression {
                            orig: if !arg.is_pointer {
                                Expression::NumberValue {
                                    value: arg.execute(ast)
                                }
                            } else {
                                Expression::None
                            },
                            is_pointer: arg.is_pointer,
                            pointer_to: arg.pointer_to
                        },
                        function_argument: true
                    });

                    ptr += 1;
                }

                ast.variables.extend(vars);

                let result = self.definition.execute(ast);

                ast.variables = ast.variables.clone().into_iter().filter(|v| !v.function_argument).collect::<Vec<RuntimeVariable>>();

                let tuple = Tuple {
                    a: args.clone(),
                    b: result.clone()
                };

                for fun in &ast.functions {
                    for t in &fun.cache {
                        if !self.cache.contains(t) {
                            self.cache.push(t.clone());
                        }
                    }
                }

                self.cache.push(tuple);

                result
            }
        } else {
            let mut ptr = 0;
            let mut vars = Vec::<RuntimeVariable>::new();

            for param in &self.parameters {
                let arg = args.get(ptr).unwrap().clone();

                vars.push(RuntimeVariable {
                    name: param.clone(),
                    definition: RuntimeExpression {
                        orig: if !arg.is_pointer {
                            Expression::NumberValue {
                                value: arg.execute(ast)
                            }
                        } else {
                            Expression::NumberValue {
                                value: arg.pointer_to.clone().unwrap().get_value(ast)
                            }
                        },
                        is_pointer: arg.is_pointer,
                        pointer_to: arg.pointer_to
                    },
                    function_argument: true
                });

                ptr += 1;
            }

            ast.variables.extend(vars);

            let result = self.definition.execute(ast);

            ast.variables = ast.variables.clone().into_iter().filter(|v| !v.function_argument).collect::<Vec<RuntimeVariable>>();

            result
        }
    }
}

impl PartialEq<RuntimeExpression> for RuntimeExpression {
    fn eq(&self, other: &RuntimeExpression) -> bool {
        other.orig.eq(self.orig()) && other.is_pointer.eq(&self.is_pointer) && other.pointer_to.clone().unwrap().eq(&self.pointer_to.clone().unwrap())
    }

    fn ne(&self, other: &RuntimeExpression) -> bool {
        !self.eq(other)
    }
}

impl PartialEq<RuntimeVariable> for RuntimeVariable {
    fn eq(&self, other: &RuntimeVariable) -> bool {
        self.function_argument.eq(&other.function_argument.clone()) && self.definition.eq(&other.definition.clone()) && self.name.eq(&other.name.clone())
    }

    fn ne(&self, other: &RuntimeVariable) -> bool {
        !self.eq(other)
    }
}

impl RuntimeExpression {
    pub fn empty() -> Self {
        Self {
            orig: Expression::None,
            is_pointer: false,
            pointer_to: Box::new(None)
        }
    }

    pub fn from(orig: Expression, ast: &RuntimeAST) -> Self {
        RuntimeExpression {
            orig: orig.clone(),
            is_pointer: match &orig {
                Expression::Pointer { .. } => true,
                _ => false
            },
            pointer_to: Box::new(match orig.clone() {
                Expression::Pointer { to } => Some(ast.lookup_variable(&to)),
                _ => None
            })
        }
    }

    pub fn orig(&self) -> &Expression {
        &self.orig
    }

    pub fn execute(&self, ast: &mut RuntimeAST) -> BigInt {
        if self.is_pointer {
            self.pointer_to.clone().unwrap().get_value(ast)
        } else {
            RuntimeExpression::execute_expr(&self.orig, ast)
        }
    }

    pub fn execute_expr(expr: &Expression, ast: &mut RuntimeAST) -> BigInt {
        match expr {
            Expression::NumberValue { value } =>
                value.clone(),
            Expression::VariableAccess { variable } =>
                ast.lookup_variable(&variable.to_owned()).get_value(ast),
            Expression::Math { var1, var2, math } =>
                RuntimeExpression::run_math(math.clone(), RuntimeExpression::from(*var1.clone(), ast), RuntimeExpression::from(*var2.clone(), ast), ast),
            Expression::FunctionInvocation { function, arguments } =>
                ast.invoke_function(&function.to_owned(), arguments.into_iter().map(|expr| RuntimeExpression::from(expr.clone(), ast)).collect::<Vec<RuntimeExpression>>()),
            Expression::VariableAssignment { variable, value } => {
                let val = RuntimeExpression::from(*value.clone(), ast).execute(ast);

                ast.reassign_variable(ast.lookup_variable(&variable.to_owned()), val)
            },
            Expression::None | Expression::External | Expression::Pointer { .. } =>
                panic!("Can not execute Expression::None | Expression::External | Expression::Pointer => {}", RuntimeExpression::expr_to_string(expr)),
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
            MathType::Pow               => var1.execute(ast).pow(*var2.execute(ast).to_u32_digits().1.last().unwrap())
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
            Expression::Pointer { to } => format!("*{}", to)
        }
    }
}