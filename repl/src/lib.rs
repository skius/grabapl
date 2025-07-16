use chumsky::prelude::*;

//================================================================================
// Abstract Syntax Tree (AST) - (No changes needed here)
//================================================================================

#[derive(Debug, Clone)]
pub struct Program(pub Vec<FuncDef>);

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_params: Vec<TypedParam>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParam {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone)]
pub struct Block(pub Vec<Statement>);

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStmt),
    If(IfStmt),
    Expr(Expr),
    Return(ReturnStmt),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub var_name: String,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Condition,
    pub true_branch: Block,
    pub false_branch: Option<Block>,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Shape(Vec<ShapePattern>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum ShapePattern {
    Node { name: String, ty: String },
    Edge { src: String, dst: String, ty: String },
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub values: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Call {
        func: String,
        args: Vec<Expr>,
    },
    BuiltinCall {
        func: String,
        op: String,
        args: Vec<Expr>,
    },
    Literal(Literal),
    Ident(String),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
}

//================================================================================
// Parser Definition
//================================================================================

type ParserInput<'a> = &'a str;
// The error type now uses 'a to match the input lifetime.
type ParserError<'a> = extra::Err<Simple<'a, char>>;

pub fn parser<'a>() -> impl Parser<'a, ParserInput<'a>, Program, ParserError<'a>> {
    let ident = text::ident().padded();

    let int_literal = just('-')
        .or_not()
        .then(text::int(10))
        .to_slice()
        .from_str()
        .unwrapped()
        .padded();

    let mut expr = Recursive::declare();
    let mut statement = Recursive::declare();

    let args = expr
        .clone()
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>() // Added .collect() to create the Vec
        .delimited_by(just('('), just(')'))
        .padded();

    let call = ident
        .then(
            ident
                .delimited_by(just('['), just(']'))
                .padded()
                .or_not(),
        )
        .then(args)
        // FIX 1: Add explicit type annotations to the closure parameters.
        // This helps the compiler resolve the types for `func`, `op`, and `args`.
        // .map(|((func, op): (&str, Option<&str>), args: Vec<Expr>)| match op {
        .map(|((func, op), args): ((&str, Option<&str>), Vec<Expr>)| match op {
            Some(op) => Expr::BuiltinCall {
                func: func.to_string(),
                op: op.to_string(),
                args,
            },
            None => Expr::Call {
                func: func.to_string(),
                args,
            },
        });

    let atom = choice((
        int_literal.map(Literal::Int).map(Expr::Literal),
        call,
        ident.map(|i: &str| Expr::Ident(i.to_string())),
    ))
        .padded();

    expr.define(atom);

    let block = statement
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded())
        .map(Block);

    let let_stmt = text::keyword("let!")
        .ignore_then(ident)
        .then_ignore(just('=').padded())
        .then(expr.clone())
        .then_ignore(just(';').padded())
        .map(|(var_name, expr): (&str, Expr)| {
            Statement::Let(LetStmt {
                var_name: var_name.to_string(),
                expr,
            })
        });

    let shape_pattern = {
        let node_pattern = text::keyword("node")
            .ignore_then(ident)
            .then_ignore(just(':').padded())
            .then(ident)
            .map(|(name, ty): (&str, &str)| ShapePattern::Node {
                name: name.to_string(),
                ty: ty.to_string(),
            });

        let edge_pattern = text::keyword("edge")
            .ignore_then(ident)
            .then_ignore(just("->").padded())
            .then(ident)
            .then_ignore(just(':').padded())
            .then(ident)
            .map(|((src, dst), ty): ((&str, &str), &str)| ShapePattern::Edge {
                src: src.to_string(),
                dst: dst.to_string(),
                ty: ty.to_string(),
            });

        choice((node_pattern, edge_pattern))
    };

    let shape_query = text::keyword("shape")
        .ignore_then(
            shape_pattern
                .separated_by(just(',').padded())
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just('[').padded(), just(']').padded()),
        )
        .map(Condition::Shape);

    let condition = choice((shape_query, expr.clone().map(Condition::Expr))).padded();

    let if_stmt = recursive(|if_stmt| {
        text::keyword("if")
            .ignore_then(condition)
            .then(block.clone())
            .then(
                text::keyword("else")
                    .padded()
                    .ignore_then(if_stmt.map(|s| Block(vec![Statement::If(s)])).or(block.clone()))
                    .or_not(),
            )
            .map(|((condition, true_branch), false_branch)| {
                IfStmt {
                    condition,
                    true_branch,
                    false_branch,
                }
            })
    }).map(Statement::If);

    let return_stmt = text::keyword("return")
        .ignore_then(
            ident
                .then_ignore(just(':').padded())
                .then(ident)
                .map(|(n, v): (&str, &str)| (n.to_string(), v.to_string()))
                .separated_by(just(',').padded())
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just('(').padded(), just(')').padded()),
        )
        .then_ignore(just(';').padded())
        .map(|values| Statement::Return(ReturnStmt { values }));

    let expr_stmt = expr.clone().then_ignore(just(';').padded()).map(Statement::Expr);

    // statement.define(choice((let_stmt, if_stmt, return_stmt, expr_stmt)).padded());
    statement.define(choice((let_stmt, if_stmt, return_stmt, expr_stmt)));

    let typed_param = ident
        .then_ignore(just(':').padded())
        .then(ident)
        .map(|(name, ty): (&str, &str)| TypedParam {
            name: name.to_string(),
            ty: ty.to_string(),
        });

    let param_list = typed_param
        .separated_by(just(',').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded());

    let func_def = text::keyword("def")
        .ignore_then(ident)
        .then(param_list.clone())
        .then(just("->").padded().ignore_then(param_list).or_not())
        .then(block)
        // .map(|(((name, params), return_params), body)| FuncDef {
        // with type params
        .map(|(((name, params), return_params), body): (((&str, Vec<TypedParam>), Option<Vec<TypedParam>>), Block)| FuncDef {
            name: name.to_string(),
            params,
            return_params: return_params.unwrap_or_default(),
            body,
        });

    let program = func_def
        .padded_by(comments()) // Allow comments between functions
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map(Program);

    program
}

