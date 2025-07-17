pub mod minirust;

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

    /// May not parse ].
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
    Num(f64),
    // Str(&'src str),
    // Op(&'src str),
    Arrow,
    Ctrl(char),
    Ident(&'src str),
    Fn,
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
            Token::Ctrl(c) => write!(f, "{}", c),
            Token::Ident(i) => write!(f, "{}", i),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::LetBang => write!(f, "let!"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Shape => write!(f, "shape"),
            Token::MacroArgs(s) => write!(f, "[{}]", s),
            // Token::NodeType(s) => write!(f, "{}", s),
        }
    }
}

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    // A parser for numbers
    let num = text::int(10)
        .then(just('.').then(text::digits(10)).or_not())
        .to_slice()
        .from_str()
        .unwrapped()
        .map(Token::Num);

    // A parser for control characters (delimiters, semicolons, etc.)
    let ctrl = one_of("()[]{};,?:*=/<>\"'").map(Token::Ctrl);

    let arrow = just("->").to(Token::Arrow);

    // A parser for identifiers and keywords
    let ident = text::ascii::ident().map(|ident: &str| match ident {
        "fn" => Token::Fn,
        "let!" => Token::LetBang,
        "let" => Token::Let,
        "if" => Token::If,
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

    // '[', any text except '[', ']', then ']'. eg: [arg1, arg2, arg3]
    let macro_args = any::<&'src str, extra::Err<Rich<'src, char, Span>>>()
        .filter(|c| *c != '[' && *c != ']')
        .repeated()
        .to_slice()
        .delimited_by(just('['), just(']'))
        .map(Token::MacroArgs);

    // A single token can be one of the above
    // (macro_args needs to be before ctrl, since ctrl has the same prefix)
    let token = num.or(macro_args).or(arrow).or(ctrl).or(ident); //.or(node_type_src);

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
pub enum MacroArgs<CS: CustomSyntax> {
    Custom(CS::MacroArgType),
    Lib(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnCallExpr<'src, CS: CustomSyntax> {
    pub name: Spanned<&'src str>,
    pub macro_args: Spanned<MacroArgs<CS>>,
    pub args: Vec<Spanned<&'src str>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr<'src, CS: CustomSyntax> {
    FnCall {
        name: Spanned<&'src str>,
        macro_args: Spanned<MacroArgs<CS>>,
        // just for testing
        args: Vec<Spanned<FnNodeParam<'src, CS>>>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LetStmt<'src, CS: CustomSyntax> {
    pub bang: bool,
    pub ident: Spanned<&'src str>,
    pub call: Spanned<FnCallExpr<'src, CS>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeQueryParams<'src, CS: CustomSyntax> {
    pub params: Vec<Spanned<FnImplicitParam<'src, CS>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IfCond<'src, CS: CustomSyntax> {
    Query(Spanned<FnCallExpr<'src, CS>>),
    Shape(Spanned<ShapeQueryParams<'src, CS>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfStmt<'src, CS: CustomSyntax> {
    pub cond: IfCond<'src, CS>,
    pub then_block: Spanned<Block<'src, CS>>,
    // if it doesn't exist, take empty span at the end of then_block
    pub else_block: Spanned<Block<'src, CS>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReturnStmtMapping<'src, CS: CustomSyntax> {
    Node {
        /// The name of the output marker
        ret_name: Spanned<&'src str>,
        /// The node to return
        ident: Spanned<&'src str>,
    },
    Edge {
        src: Spanned<&'src str>,
        dst: Spanned<&'src str>,
        edge_type: Spanned<CS::AbstractEdgeType>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReturnStmt<'src, CS: CustomSyntax> {
    pub mapping: Vec<Spanned<FnImplicitParam<'src, CS>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement<'src, CS: CustomSyntax> {
    Let(Spanned<LetStmt<'src, CS>>),
    FnCall(Spanned<FnCallExpr<'src, CS>>),
    If(Spanned<IfStmt<'src, CS>>),
    Return(Spanned<ReturnStmt<'src, CS>>),
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
    pub functions: HashMap<&'src str, Spanned<FnDef<'src, CS>>>,
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
        );

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
        );

    let spanned_fn_implicit_param = spanned_fn_explicit_param.clone().map(|(explicit_param, span)| {
        (FnImplicitParam::Node(explicit_param), span)
    }).or(spanned_fn_implicit_edge_param);

    let fn_implicit_params = spanned_fn_implicit_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>();

    let block = recursive(|block| {

        let macro_args_str = select! {
            Token::MacroArgs(arg) => arg,
        };

        let lib_macro_args = any::<&'src str, extra::Err<Rich<'src, char, Span>>>()
            .filter(|c| *c != ']' && *c != ',')
            .repeated()
            .to_slice()
            .separated_by(just(','))
            .collect()
            .map(|args: Vec<&'src str>| args.into_iter().map(String::from).collect())
            .map(MacroArgs::<CS>::Lib)
            .padded();

        let tok_lib_macro_args = macro_args_str
            .try_map_with(move |src, e| {
                // parse with lib_macro_args
                lib_macro_args
                    .parse(src)
                    .into_result()
                    .map_err(|errs| Rich::custom(e.span(), format!("Failed to parse macro args: {}, errs: {:?}", src, errs)))
            });

        let fn_call_expr = ident_str
            .then(tok_lib_macro_args.map_with(|args, e| (args, e.span())))
            .then_ignore(just(Token::Ctrl('(')))
            .then(
                ident_str
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
            });

        let if_cond_shape = just(Token::Shape)
            .ignore_then(just(Token::Ctrl('[')))
            .ignore_then(fn_implicit_params.clone())
            .then_ignore(just(Token::Ctrl(']')))
            .map_with(|args, e| {
                (
                    ShapeQueryParams { params: args },
                    e.span(),
                )
            })
            .map(IfCond::Shape);

        let if_cond_query = fn_call_expr.clone()
            .map(|spanned_call| IfCond::Query(spanned_call));

        let if_cond = if_cond_shape.or(if_cond_query);

        // TODO: continue with entire if else statements.


        let fn_call_stmt = fn_call_expr.clone()
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
            .map_with(
                |((bang, (name, name_span)), (call, call_span)), e| {
                    (
                        LetStmt {
                            bang,
                            ident: (name, name_span),
                            call: (call, call_span),
                        },
                        e.span(),
                    )
                },
            )
            .map(Statement::Let)
            .map_with(|let_stmt, e| (let_stmt, e.span()))
            .labelled("let statement");


        let stmt = let_stmt
            .or(fn_call_stmt)
            .labelled("statement");

        let block_many_stmts = stmt
            .repeated()
            .collect()
            .map(|stmts| Block { statements: stmts })
            .labelled("block");

        block_many_stmts
    });



    let fn_return_signature = fn_implicit_params.clone();

    let optional_fn_return_signature = (
        just(Token::Arrow)
        .ignore_then(just(Token::Ctrl('(')))
        .ignore_then(fn_return_signature)
        .then_ignore(just(Token::Ctrl(')'))))
    .or_not()
    .map(|opt| opt.unwrap_or_default());



    let fn_explicit_params = spanned_fn_explicit_param
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>();

    let optional_fn_implicit_param = (just(Token::Ctrl('['))
        .ignore_then(fn_implicit_params)
        .then_ignore(just(Token::Ctrl(']'))))
    .or_not()
    .map(|opt| opt.unwrap_or_default());

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
            let mut funcs_map = HashMap::new();
            for (func, func_span) in functions_with_span {
                funcs_map.insert(func.name.0, (func, func_span));
            }
            (
                Program {
                    functions: funcs_map,
                },
                e.span(),
            )
        })
        .labelled("program");

    program
}

pub fn first_parser<'tokens, 'src: 'tokens, I, CS: CustomSyntax>()
-> impl Parser<'tokens, I, Spanned<Expr<'src, CS>>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>
        + SliceInput<
            'tokens,
            Token = Token<'src>,
            Span = Span,
            Slice = &'tokens [Spanned<Token<'src>>],
        >,
{
    let ident = select! {
        Token::Ident(ident) => ident,
    }
    .labelled("identifier")
    .map_with(|ident, e| (ident, e.span()));

    // let ident_list = ident
    //     .separated_by(just(Token::Ctrl(',')))
    //     .allow_trailing()
    //     .collect()
    //     .labelled("identifier list");

    let macro_args = select! {
        Token::MacroArgs(arg) => arg,
    }
    .labelled("client provided argument")
    .map_with(|arg, e| (arg, e.span()));

    let lib_macro_args = any::<&'src str, extra::Err<Rich<'src, char, Span>>>()
        .filter(|c| *c != ']' && *c != ',')
        .repeated()
        .to_slice()
        .separated_by(just(','))
        .collect()
        .map(|args: Vec<&'src str>| args.into_iter().map(String::from).collect())
        .map(MacroArgs::Lib)
        .padded();

    let macro_arg_parser =
        lib_macro_args.or(CS::get_macro_arg_parser().map(MacroArgs::Custom).padded());

    // ident : NodeType
    let fn_param_parser = ident
        .then_ignore(just(Token::Ctrl(':')))
        .then(CS::get_node_type_parser().map_with(|s, e| (s, e.span())))
        .try_map_with(
            |((name, n_span), (node_type, node_type_span)), overall_span| {
                Ok((
                    FnNodeParam {
                        name: (name, n_span),
                        node_type: (node_type, node_type_span),
                    },
                    overall_span.span(),
                ))
            },
        );
    // .try_map(|((name, n_span), (node_type_src, node_type_span)), overall_span| {
    //     // parse with CS::get_node_type_parser()
    //     let node_type = CS::get_node_type_parser()
    //         .parse(node_type_src)
    //         .into_result().map_err(|errs| {
    //         Rich::custom(node_type_span, format!("Failed to parse node type: {}, errs: {:?}", node_type_src, errs))
    //     })?;
    //     // unreachable!();
    //     Ok((FnParam {
    //         name: (name, n_span),
    //         node_type: (node_type, node_type_span),
    //     }, overall_span))
    // });

    let fn_param_list_parser = fn_param_parser
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .labelled("function parameters");

    // A parser for function calls
    let fn_call = ident
        .then(macro_args)
        // .map_with(|(name, args), e| Expr::FnCall {
        .try_map(
            move |((name, n_span), (args_src, args_src_span)), _overall_span| {
                // parse with lib_macro_args or macro_arg_parser

                // parse args_src with CS::get_arg_parser()
                let args = macro_arg_parser
                    .parse(args_src)
                    .into_result()
                    .map_err(|errs| {
                        Rich::custom(
                            args_src_span,
                            format!("Failed to parse arguments: {}, errs: {:?}", args_src, errs),
                        )
                    })?;
                Ok((name, n_span, args, args_src_span))
            },
        )
        .then_ignore(just(Token::Ctrl('(')))
        .then(fn_param_list_parser)
        .then_ignore(just(Token::Ctrl(')')))
        .map(|((name, n_span, args, args_src_span), args_list)| {
            // Create the expression
            Expr::FnCall {
                name: (name, n_span),
                macro_args: (args, args_src_span),
                args: args_list,
            }
        });

    // The main parser that returns the expression
    fn_call.map_with(|expr, e| (expr, e.span()))
}
