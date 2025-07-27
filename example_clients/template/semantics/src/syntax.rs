//! Defines the textual syntax of our semantics for plugging into `grabapl`'s syntax parser.
//!
//! This includes:
//! * Syntax of node types and edge types
//! * Custom arguments to our builtin operations and queries.
//!
//! The main glue trait for this is [`SemanticsWithCustomSyntax`], which defines how to convert
//! between parsed syntax and the semantics types, operations, and queries.
//!
//! There is also the [`CustomSyntax`] trait, which is separated from the semantics to allow for
//! reusable node and edge type definitions across different semantics implementations with similar types.
//!
//! We will implement everything from scratch.
//!
//! See the tests of this module for syntax examples.
//!
//! `grabapl` uses [`chumsky`], a parser combinator library, to parse its syntax.

use chumsky::input::ValueInput;
use chumsky::{select, Parser};
use chumsky::prelude::just;
use syntax::custom_syntax::{CustomSyntax, SemanticsWithCustomSyntax};
use syntax::{MacroArgs, Span, Token};
use syntax::interpreter::lex_then_parse;
use crate::{EdgeType, NodeType, TheSemantics, TheOperation, TheQuery, IntComparison, NodeValue, EdgeValue};

/// This type glues together the type definitions and parsing logic for our custom syntax via
/// its implementation of [`CustomSyntax`].
#[derive(Clone, Debug)]
pub struct TheCustomSyntax;

/// Helper that provides a parser for edge values from [`grabapl_syntax`](syntax)'s [`Token`] type.
pub fn edge_value_parser<'src: 'tokens, 'tokens, I: ValueInput<'tokens, Token=Token<'src>, Span=Span>>() -> impl Parser<'tokens, I, EdgeValue, chumsky::extra::Err<chumsky::error::Rich<'tokens, Token<'src>, Span>>> + Clone {
    let unit = just(Token::Ctrl('(')).then_ignore(just(Token::Ctrl(')'))).to(EdgeValue::Unit);
    let specific_string = select! { Token::Str(s) => s.to_owned() }.map(EdgeValue::String);
    let integer = select! { Token::Num(i) => i }.map(EdgeValue::Integer);

    unit.or(specific_string).or(integer)
}

/// Helper that provides a parser for node values from [`grabapl_syntax`](syntax)'s [`Token`] type.
pub fn node_value_parser<'src: 'tokens, 'tokens, I: ValueInput<'tokens, Token=Token<'src>, Span=Span>>() -> impl Parser<'tokens, I, NodeValue, chumsky::extra::Err<chumsky::error::Rich<'tokens, Token<'src>, Span>>> + Clone {
    let int = select! { Token::Num(i) => i }.map(NodeValue::Integer);
    let string = select! { Token::Str(s) => s.to_owned() }.map(NodeValue::String);

    int.or(string)
}


impl CustomSyntax for TheCustomSyntax {
    /// We will directly use the semantics' node type as the syntax struct.
    type AbstractNodeType = NodeType;
    /// We will directly use the semantics' edge type as the syntax struct.
    type AbstractEdgeType = EdgeType;

    /// We need to provide a parser for the node type that consumes `grabapl`'s [`Token`] type.
    ///
    /// Our node types are straightforward: we'll support parsing `int`, `string`, and `any`.
    ///
    /// See [`CustomSyntax::get_node_type_parser`]'s documentation for more information.
    fn get_node_type_parser<'src: 'tokens, 'tokens, I: ValueInput<'tokens, Token=Token<'src>, Span=Span>>() -> impl Parser<'tokens, I, Self::AbstractNodeType, chumsky::extra::Err<chumsky::error::Rich<'tokens, Token<'src>, Span>>> + Clone {
        // we parse "int", "string", or "any"
        let int = just(Token::Ident("int")).to(NodeType::Integer);
        let string = just(Token::Ident("string")).to(NodeType::String);
        let any = just(Token::Ident("any")).to(NodeType::Any);

        int.or(string).or(any)
    }

