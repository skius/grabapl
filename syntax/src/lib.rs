pub mod interpreter;
pub mod custom_syntax;

use crate::interpreter::{interpret, InterpreterResult};
use ariadne::{sources, Color, Label, Report, ReportKind};
use chumsky::input::SliceInput;
use chumsky::{input::ValueInput, prelude::*};
use grabapl::operation::marker::SkipMarkers;
use grabapl::prelude::{OperationContext, OperationId};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::Debug;
use std::io::BufWriter;
use std::ops::Range;
use custom_syntax::{CustomSyntax, SemanticsWithCustomSyntax};
use grabapl::operation::builder::IntermediateState;

// A few type definitions to be used by our parsers below
pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'src> {
    // do we want these?
    Bool(bool),
    Num(i32),
    Str(&'src str),
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
            Token::Str(s) => write!(f, "\"{}\"", s),
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

    // A parser for strings
    let str_ = just('"')
        .ignore_then(none_of('"').repeated().to_slice())
        .then_ignore(just('"'))
        .map(Token::Str);

    let macro_args = none_of("`")
        .repeated()
        .to_slice()
        .delimited_by(just('`'), just('`'))
        .map(Token::MacroArgs);

    let macro_args_opt2 = none_of("%")
        .repeated()
        .to_slice()
        .delimited_by(just('%'), just('%'))
        .map(Token::MacroArgs);

    let macro_args_opt3 = none_of("<>")
        .repeated()
        .to_slice()
        .delimited_by(just('<'), just('>'))
        .map(Token::MacroArgs);

    // A single token can be one of the above
    // (macro_args needs to be before ctrl, since ctrl has the same prefix)
    let token = let_bang
        .or(num)
        .or(arrow)
        .or(rev_arrow)
        .or(macro_args)
        .or(macro_args_opt2)
        .or(macro_args_opt3)
        .or(str_)
        .or(ctrl)
        .or(ident)
        .boxed(); //.or(node_type_src);

    let comment = just("//")
        .then(any().and_is(just('\n').not()).repeated())
        .padded();

    let block_comment = just("/*")
        .then(any().and_is(just("*/").not()).repeated())
        .then_ignore(just("*/"))
        .padded();

    token
        .map_with(|tok, e| (tok, e.span()))
        .padded_by(comment.or(block_comment).repeated())
        .padded()
        // If we encounter an error, skip and attempt to lex the next character as a token instead
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}

