use chumsky::prelude::*;

//================================================================================
// Abstract Syntax Tree (AST)
//================================================================================

#[derive(Debug, Clone)]
pub struct Program(pub Vec<FuncDef>);
#[derive(Debug, Clone)]
pub struct FuncDef { pub name: String, pub params: Vec<TypedParam>, pub return_params: Vec<TypedParam>, pub body: Block }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParam { pub name: String, pub ty: String }
#[derive(Debug, Clone)]
pub struct Block(pub Vec<Statement>);
#[derive(Debug, Clone)]
pub enum Statement { Let(LetStmt), If(IfStmt), Expr(Expr), Return(ReturnStmt) }
#[derive(Debug, Clone)]
pub struct LetStmt { pub var_name: String, pub expr: Expr }
#[derive(Debug, Clone)]
pub struct IfStmt { pub condition: Condition, pub true_branch: Block, pub false_branch: Option<Block> }
#[derive(Debug, Clone)]
pub enum Condition { Shape(Vec<ShapePattern>), Expr(Expr) }
#[derive(Debug, Clone)]
pub enum ShapePattern { Node { name: String, ty: String }, Edge { src: String, dst: String, ty: String } }
#[derive(Debug, Clone)]
pub struct ReturnStmt { pub values: Vec<(String, String)> }
#[derive(Debug, Clone)]
pub enum Expr { Call { func: String, args: Vec<Expr> }, BuiltinCall { func: String, op: String, args: Vec<Expr> }, Literal(Literal), Ident(String) }
#[derive(Debug, Clone)]
pub enum Literal { Int(i64) }

//================================================================================
// Parser Definition
//================================================================================

type ParserInput<'a> = &'a str;
type ParserError<'a> = extra::Err<Simple<'a, char>>;