// A parser for C-style comments (// and /* */) that consumes them
fn comments<'a>() -> impl Parser<'a, ParserInput<'a>, (), ParserError<'a>> + Clone {
    let single_line = just("//").then(any().and_is(just('\n').not()).repeated()).ignored();
    let multi_line = just("/*").then(any().and_is(just("*/").not()).repeated()).then(just("*/")).ignored();
    single_line.or(multi_line).padded().repeated().ignored()
}

//================================================================================
// Main (for testing)
//================================================================================

fn main() {
    let src = r#"
    // A function to remove the max value from a conceptual heap.
    def max_heap_remove(sentinel: Object) -> (max_value: Integer) {
        let! max_value = add_node(-1); // Initialize a return node

        if shape [
            root: Integer,
            sentinel -> root: Wildcard
        ] {
            // If a root exists, start the recursive removal process.
            max_heap_remove_helper(root, max_value);
        } else {
            // The heap is empty, do nothing.
        }

        return (max_value: max_value);
    }

    /* A helper function to perform the recursive heap logic.
       It handles several cases: two children, one child, or a leaf.
    */
    def max_heap_remove_helper(root: Integer, max_value: Integer) {
        copy_value_from_to(root, max_value);

        if shape [
            left: Integer,
            root -> left: Wildcard,
            right: Integer,
            root -> right: Wildcard
        ] {
            // Two children exist. Find the larger one and pull its value up.
            if cmp_fst_snd[>](left, right) {
                let! temp_max = add_node(-1);
                max_heap_remove_helper(left, temp_max);
                copy_value_from_to(temp_max, root);
                remove_node(temp_max);
            } else {
                let! temp_max = add_node(-1);
                max_heap_remove_helper(right, temp_max);
                copy_value_from_to(temp_max, root);
                remove_node(temp_max);
            }
        } else if shape [
            child: Integer,
            root -> child: Wildcard
        ] {
            // Only one child exists.
            let! temp_max = add_node(-1);
            max_heap_remove_helper(child, temp_max);
            copy_value_from_to(temp_max, root);
            remove_node(temp_max);
        } else {
            // This is a leaf node.
            remove_node(root);
        }
    }
    "#;

    let src = r#"
    def max_heap_remove(sentinel: Object) -> (max_value: Integer) {
        let! max_value = add_node(-1);

        if shape [
            root: Integer,
            sentinel -> root: Wildcard
        ] {
            max_heap_remove_helper(root, max_value);
        } else {
        }

        return (max_value: max_value);
    }

    def max_heap_remove_helper(root: Integer, max_value: Integer) {
        copy_value_from_to(root, max_value);

        if shape [
            left: Integer,
            root -> left: Wildcard,
            right: Integer,
            root -> right: Wildcard
        ] {
            if cmp_fst_snd[>](left, right) {
                let! temp_max = add_node(-1);
                max_heap_remove_helper(left, temp_max);
                copy_value_from_to(temp_max, root);
                remove_node(temp_max);
            } else {
                let! temp_max = add_node(-1);
                max_heap_remove_helper(right, temp_max);
                copy_value_from_to(temp_max, root);
                remove_node(temp_max);
            }
        } else if shape [
            child: Integer,
            root -> child: Wildcard
        ] {
            let! temp_max = add_node(-1);
            max_heap_remove_helper(child, temp_max);
            copy_value_from_to(temp_max, root);
            remove_node(temp_max);
        } else {
            remove_node(root);
        }
    }
    "#;

    // FIX 2: Call `.into_result()` to convert Chumsky's ParseResult into
    // a standard Rust Result, which can be used in a `match` statement.
    match parser().parse(src).into_result() {
        Ok(ast) => println!("{:#?}", ast),
        Err(errors) => errors
            .into_iter()
            // Make error printing a bit nicer
            .for_each(|e| println!("{}", e.map_token(|c| c.to_string()))),
    }
}