    /// We need to provide a parser for the edge type that consumes `grabapl`'s [`Token`] type.
    ///
    /// We will support the following edge types:
    /// - `()`: the unit edge type
    /// - `"some string"`: a string edge type with a statically known specific string value
    /// - `string`: a string edge type with an arbitrary string value
    /// - `int`: an integer edge type
    /// - `*` or `any`: a wildcard edge type that matches any edge
    ///
    /// See [`CustomSyntax::get_edge_type_parser`]'s documentation for more information.
    fn get_edge_type_parser<'src: 'tokens, 'tokens, I: ValueInput<'tokens, Token=Token<'src>, Span=Span>>() -> impl Parser<'tokens, I, Self::AbstractEdgeType, chumsky::extra::Err<chumsky::error::Rich<'tokens, Token<'src>, Span>>> + Clone {
        let unit = just(Token::Ctrl('(')).then_ignore(just(Token::Ctrl(')'))).to(EdgeType::Unit);
        let specific_string = select! { Token::Str(s) => s }.map(ToOwned::to_owned).map(EdgeType::ExactString);
        let any_string = just(Token::Ident("string")).to(EdgeType::String);
        let integer = just(Token::Ident("int")).to(EdgeType::Integer);
        let wildcard = just(Token::Ident("any")).or(just(Token::Ctrl('*'))).to(EdgeType::Any);

        unit.or(specific_string).or(any_string).or(integer).or(wildcard)
    }
}

impl SemanticsWithCustomSyntax for TheSemantics {
    type CS = TheCustomSyntax;

