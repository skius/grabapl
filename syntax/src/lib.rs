pub mod interpreter;
pub mod minirust;
pub mod semantics;

use chumsky::error::LabelError;
use chumsky::extra::ParserExtra;
use chumsky::input::{MapExtra, SliceInput, StrInput};
use chumsky::text::{Char, TextExpected};
use chumsky::util::MaybeRef;
use chumsky::{input::ValueInput, prelude::*};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

pub trait CustomSyntax: Clone + Debug + 'static {
    type MacroArgType: Clone + fmt::Debug + Default + PartialEq;

    type AbstractNodeType: Clone + Debug + PartialEq;
    type AbstractEdgeType: Clone + Debug + PartialEq;

    fn get_macro_arg_parser<'src>()
    -> impl Parser<'src, &'src str, Self::MacroArgType, extra::Err<Rich<'src, char, Span>>> + Clone;

    fn get_node_type_parser<
        'src: 'tokens,
        'tokens,
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    >()
    -> impl Parser<'tokens, I, Self::AbstractNodeType, extra::Err<Rich<'tokens, Token<'src>, Span>>>
    + Clone;
    fn get_edge_type_parser<
        'src: 'tokens,
        'tokens,
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    >()
    -> impl Parser<'tokens, I, Self::AbstractEdgeType, extra::Err<Rich<'tokens, Token<'src>, Span>>>
    + Clone;
}

#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomSyntax;