// TODO: we cannot really have this. Since both lib and custom might be able to parse the entire macro syntax,
//  this distinction needs to happen afterwards.
//  I suppose we could delay lib parsing and just store the tokens that are parsed into CS::MacroArgType for later consumption?
#[derive(Clone, Debug, PartialEq, Copy)]
pub struct MacroArgs<'src>(pub &'src str);

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
    // TODO: spanned<>
    pub skip_markers: SkipMarkers,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IfCond<'src, CS: CustomSyntax> {
    Query(Spanned<FnCallExpr<'src>>),
    Shape(Spanned<ShapeQueryParams<'src, CS>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfStmt<'src, CS: CustomSyntax> {
    pub cond: Spanned<IfCond<'src, CS>>,
    pub then_block: Spanned<Block<'src, CS>>,
    // if it doesn't exist, take empty span at the end of then_block
    pub else_block: Spanned<Block<'src, CS>>,
}

#[derive(Clone, derive_more::Debug, PartialEq, Copy)]
pub enum NodeId<'src> {
    #[debug("{_0}")]
    Single(&'src str),
    #[debug("{0}.{1}", _0.0, _1.0)]
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

    pub fn single(&self) -> Option<&'src str> {
        match self {
            NodeId::Single(name) => Some(name),
            _ => None,
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
    .map_with(|ident, e| (ident, e.span()))
        .labelled("identifier");

    let spanned_output_node_id = ident_str
        .then_ignore(just(Token::Ctrl('.')))
        .then(ident_str)
        .map_with(|(spanned_op, spanned_node), e| {
            (NodeId::Output(spanned_op, spanned_node), e.span())
        })
        .boxed();

    let spanned_node_id = spanned_output_node_id
        .or(ident_str.map(|(name, span)| (NodeId::Single(name), span)))
        .boxed()
        .labelled("node identifier");

    let spanned_fn_implicit_edge_param = ident_str
        .then_ignore(just(Token::Arrow))
        .then(ident_str)
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_edge_type_parser().labelled("edge type").map_with(|s, e| (s, e.span())))
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
        .boxed()
        .labelled("implicit edge function parameter");

    let spanned_fn_explicit_param = ident_str
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_node_type_parser().labelled("node type").map_with(|s, e| (s, e.span())))
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
        .boxed()
        .labelled("explicit function parameter");

    let spanned_fn_implicit_param = spanned_fn_explicit_param
        .clone()
        .map(|(explicit_param, span)| (FnImplicitParam::Node(explicit_param), span))
        .or(spanned_fn_implicit_edge_param)
        .boxed()
        .labelled("implicit function parameter");

    let fn_implicit_params = spanned_fn_implicit_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>();

    let shape_node_param = spanned_node_id
        .clone()
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_node_type_parser().labelled("node type").map_with(|s, e| (s, e.span())))
        .map(
            |((name, name_span), (node_type, node_type_span))| ShapeNodeParam {
                name: (name, name_span),
                node_type: (node_type, node_type_span),
            },
        )
        .boxed()
        .labelled("shape query node parameter");

    let shape_edge_param = spanned_node_id
        .clone()
        .then_ignore(just(Token::Arrow))
        .then(spanned_node_id.clone())
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_edge_type_parser().labelled("edge type").map_with(|s, e| (s, e.span())))
        .map(
            |(((src, src_span), (dst, dst_span)), (edge_type, edge_type_span))| ShapeEdgeParam {
                src: (src, src_span),
                dst: (dst, dst_span),
                edge_type: (edge_type, edge_type_span),
            },
        )
        .boxed()
        .labelled("shape query edge parameter");

    let spanned_shape_param = shape_node_param
        .map(|node_param| ShapeQueryParam::Node(node_param))
        .or(shape_edge_param.map(|edge_param| ShapeQueryParam::Edge(edge_param)))
        .map_with(|s, e| (s, e.span()))
        .boxed();

    let shape_params = spanned_shape_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .map(|params| ShapeQueryParams {
            params,
            skip_markers: SkipMarkers::default(),
        })
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
            .then(spanned_macro_args.or_not())
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

        // TODO: add support for `skipping all` in addition to the current `skipping [...]` syntax.
        let optional_skipping_markers = select! {
            Token::Ident("skipping") => (),
        }.labelled("'skipping'")
            .ignore_then(just(Token::Ctrl('[')))
            .ignore_then(
                select! { Token::Str(s) => s }.labelled("marker literal (e.g., '\"visited\"')")
                    .separated_by(just(Token::Ctrl(',')))
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then_ignore(just(Token::Ctrl(']')))
            .or_not()
            .map(|opt| {
                opt.map(|marker_names| SkipMarkers::new(marker_names))
                    .unwrap_or(SkipMarkers::default())
            })
            .boxed()
            .labelled("skipping markers");

        let if_cond_shape = just(Token::Shape)
            .ignore_then(just(Token::Ctrl('[')))
            .ignore_then(shape_params.clone())
            .then_ignore(just(Token::Ctrl(']')))
            .then(optional_skipping_markers)
            .map(|(mut params, skip_markers)| {
                params.skip_markers = skip_markers;
                params
            })
            .map_with(|params, e| (params, e.span()))
            .map(IfCond::Shape);

        let if_cond_query = fn_call_expr
            .clone()
            .map(|spanned_call| IfCond::Query(spanned_call));

        let spanned_if_cond = if_cond_shape
            .or(if_cond_query)
            .map_with(|c, e| (c, e.span()))
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
            .then(CS::get_edge_type_parser().labelled("edge type").map_with(|s, e| (s, e.span())))
            .map_with(|((src, dst), edge_typ), e| {
                (
                    ReturnStmtMapping::Edge {
                        src,
                        dst,
                        edge_type: edge_typ,
                    },
                    e.span(),
                )
            })
            .boxed()
            .labelled("return edge mapping");

        let spanned_return_mappings = spanned_return_node_mapping
            .or(spanned_return_edge_mapping)
            .separated_by(just(Token::Ctrl(',')))
            .allow_trailing()
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

        let spanned_rename_stmt = ident_str
            .then_ignore(just(Token::RevArrow))
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
    .boxed()
    .labelled("function return signature");

    let fn_explicit_params = spanned_fn_explicit_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>();

    let optional_fn_implicit_param = (just(Token::Ctrl('['))
        .ignore_then(fn_implicit_params)
        .then_ignore(just(Token::Ctrl(']'))))
    .or_not()
    .map(|opt| opt.unwrap_or_default())
    .boxed()
    .labelled("implicit function parameters");

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
        )
        .boxed()
        .labelled("function definition");

    let program = fn_def
        .map_with(|fn_def, e| (fn_def, e.span()))
        .repeated()
        .collect::<Vec<_>>()
        .validate(|functions_with_span, e, emitter| {
            let mut funcs_list = Vec::new();
            let mut seen_names = HashSet::new();
            for (func, func_span) in functions_with_span {
                if seen_names.contains(func.name.0) {
                    emitter.emit(
                        Rich::custom(
                            func.name.1,
                            format!("Function `{}` is defined multiple times", func.name.0),
                        )
                    );
                    continue;
                }
                seen_names.insert(func.name.0);
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

/// Important syntax note: mutually recursive functions are not supported.
/// Function definitions must be ordered in reverse C/C++ order, i.e.,
/// if function `foo` calls `bar`, then `bar` must be defined after `foo` in the source.
// TODO: rework this function. terrible.
pub fn parse_to_op_ctx_and_map<'src, S: SemanticsWithCustomSyntax>(
    src: &'src str,
) -> (OperationContext<S>, HashMap<&'src str, OperationId>) {
    match try_parse_to_op_ctx_and_map::<S>(src, true).op_ctx_and_map {
        Ok((op_ctx, fn_map)) => (op_ctx, fn_map),
        Err(WithLineColSpans { value: output, .. }) => {
            panic!("Failed to parse the input source code:\n{output}")
        }
    }
}

pub fn try_parse_to_op_ctx_and_map<'src, S: SemanticsWithCustomSyntax>(
    src: &'src str,
    color_enabled: bool,
) -> InterpreterResult<'src, S, WithLineColSpans<String>> {
    let filename = "input".to_string();
    let (tokens, errs) = lexer().parse(src).into_output_errors();

    fn string_of_report(filename: String, src: &str, report: Report<(String, Range<usize>)>) -> String {
        let mut output_buf = BufWriter::new(Vec::new());
        report
            .write(sources([(filename, src)]), &mut output_buf)
            .unwrap();
        String::from_utf8(output_buf.into_inner().unwrap()).unwrap()
    }

    // println!("Tokens: {tokens:?}");

    let parse_errs = if let Some(tokens) = &tokens {
        let (ast, parse_errs) = program_parser::<_, S::CS>()
            .map_with(|ast, e| (ast, e.span()))
            .parse(
                tokens
                    .as_slice()
                    .map((src.len()..src.len()).into(), |(t, s)| (t, s)),
            )
            .into_output_errors();

        // Note: if we wanted to also proceed with a error-recovered AST, this filter predicate needs to be changed, and the errors would still need
        // to be propagated somehow.
        if let Some((program, file_span)) = ast.filter(|_| errs.len() + parse_errs.len() == 0) {
            let res = interpret::<S>(program);
            match res.op_ctx_and_map {
                Ok((op_ctx, fns_to_ids)) => {
                    return InterpreterResult {
                        op_ctx_and_map: Ok((op_ctx, fns_to_ids)),
                        state_map: res.state_map,
                    }
                }
                Err(e) => {

                    let detailed_message = format!("{:?}", e);
                    let error_span = e.current_context().span;

                    let line_col_span = span_into_line_col_start_and_line_col_end(&error_span, src);

                    // the amount of spaces depends on the printed line number of the error.
                    // this is just for prettier formatting below of the pipe prefix.
                    // ariadne tracking issue: https://github.com/zesterer/ariadne/issues/68
                    let line_num = line_col_span.line_end;
                    let num_spaces = format!("{line_num}").len();
                    let spaces = " ".repeat(num_spaces);

                    // TODO: need to consider here if color is enabled or not.
                    let detailed_message_pipe_mapped = detailed_message
                        .lines()
                        .map(|line| format!(" [38;5;240m {spaces}│[0m  {line}"))
                        .collect::<Vec<_>>()
                        .join("\n");

                    // build report with error
                    let err_string = string_of_report(filename.clone(), src, Report::build(
                        ReportKind::Error,
                        (filename.clone(), error_span.into_range()),
                    )
                        .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte).with_color(color_enabled))
                        .with_message(e.to_string())
                        .with_label(
                            Label::new((filename.clone(), error_span.into_range()))
                                .with_message(format!("detailed message:\n{detailed_message_pipe_mapped}"))
                                .with_color(Color::Red),
                        )
                        .finish());

                    let err = Err(WithLineColSpans {
                        value: err_string,
                        spans: vec![line_col_span],
                    });
                    return InterpreterResult {
                        op_ctx_and_map: err,
                        state_map: res.state_map,
                    }
                }
            }
        }

        parse_errs
    } else {
        Vec::new()
    };

    let mut output_buf = BufWriter::new(Vec::new());

    let mut line_col_spans = Vec::new();

    errs.into_iter()
        .map(|e| e.map_token(|c| c.to_string()))
        .chain(
            parse_errs
                .into_iter()
                .map(|e| e.map_token(|tok| tok.to_string())),
        )
        .for_each(|e| {
            line_col_spans.push(span_into_line_col_start_and_line_col_end(&e.span(), src));
            Report::build(ReportKind::Error, (filename.clone(), e.span().into_range()))
                .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte).with_color(color_enabled))
                .with_message(e.to_string())
                .with_label(
                    Label::new((filename.clone(), e.span().into_range()))
                        .with_message(e.reason().to_string())
                        .with_color(Color::Red),
                )
                .with_labels(e.contexts().map(|(label, span)| {
                    Label::new((filename.clone(), span.into_range()))
                        .with_message(format!("while parsing this {label}"))
                        .with_color(Color::Yellow)
                }))
                .finish()
                .write(sources([(filename.clone(), src)]), &mut output_buf)
                .unwrap()
        });

    let output = String::from_utf8(output_buf.into_inner().unwrap()).unwrap();
    InterpreterResult {
        op_ctx_and_map: Err(WithLineColSpans {
            value: output,
            spans: line_col_spans,
        }),
        state_map: HashMap::new(),
    }
}