    /// Parses a function name with additional arguments into a [`TheOperation`].
    ///
    /// The additional arguments provided can be used to construct a operation that requires
    /// additional "meta-arguments" to be passed.
    ///
    /// See the implementation of this function for the precise syntax we support for our builtin operations.
    ///
    /// # Example
    ///
    /// For example, the [`TheOperation::AddConstant`] operation requires the user to specify
    /// *which* constant value to add. This will be passed in the `args` parameter.
    ///
    /// A user might write a function call in any of these forms:
    /// ```rust,ignore
    /// add_constant<5>(some_node);
    /// add_constant`5`(some_node);
    /// add_constant%5%(some_node);
    /// ```
    /// The "5" portion will be passed in the `args` parameter as a string. This function can
    /// then interpret that however it wants.
    fn find_builtin_op(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinOperation> {
        match name {
            "new_node" | "add_node" => {
                // optional args. if not provided, we just return a int(0) node.
                let Some(args) = args else {
                    return Some(TheOperation::NewNode { value: NodeValue::Integer(0) });
                };
                let args_src = args.0;
                // if args_src is provided, it must be a node value
                let value_parser = node_value_parser();
                // we reuse `syntax`'s lexer, because it's enough for our purposes.
                let res = lex_then_parse(args_src, value_parser).ok()?;
                Some(TheOperation::NewNode { value: res })
            }
            "remove_node" | "delete_node" => {
                Some(TheOperation::RemoveNode)
            }
            "append_snd_to_fst" => {
                Some(TheOperation::AppendSndToFst)
            }
            "add_snd_to_fst" => {
                Some(TheOperation::AddSndToFst)
            }
            "add_constant" => {
                // we expect a single argument that is a number
                let args_src = args?.0;
                let int = select! { Token::Num(i) => i };
                let constant = lex_then_parse(args_src, int).ok()?;
                Some(TheOperation::AddConstant { constant })
            }
            "copy_value_from_to" => {
                Some(TheOperation::CopyValueFromTo)
            }
            "new_edge" | "add_edge" => {
                // optional args. if not provided, we just return a unit edge.
                let Some(args) = args else {
                    return Some(TheOperation::NewEdge { value: EdgeValue::Unit });
                };
                let args_src = args.0;
                // if args_src is provided, it must be a valid edge value
                let value_parser = edge_value_parser();
                let res = lex_then_parse(args_src, value_parser).ok()?;

                Some(TheOperation::NewEdge { value: res })
            }
            "remove_edge" | "delete_edge" => {
                Some(TheOperation::RemoveEdge)
            }
            "extract_edge_to_node" | "extract_edge" => {
                Some(TheOperation::ExtractEdgeToNode)
            }
            "string_length" | "str_len" => {
                Some(TheOperation::StringLength)
            }
            // anything else is not our job.
            _ => None,
        }
    }

    /// Parses a query name with additional arguments into a [`TheQuery`].
    ///
    /// See [`SemanticsWithCustomSyntax::find_builtin_op`] for more information on the parameters.
    fn find_builtin_query(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinQuery> {
        match name {
            "is_eq" => {
                // expects an argument of node value to compare against.
                let args_src = args?.0;
                let value_parser = node_value_parser();
                let res = lex_then_parse(args_src, value_parser).ok()?;
                Some(TheQuery::IsEq { value: res })
            }
            "eq" | "equals" => {
                Some(TheQuery::Equal)
            }
            "gt" | "fst_gt_snd" => {
                Some(TheQuery::CompareInt { cmp: IntComparison::Gt })
            }
            "lt" | "fst_lt_snd" => {
                Some(TheQuery::CompareInt { cmp: IntComparison::Lt })
            }
            "gte" | "fst_gte_snd" => {
                Some(TheQuery::CompareInt { cmp: IntComparison::Gte })
            }
            "lte" | "fst_lte_snd" => {
                Some(TheQuery::CompareInt { cmp: IntComparison::Lte })
            }
            "fst_int_eq_snd" => {
                // Note: I suppose this is not really necessary, given the `(Any, Any)` TheQuery::Equal query exists.
                Some(TheQuery::CompareInt { cmp: IntComparison::Eq })
            }
            // anything else is not our job.
            _ => None,
        }
    }

    /// Our conversion is the identity function, because we directly parse into our semantics' types.
    fn convert_node_type(syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractNodeType) -> Option<Self::NodeAbstract> {
        Some(syn_typ)
    }

    /// Our conversion is the identity function, because we directly parse into our semantics' types.
    fn convert_edge_type(syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractEdgeType) -> Option<Self::EdgeAbstract> {
        Some(syn_typ)
    }
}

#[cfg(test)]
mod tests {
    use grabapl::prelude::run_from_concrete;
    use grabapl::Semantics;
    use super::*;

    // we also include some semantics tests here, because it's easy
    #[test]
    fn it_parses_successfully() {
        let res = syntax::try_parse_to_op_ctx_and_map::<TheSemantics>(stringify!(
            fn foo(
                /* node types */
                x: int,
                y: string,
                z: any,
                a: any,
                b: any
            ) [
                /* edge types */
                x -> y: "specific",
                y -> z: int,
                z -> x: string,
                a -> b: *,
                b -> a: (),
                a -> x: any,
            ] {

            }

            fn bar() {
                /* functions */
                let! new0_int = new_node();
                let! new1_int = new_node<5>();
                let! new2_string = new_node<"hello">();
                let! new3_string = new_node<" world">();

                remove_node(new0_int);
                append_snd_to_fst(new2_string, new3_string);
                if is_eq<"hello world">(new2_string) {

                } else {
                    diverge<"string not equal">();
                }

                let! two = add_node<2>();
                add_snd_to_fst(new1_int, two);
                add_constant<10>(new1_int);
                if is_eq<17>(new1_int) {

                } else {
                    diverge<"int not equal">();
                }

                copy_value_from_to(new2_string, new1_int);
                // new1_int should be a string now
                let! str_len = string_length(new1_int);
                if is_eq<11>(str_len) {

                } else {
                    diverge<"string length not equal">();
                }

                let! start = add_node();
                let! end = add_node();
                new_edge<"start to end">(start, end);
                if shape [
                    start -> end: "start to end",
                ] {

                } else {
                    diverge<"shape not matched">();
                }

                remove_edge(start, end);
                if shape [
                    start -> end: *,
                ] {
                    diverge<"edge not removed">();
                } else {

                }

                add_edge<42>(start, end);
                let! edge_val = extract_edge_to_node(start, end);
                if is_eq<42>(edge_val) {

                } else {
                    diverge<"extracted edge value not equal">();
                }

                /* queries */
                let! forty_two = new_node<42>();
                if eq(forty_two, edge_val) {
                    // forty_two and edge_val should be equal
                } else {
                    diverge<"forty_two not equal to edge_val">();
                }

                // int comparisons
                let! forty_one = new_node<41>();
                if gt(forty_two, forty_one) {
                    // forty_two should be greater than forty_one
                } else {
                    diverge<"forty_two not greater than forty_one">();
                }

                if lt(forty_one, forty_two) {
                    // forty_one should be less than forty_two
                } else {
                    diverge<"forty_one not less than forty_two">();
                }

                if lte(forty_one, forty_two) {
                    // forty_one should be less than or equal to forty_two
                } else {
                    diverge<"forty_one not less than or equal to forty_two">();
                }

                if gte(forty_two, forty_one) {
                    // forty_two should be greater than or equal to forty_one
                } else {
                    diverge<"forty_two not greater than or equal to forty_one">();
                }
            }
        ), true /* color enabled for error messages*/);
        let (op_ctx, fn_map) = res.op_ctx_and_map.unwrap();

        let bar_id = fn_map["bar"];

        let mut g = TheSemantics::new_concrete_graph();
        let res = run_from_concrete(&mut g, &op_ctx, bar_id, &[]);
        let res = res.unwrap();

    }
}