// TODO: borrow?
#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomStructField {
    pub name: String,
    pub typ: MyCustomType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomStruct {
    pub name: String,
    pub fields: Vec<MyCustomStructField>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MyCustomType {
    Primitive(String),
    Custom(MyCustomStruct),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeType {
    Exact(String),
    Wildcard,
}

impl CustomSyntax for MyCustomSyntax {
    type MacroArgType = Vec<String>;
    type AbstractNodeType = MyCustomType;
    type AbstractEdgeType = EdgeType;

    fn get_macro_arg_parser<'src>()
    -> impl Parser<'src, &'src str, Self::MacroArgType, extra::Err<Rich<'src, char, Span>>> + Clone
    {
        // a comma separated list of strings

        any::<&'src str, extra::Err<Rich<'src, char, Span>>>()
            .filter(|c| *c != ']' && *c != ',')
            .repeated()
            .to_slice()
            .separated_by(just(','))
            .collect()
            .map(|args: Vec<&'src str>| args.into_iter().map(String::from).collect())
            .padded()
    }

    fn get_node_type_parser<
        'src: 'tokens,
        'tokens,
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    >()
    -> impl Parser<'tokens, I, Self::AbstractNodeType, extra::Err<Rich<'tokens, Token<'src>, Span>>>
    + Clone {
        recursive(|my_typ| {
            let field_name = select! {
                Token::Ident(name) => name,
            }
            .labelled("field name");

            let primitive_type = select! {
                Token::Ident(primitive_name) => primitive_name,
            }
            .map(|name: &str| MyCustomType::Primitive(name.to_string()));

            let field_type = my_typ.labelled("field type");

            let field = field_name
                .then_ignore(just(Token::Ctrl(':')))
                .then(field_type)
                .map(|(name, typ)| MyCustomStructField {
                    name: name.to_string(),
                    typ,
                });
            let fields = field
                .separated_by(just(Token::Ctrl(',')))
                .allow_trailing()
                .collect::<Vec<_>>()
                .labelled("fields");

            let struct_name = select! {
                Token::Ident(name) => name,
            }
            .labelled("struct name");

            let entire_struct = struct_name
                .then_ignore(just(Token::Ctrl('{')))
                .then(fields)
                .then_ignore(just(Token::Ctrl('}')))
                .map(|(name, fields)| {
                    MyCustomType::Custom(MyCustomStruct {
                        name: name.to_string(),
                        fields,
                    })
                });

            entire_struct.or(primitive_type)
        })
    }

    fn get_edge_type_parser<
        'src: 'tokens,
        'tokens,
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    >()
    -> impl Parser<'tokens, I, Self::AbstractEdgeType, extra::Err<Rich<'tokens, Token<'src>, Span>>>
    + Clone {
        // * is Wildcard
        // "string" is Exact
        let wildcard = just(Token::Ctrl('*'))
            .to(EdgeType::Wildcard)
            .labelled("wildcard edge type");

        // TODO: oh. for string we need actual lexer support.
        let ident = select! {
            Token::Ident(ident) => ident,
        };

        let exact = just(Token::Ctrl('"'))
            .ignore_then(ident)
            .then_ignore(just(Token::Ctrl('"')))
            .map(|s: &'src str| EdgeType::Exact(s.to_string()))
            .labelled("exact edge type");

        wildcard.or(exact)
    }
}

// A few type definitions to be used by our parsers below
pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'src> {
    // do we want these?
    Bool(bool),
    Num(i32),
    // Str(&'src str),
    // Op(&'src str),
    Arrow,
    RevArrow,
    Ctrl(char),
    Ident(&'src str),
    Fn,
    Return,
    Let,
    LetBang,
    If,
    Else,
    Shape,
    MacroArgs(&'src str),
    // NodeType(&'src str),
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Bool(b) => write!(f, "{}", b),
            Token::Num(n) => write!(f, "{}", n),
            // Token::Str(s) => write!(f, "\"{}\"", s),
            // Token::Op(op) => write!(f, "{}", op),
            Token::Arrow => write!(f, "->"),
            Token::RevArrow => write!(f, "<-"),
            Token::Ctrl(c) => write!(f, "{}", c),
            Token::Ident(i) => write!(f, "{}", i),
            Token::Fn => write!(f, "fn"),
            Token::Return => write!(f, "return"),
            Token::Let => write!(f, "let"),
            Token::LetBang => write!(f, "let!"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Shape => write!(f, "shape"),
            Token::MacroArgs(s) => write!(f, "`{}`", s),
            // Token::NodeType(s) => write!(f, "{}", s),
        }
    }
}

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    // A parser for numbers
    let num = text::int(10)
        .to_slice()
        .from_str()
        .unwrapped()
        .map(Token::Num);

    // A parser for control characters (delimiters, semicolons, etc.)
    let ctrl = one_of("-()[]{};,?:*=/<>\"'.").map(Token::Ctrl);

    let arrow = just("->").to(Token::Arrow);
    let rev_arrow = just("<-").to(Token::RevArrow);

    let let_bang = just("let!").to(Token::LetBang);

    // A parser for identifiers and keywords
    let ident = text::ascii::ident().map(|ident: &str| match ident {
        "fn" => Token::Fn,
        "let" => Token::Let,
        "if" => Token::If,
        "return" => Token::Return,
        "else" => Token::Else,
        "shape" => Token::Shape,
        "true" => Token::Bool(true),
        "false" => Token::Bool(false),
        _ => Token::Ident(ident),
    });

    // node type parser.
    // node type returns a src that can then be parsed by a client provided parser.
    // it is the entire string slice until the next ',', but it skips over matched parentheses.
    // eg, it should parse "{ x , b } , bla"'s prefix "{ x , b }".
    // or "(,,,())" is fine as well.

    // unfortunately, below clashes with 'ident' parser.
    // let inner = recursive(|inner| {
    //     let mut matched_square = inner.clone()
    //         .repeated()
    //         .delimited_by(just('['), just(']'))
    //         .to_slice()
    //         ;
    //     let mut matched_paren = inner.clone()
    //         .repeated()
    //         .delimited_by(just('('), just(')'))
    //         .to_slice()
    //         ;
    //     let mut matched_brace = inner.clone()
    //         .repeated()
    //         .delimited_by(just('{'), just('}'))
    //         .to_slice()
    //         ;
    //     let mut matched_angle = inner.clone()
    //         .repeated()
    //         .delimited_by(just('<'), just('>'))
    //         .to_slice()
    //         ;
    //
    //     let inner_content = none_of("()[]{}<>").repeated().at_least(1).to_slice()
    //         .or(matched_square)
    //         .or(matched_paren)
    //         .or(matched_brace)
    //         .or(matched_angle);
    //     inner_content
    //
    //     // just('[').repeated().delimited_by(just('['), just(']')).or(just('b').repeated())
    // });
    //
    // // let mut inner = Recursive::declare();
    //
    //
    // // inner.define(inner_content.to_slice());
    //
    // let node_type_src = inner.repeated().at_least(1).to_slice().map(Token::NodeType);

    // let node_type_src = just('`').ignore_then(none_of("`")
    //     .repeated()
    //     .to_slice())
    //     .then_ignore(just('`'))
    //     .map(Token::NodeType);

    let macro_args = none_of("`")
        .repeated()
        .to_slice()
        .delimited_by(just('`'), just('`'))
        .map(Token::MacroArgs);

    // A single token can be one of the above
    // (macro_args needs to be before ctrl, since ctrl has the same prefix)
    let token = let_bang
        .or(num)
        .or(macro_args)
        .or(arrow)
        .or(rev_arrow)
        .or(ctrl)
        .or(ident)
        .boxed(); //.or(node_type_src);

    let comment = just("//")
        .then(any().and_is(just('\n').not()).repeated())
        .padded();

    token
        .map_with(|tok, e| (tok, e.span()))
        .padded_by(comment.repeated())
        .padded()
        // If we encounter an error, skip and attempt to lex the next character as a token instead
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}

// TODO: we cannot really have this. Since both lib and custom might be able to parse the entire macro syntax,
//  this distinction needs to happen afterwards.
//  I suppose we could delay lib parsing and just store the tokens that are parsed into CS::MacroArgType for later consumption?
#[derive(Clone, Debug, PartialEq)]
pub struct MacroArgs<'src> (&'src str);

#[derive(Clone, Debug, PartialEq)]
pub struct FnCallExpr<'src> {
    pub name: Spanned<&'src str>,
    pub macro_args: Option<Spanned<MacroArgs<'src>>>,
    pub args: Vec<Spanned<NodeId<'src>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LetStmt<'src> {
    pub bang: bool,
    pub ident: Spanned<&'src str>,
    pub call: Spanned<FnCallExpr<'src>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeNodeParam<'src, CS: CustomSyntax> {
    pub name: Spanned<NodeId<'src>>,
    pub node_type: Spanned<CS::AbstractNodeType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeEdgeParam<'src, CS: CustomSyntax> {
    pub src: Spanned<NodeId<'src>>,
    pub dst: Spanned<NodeId<'src>>,
    pub edge_type: Spanned<CS::AbstractEdgeType>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeQueryParam<'src, CS: CustomSyntax> {
    Node(ShapeNodeParam<'src, CS>),
    Edge(ShapeEdgeParam<'src, CS>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeQueryParams<'src, CS: CustomSyntax> {
    pub params: Vec<Spanned<ShapeQueryParam<'src, CS>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IfCond<'src, CS: CustomSyntax> {
    // TODO: Spanned<> should be moved out and into IfStmt::cond
    Query(FnCallExpr<'src>),
    Shape(ShapeQueryParams<'src, CS>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfStmt<'src, CS: CustomSyntax> {
    pub cond: Spanned<IfCond<'src, CS>>,
    pub then_block: Spanned<Block<'src, CS>>,
    // if it doesn't exist, take empty span at the end of then_block
    pub else_block: Spanned<Block<'src, CS>>,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum NodeId<'src> {
    Single(&'src str),
    Output(Spanned<&'src str>, Spanned<&'src str>),
}

impl<'src> NodeId<'src> {
    pub fn must_single(&self) -> &'src str {
        match self {
            NodeId::Single(name) => name,
            _ => {
                panic!("NodeId must be a single node, but was: {:?}", self);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReturnStmtMapping<'src, CS: CustomSyntax> {
    Node {
        /// The name of the output marker
        ret_name: Spanned<&'src str>,
        /// The node to return
        node: Spanned<NodeId<'src>>,
    },
    Edge {
        src: Spanned<NodeId<'src>>,
        dst: Spanned<NodeId<'src>>,
        edge_type: Spanned<CS::AbstractEdgeType>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReturnStmt<'src, CS: CustomSyntax> {
    pub mapping: Vec<Spanned<ReturnStmtMapping<'src, CS>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenameStmt<'src> {
    pub new_name: Spanned<&'src str>,
    pub src: Spanned<NodeId<'src>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement<'src, CS: CustomSyntax> {
    Let(Spanned<LetStmt<'src>>),
    FnCall(Spanned<FnCallExpr<'src>>),
    If(Spanned<IfStmt<'src, CS>>),
    Return(Spanned<ReturnStmt<'src, CS>>),
    Rename(Spanned<RenameStmt<'src>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block<'src, CS: CustomSyntax> {
    pub statements: Vec<Spanned<Statement<'src, CS>>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FnNodeParam<'src, CS: CustomSyntax> {
    pub name: Spanned<&'src str>,
    pub node_type: Spanned<CS::AbstractNodeType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnEdgeParam<'src, CS: CustomSyntax> {
    pub src: Spanned<&'src str>,
    pub dst: Spanned<&'src str>,
    pub edge_type: Spanned<CS::AbstractEdgeType>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FnImplicitParam<'src, CS: CustomSyntax> {
    Node(FnNodeParam<'src, CS>),
    Edge(FnEdgeParam<'src, CS>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnDef<'src, CS: CustomSyntax> {
    pub name: Spanned<&'src str>,
    pub explicit_params: Vec<Spanned<FnNodeParam<'src, CS>>>,
    pub implicit_params: Vec<Spanned<FnImplicitParam<'src, CS>>>,
    pub return_signature: Vec<Spanned<FnImplicitParam<'src, CS>>>,
    pub body: Spanned<Block<'src, CS>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program<'src, CS: CustomSyntax> {
    // vec to preserve order. functions must be ordered according to their dependency order. no mutual recursion supported right now.
    // wrapper functions first, then their dependencies.
    pub functions: Vec<(&'src str, Spanned<FnDef<'src, CS>>)>,
}

pub fn program_parser<'tokens, 'src: 'tokens, I, CS: CustomSyntax>()
-> impl Parser<'tokens, I, Spanned<Program<'src, CS>>, extra::Err<Rich<'tokens, Token<'src>, Span>>>
+ Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>
        + SliceInput<
            'tokens,
            Token = Token<'src>,
            Span = Span,
            Slice = &'tokens [Spanned<Token<'src>>],
        >,
{
    let ident_str = select! {
        Token::Ident(ident) => ident,
    }
    .map_with(|ident, e| (ident, e.span()));

    let spanned_output_node_id = ident_str
        .then_ignore(just(Token::Ctrl('.')))
        .then(ident_str)
        .map_with(|(spanned_op, spanned_node), e| {
            (NodeId::Output(spanned_op, spanned_node), e.span())
        })
        .boxed();

    let spanned_node_id = spanned_output_node_id
        .or(ident_str.map(|(name, span)| (NodeId::Single(name), span)))
        .boxed();

    let spanned_fn_implicit_edge_param = ident_str
        .then_ignore(just(Token::Arrow))
        .then(ident_str)
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_edge_type_parser().map_with(|s, e| (s, e.span())))
        .map_with(
            |(((src, src_span), (dst, dst_span)), (edge_type, edge_type_span)), overall_span| {
                (
                    FnImplicitParam::Edge(FnEdgeParam {
                        src: (src, src_span),
                        dst: (dst, dst_span),
                        edge_type: (edge_type, edge_type_span),
                    }),
                    overall_span.span(),
                )
            },
        )
        .boxed();

    let spanned_fn_explicit_param = ident_str
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_node_type_parser().map_with(|s, e| (s, e.span())))
        .map_with(
            |((name, n_span), (node_type, node_type_span)), overall_span| {
                (
                    FnNodeParam {
                        name: (name, n_span),
                        node_type: (node_type, node_type_span),
                    },
                    overall_span.span(),
                )
            },
        )
        .boxed();

    let spanned_fn_implicit_param = spanned_fn_explicit_param
        .clone()
        .map(|(explicit_param, span)| (FnImplicitParam::Node(explicit_param), span))
        .or(spanned_fn_implicit_edge_param)
        .boxed();

    let fn_implicit_params = spanned_fn_implicit_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>();

    let shape_node_param = spanned_node_id.clone()
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_node_type_parser().map_with(|s, e| (s, e.span())))
        .map(|((name, name_span), (node_type, node_type_span))| {
            (
                ShapeNodeParam {
                    name: (name, name_span),
                    node_type: (node_type, node_type_span),
                }
            )
        })
        .boxed();

    let shape_edge_param = spanned_node_id.clone()
        .then_ignore(just(Token::Arrow))
        .then(spanned_node_id.clone())
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_edge_type_parser().map_with(|s, e| (s, e.span())))
        .map(
            |(((src, src_span), (dst, dst_span)), (edge_type, edge_type_span))| {
                (
                    ShapeEdgeParam {
                        src: (src, src_span),
                        dst: (dst, dst_span),
                        edge_type: (edge_type, edge_type_span),
                    }
                )
            },
        )
        .boxed();

    let spanned_shape_param = shape_node_param.map(|(node_param)| {
        ShapeQueryParam::Node(node_param)
    }).or(
        shape_edge_param.map(|(edge_param)| {
            ShapeQueryParam::Edge(edge_param)
        }),
    )
        .map_with(|s, e| (s, e.span()))
        .boxed();

    let shape_params = spanned_shape_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .map(|params| (ShapeQueryParams { params } ))
        .boxed();

    let block = recursive(|block| {
        let macro_args_str = select! {
            Token::MacroArgs(arg) => arg,
        };

        let spanned_macro_args = macro_args_str
            .map_with(|src, e| (MacroArgs(src), e.span()))
            .labelled("macro args")
            .boxed();

        let fn_call_expr = ident_str
            .then(
                spanned_macro_args
                    .or_not(),
            )
            .then_ignore(just(Token::Ctrl('(')))
            .then(
                spanned_node_id
                    .clone()
                    .separated_by(just(Token::Ctrl(',')))
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then_ignore(just(Token::Ctrl(')')))
            .map_with(|((spanned_name, spanned_macro_args), args), e| {
                (
                    FnCallExpr {
                        name: spanned_name,
                        macro_args: spanned_macro_args,
                        args,
                    },
                    e.span(),
                )
            })
            .boxed();



        let if_cond_shape = just(Token::Shape)
            .ignore_then(just(Token::Ctrl('[')))
            .ignore_then(shape_params.clone())
            .then_ignore(just(Token::Ctrl(']')))
            .map(IfCond::Shape);

        let if_cond_query = fn_call_expr
            .clone()
            .map(|spanned_call| IfCond::Query(spanned_call.0));

        let spanned_if_cond = if_cond_shape.or(if_cond_query).map_with(|c, e| (c, e.span()))
            .labelled("if condition")
            .boxed();

        let if_stmt = recursive(|if_stmt| {
            let spanned_block_wrapped_stmts = block
                .clone()
                .delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}')))
                .map_with(|block, e| (block, e.span()))
                .labelled("block");

            let if_stmt_as_block = if_stmt.clone().map_with(|if_stmt, e| Block {
                statements: vec![(if_stmt, e.span())],
            });

            let spanned_block_if_or_wrapped_stmts = spanned_block_wrapped_stmts
                .clone()
                .or(if_stmt_as_block.map_with(|if_stmt, e| (if_stmt, e.span())))
                .boxed();

            let optional_else_part = just(Token::Else)
                .ignore_then(spanned_block_if_or_wrapped_stmts)
                .or_not()
                .map_with(|opt, e| opt.unwrap_or((Block { statements: vec![] }, e.span())))
                .labelled("else")
                .boxed();

            let if_stmt = just(Token::If)
                .ignore_then(spanned_if_cond)
                .then_ignore(just(Token::Ctrl('{')))
                .then(block.clone().map_with(|block, e| (block, e.span())))
                .then_ignore(just(Token::Ctrl('}')))
                .then(optional_else_part)
                .map_with(|((cond, spanned_then_block), spanned_else_block), e| {
                    (
                        IfStmt {
                            cond,
                            then_block: spanned_then_block,
                            else_block: spanned_else_block,
                        },
                        e.span(),
                    )
                })
                .map(Statement::If)
                .labelled("if statement")
                .boxed();

            // TODO: continue with entire if else statements.
            if_stmt
        });

        let spanned_if_stmt = if_stmt
            .map_with(|if_stmt, e| (if_stmt, e.span()))
            .labelled("if statement");

        let fn_call_stmt = fn_call_expr
            .clone()
            .then_ignore(just(Token::Ctrl(';')))
            .map_with(|spanned_call, e| (Statement::FnCall(spanned_call), e.span()))
            .labelled("function call statement");

        let let_or_let_bang = select! {
            Token::Let => false,
            Token::LetBang => true,
        };

        let let_stmt = let_or_let_bang
            .then(ident_str)
            .then_ignore(just(Token::Ctrl('=')))
            .then(fn_call_expr)
            .then_ignore(just(Token::Ctrl(';')))
            .map_with(|((bang, (name, name_span)), (call, call_span)), e| {
                (
                    LetStmt {
                        bang,
                        ident: (name, name_span),
                        call: (call, call_span),
                    },
                    e.span(),
                )
            })
            .map(Statement::Let)
            .map_with(|let_stmt, e| (let_stmt, e.span()))
            .labelled("let statement");

        let spanned_return_node_mapping = ident_str
            .then_ignore(just(Token::Ctrl(':')))
            .then(spanned_node_id.clone())
            .map_with(|(ret_name, node_id), e| {
                (
                    ReturnStmtMapping::Node {
                        ret_name,
                        node: node_id,
                    },
                    e.span(),
                )
            });

        let spanned_return_edge_mapping = spanned_node_id
            .clone()
            .then_ignore(just(Token::Arrow))
            .then(spanned_node_id.clone())
            .then_ignore(just(Token::Ctrl(':')))
            .then(CS::get_edge_type_parser().map_with(|s, e| (s, e.span())))
            .map_with(|((src, dst), edge_typ), e| {
                (
                    ReturnStmtMapping::Edge {
                        src,
                        dst,
                        edge_type: edge_typ,
                    },
                    e.span(),
                )
            });

        let spanned_return_mappings = spanned_return_node_mapping
            .or(spanned_return_edge_mapping)
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .boxed();

        let spanned_return_stmt = just(Token::Return)
            .ignore_then(just(Token::Ctrl('(')))
            .ignore_then(spanned_return_mappings)
            .then_ignore(just(Token::Ctrl(')')))
            .then_ignore(just(Token::Ctrl(';')))
            .map_with(|mappings, e| (ReturnStmt { mapping: mappings }, e.span()))
            .map(Statement::Return)
            .map_with(|return_stmt, e| (return_stmt, e.span()))
            .labelled("return statement");

        let spanned_rename_stmt = ident_str.then_ignore(just(Token::RevArrow))
            .then(spanned_node_id.clone())
            .then_ignore(just(Token::Ctrl(';')))
            .map_with(|((new_name, new_name_span), (src, src_span)), e| {
                (
                    RenameStmt {
                        new_name: (new_name, new_name_span),
                        src: (src, src_span),
                    },
                    e.span(),
                )
            })
            .map(Statement::Rename)
            .map_with(|rename_stmt, e| (rename_stmt, e.span()))
            .labelled("rename statement");

        let spanned_stmt = let_stmt
            .or(fn_call_stmt)
            .or(spanned_if_stmt)
            .or(spanned_return_stmt)
            .or(spanned_rename_stmt)
            .labelled("statement")
            .boxed();

        let block_many_stmts = spanned_stmt
            .repeated()
            .collect()
            .map(|stmts| Block { statements: stmts })
            .labelled("block")
            .boxed();

        block_many_stmts
    });

    let fn_return_signature = fn_implicit_params.clone();

    let optional_fn_return_signature = (just(Token::Arrow)
        .ignore_then(just(Token::Ctrl('(')))
        .ignore_then(fn_return_signature)
        .then_ignore(just(Token::Ctrl(')'))))
    .or_not()
    .map(|opt| opt.unwrap_or_default())
    .boxed();

    let fn_explicit_params = spanned_fn_explicit_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>();

    let optional_fn_implicit_param = (just(Token::Ctrl('['))
        .ignore_then(fn_implicit_params)
        .then_ignore(just(Token::Ctrl(']'))))
    .or_not()
    .map(|opt| opt.unwrap_or_default())
    .boxed();

    let fn_def = just(Token::Fn)
        .ignore_then(ident_str)
        .then_ignore(just(Token::Ctrl('(')))
        .then(fn_explicit_params)
        .then_ignore(just(Token::Ctrl(')')))
        .then(optional_fn_implicit_param)
        .then(optional_fn_return_signature)
        .then_ignore(just(Token::Ctrl('{')))
        .then(block.map_with(|block, e| (block, e.span())))
        .then_ignore(just(Token::Ctrl('}')))
        .map(
            |(
                (((spanned_name, explicit_params), implicit_params), return_signature),
                spanned_body,
            )| FnDef {
                name: spanned_name,
                explicit_params,
                implicit_params,
                return_signature,
                body: spanned_body,
            },
        );

    let program = fn_def
        .map_with(|fn_def, e| (fn_def, e.span()))
        .repeated()
        .collect::<Vec<_>>()
        .map_with(|functions_with_span, e| {
            let mut funcs_list = Vec::new();
            for (func, func_span) in functions_with_span {
                funcs_list.push((func.name.0, (func, func_span)));
            }
            (
                Program {
                    functions: funcs_list,
                },
                e.span(),
            )
        })
        .labelled("program");

    program
}