#[derive(Clone)]
pub struct WithLineColSpans<T> {
    pub value: T,
    pub spans: Vec<LineColSpan>,
}

#[derive(Clone)]
pub struct LineColSpan {
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
}

fn span_into_line_col_start_and_line_col_end(span: &Span, src: &str) -> LineColSpan {
    let start = span.start();
    let end = span.end();
    let mut line_start = 1;
    let mut col_start = 1;
    let mut line_end = 1;
    let mut col_end = 1;

    for (i, c) in src.bytes().enumerate() {
        if i < start {
            if c == b'\n' {
                line_start += 1;
                col_start = 1;
            } else {
                col_start += 1;
            }
            // line_end and col_end must be at least this.
            line_end = line_start;
            col_end = col_start;
        } else if i < end {
            if c == b'\n' {
                line_end += 1;
                col_end = 1;
            } else {
                col_end += 1;
            }
        } else {
            break;
        }
    }

    LineColSpan {
        line_start,
        col_start,
        line_end,
        col_end,
    }
}

/// Compared to the syntax_macro, this will only parse at runtime. The syntax_macro will parse at runtime as well,
/// but will compile-error if the syntax is invalid.
#[macro_export]
macro_rules! grabapl_parse {
    ($semantics:ty, $($t:tt)*) => {syntax::parse_to_op_ctx_and_map::<$semantics>(stringify!($($t)*))};
}

#[macro_export]
macro_rules! grabapl_defs {
    ($fn_name:ident, $semantics:ty, $($t:tt)*) => {
        fn $fn_name() -> (OperationContext<$semantics>, std::collections::HashMap<&'static str, grabapl::prelude::OperationId>) {
            $crate::grabapl_parse!($semantics, $($t)*)
        }
    };
}