pub fn parser<'a>() -> impl Parser<'a, ParserInput<'a>, Program, ParserError<'a>> {
    // A robust padding parser that avoids non-consuming loops.
    let comment = {
        let single_line = just("//").then(any().and_is(just('\n').not()).repeated()).ignored();
        let multi_line = just("/*").then(any().and_is(just("*/").not()).repeated()).then(just("*/")).ignored();
        // The parser for a single unit of padding. It fails if no padding is present.
        single_line.or(multi_line).or(text::whitespace().ignored()).boxed()
    };
    // The parser for any amount of padding.
    let padding = comment.repeated().ignored();

    // A helper for creating a token parser that is padded on both sides.
    fn token<'a, P: Clone + Parser<'a, ParserInput<'a>, T, ParserError<'a>>, T>(p: P) -> impl Parser<'a, ParserInput<'a>, T, ParserError<'a>> + Clone {
        let comment = {
            let single_line = just("//").then(any().and_is(just('\n').not()).repeated()).ignored();
            let multi_line = just("/*").then(any().and_is(just("*/").not()).repeated()).then(just("*/")).ignored();
            single_line.or(multi_line).or(text::whitespace().ignored()).boxed()
        };
        // A parser for any amount of padding
        let padding = comment.repeated().ignored();
        p.padded_by(padding.clone())
    }

    let ident = token(text::ident());
    let int_literal = token(just('-')
        .or_not()
        .then(text::int(10))
        .to_slice()
        .from_str()
        .unwrapped());

    let mut expr = Recursive::declare();
    let mut statement = Recursive::declare();

    let args = expr
        .clone()
        .separated_by(token(just(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(token(just('(')), token(just(')')));

    let call = ident.clone()
        .then(ident.clone().delimited_by(token(just('[')), token(just(']'))).or_not())
        .then(args)
        .map(|((func, op), args): ((&str, Option<&str>), Vec<Expr>)| match op {
            Some(op) => Expr::BuiltinCall { func: func.to_string(), op: op.to_string(), args },
            None => Expr::Call { func: func.to_string(), args },
        });

    let atom = choice((
        int_literal.map(Literal::Int).map(Expr::Literal),
        call,
        ident.clone().map(|i: &str| Expr::Ident(i.to_string())),
    ));

    expr.define(atom);

    let block = statement
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(token(just('{')), token(just('}')))
        .map(Block);

    let let_stmt = token(text::keyword("let!"))
        .ignore_then(ident.clone())
        .then_ignore(token(just('=')))
        .then(expr.clone())
        .then_ignore(token(just(';')))
        .map(|(var_name, expr): (&str, Expr)| {
            Statement::Let(LetStmt { var_name: var_name.to_string(), expr })
        });

    let shape_pattern = {
        let node = token(text::keyword("node")).ignore_then(ident.clone())
            .then_ignore(token(just(':'))).then(ident.clone())
            .map(|(name, ty): (&str, &str)| ShapePattern::Node { name: name.to_string(), ty: ty.to_string() });
        let edge = token(text::keyword("edge")).ignore_then(ident.clone())
            .then_ignore(token(just("->"))).then(ident.clone())
            .then_ignore(token(just(':'))).then(ident.clone())
            .map(|((src, dst), ty): ((&str, &str), &str)| ShapePattern::Edge { src: src.to_string(), dst: dst.to_string(), ty: ty.to_string() });
        choice((node, edge))
    };

    let shape_query = token(text::keyword("shape"))
        .ignore_then(shape_pattern.separated_by(token(just(','))).allow_trailing().collect::<Vec<_>>()
            .delimited_by(token(just('[')), token(just(']'))))
        .map(Condition::Shape);

    let condition = choice((shape_query, expr.clone().map(Condition::Expr)));

    let if_stmt = recursive(|if_stmt| {
        token(text::keyword("if")).ignore_then(condition)
            .then(block.clone())
            .then(token(text::keyword("else")).ignore_then(
                if_stmt.map(|s| Block(vec![Statement::If(s)])).or(block.clone())
            ).or_not())
            .map(|((condition, true_branch), false_branch)| IfStmt { condition, true_branch, false_branch })
    }).map(Statement::If);

    let return_stmt = token(text::keyword("return")).ignore_then(
        ident.clone().then_ignore(token(just(':'))).then(ident.clone())
            .map(|(n, v): (&str, &str)| (n.to_string(), v.to_string()))
            .separated_by(token(just(','))).allow_trailing().collect::<Vec<_>>()
            .delimited_by(token(just('(')), token(just(')')))
    ).then_ignore(token(just(';')))
        .map(|values| Statement::Return(ReturnStmt { values }));

    let expr_stmt = expr.clone().then_ignore(token(just(';'))).map(Statement::Expr);

    statement.define(choice((let_stmt, if_stmt, return_stmt, expr_stmt)));

    let typed_param = ident.clone().then_ignore(token(just(':'))).then(ident.clone())
        .map(|(name, ty): (&str, &str)| TypedParam { name: name.to_string(), ty: ty.to_string() });

    let param_list = typed_param.separated_by(token(just(','))).allow_trailing().collect::<Vec<_>>()
        .delimited_by(token(just('(')), token(just(')')));

    let func_def = token(text::keyword("def")).ignore_then(ident.clone())
        .then(param_list.clone())
        .then(token(just("->")).ignore_then(param_list).or_not())
        .then(block)
        .map(|(((name, params), ret), body): (((&str, Vec<TypedParam>), Option<Vec<TypedParam>>), Block)| FuncDef {
            name: name.to_string(),
            params,
            return_params: ret.unwrap_or_default(),
            body,
        });

    let program = func_def.repeated().at_least(1).collect::<Vec<_>>()
        .then_ignore(padding.clone()) // Handles trailing comments/whitespace
        .then_ignore(end())
        .map(Program);

    // The top-level parser is padded to handle leading comments/whitespace.
    padding.ignore_then(program)
}

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

    match parser().parse(src).into_result() {
        Ok(ast) => println!("{:#?}", ast),
        Err(errors) => errors.into_iter().for_each(|e| println!("{}", e.map_token(|c| c.to_string()))),
    }
}