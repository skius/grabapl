pub mod minirust;

use std::fmt;
use std::fmt::Debug;
use chumsky::{input::ValueInput, prelude::*};
use chumsky::input::MapExtra;

pub trait CustomSyntax: Clone + Debug + 'static  {
    type ArgType: Clone + fmt::Debug + Default;

    fn get_arg_parser<'src>() -> impl Parser<'src, &'src str, Self::ArgType, extra::Err<Rich<'src, char, Span>>> + Clone;
}

#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomSyntax;

impl CustomSyntax for MyCustomSyntax {
    type ArgType = Vec<String>;

    fn get_arg_parser<'src>() -> impl Parser<'src, &'src str, Self::ArgType, extra::Err<Rich<'src, char, Span>>> + Clone {
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
    ClientProvided(&'src str),
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
            Token::ClientProvided(s) => write!(f, "{}", s),
        }
    }
}


pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    // A parser for numbers
    let num = text::int(10)
        .then(just('.').then(text::digits(10)).or_not())
        .to_slice()
        .from_str()
        .unwrapped()
        .map(Token::Num);


    // A parser for control characters (delimiters, semicolons, etc.)
    let ctrl = one_of("()[]{};,?").map(Token::Ctrl);

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

    // '[', any text except '[', ']', then ']'. eg: [arg1, arg2, arg3]
    let client_provided_arg = any::<&'src str, extra::Err<Rich<'src, char, Span>>>()
        .filter(|c| *c != '[' && *c != ']')
        .repeated()
        .to_slice()
        .delimited_by(just('['), just(']'))
        .map(Token::ClientProvided);

    // A single token can be one of the above
    // (client_provided needs to be before ctrl, since ctrl has the same prefix)
    let token = num.or(client_provided_arg).or(arrow).or(ctrl).or(ident);

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

#[derive(Clone, Debug, PartialEq)]
pub enum MacroArgs<CS: CustomSyntax> {
    Custom(CS::ArgType),
    Lib(Vec<String>),
}

#[derive(Debug)]
pub enum Expr<'src, CS: CustomSyntax> {
    FnCall {
        name: Spanned<&'src str>,
        macro_args: Spanned<MacroArgs<CS>>,
        args: Vec<Spanned<&'src str>>,
    }
}



pub fn first_parser<'tokens, 'src: 'tokens, I, CS: CustomSyntax>(
) -> impl Parser<'tokens, I, Spanned<Expr<'src, CS>>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    let ident = select! {
        Token::Ident(ident) => ident,
    }.labelled("identifier")
        .map_with(|ident, e| (ident, e.span()));

    let ident_list = ident
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect()
        .labelled("identifier list");

    let client_provided = select! {
        Token::ClientProvided(arg) => arg,
    }.labelled("client provided argument")
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
        lib_macro_args.or(
        CS::get_arg_parser()
        .map(MacroArgs::Custom)
        .padded());

    // A parser for function calls
    let fn_call = ident
        .then(client_provided)
        // .map_with(|(name, args), e| Expr::FnCall {
        .try_map(move |((name, n_span), (args_src, args_src_span)), _overall_span| {
            // parse with lib_macro_args or macro_arg_parser


            // parse args_src with CS::get_arg_parser()
            let args = macro_arg_parser
                .parse(args_src)
                .into_result().map_err(|errs| {
                    Rich::custom(args_src_span, format!("Failed to parse arguments: {}, errs: {:?}", args_src, errs))
            })?;
            Ok((name, n_span, args, args_src_span))
        })
        .then_ignore(just(Token::Ctrl('(')))
        .then(ident_list)
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
    fn_call
        .map_with(|expr, e| (expr, e.span()))
}

