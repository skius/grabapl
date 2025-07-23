pub mod example;
mod example_with_ref;

use crate::{MacroArgs, Span, Token};
use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::{IterParser, Parser, extra};
use grabapl::Semantics;
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

// TODO: figure out how to remove Debug constraint? or is it ok if not?
// TODO: actually, why does Builder::show_state need Debug? maybe lift that?
pub trait SemanticsWithCustomSyntax:
    Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone, NodeAbstract: Debug, EdgeAbstract: Debug>
{
    type CS: CustomSyntax;

    fn find_builtin_op(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinOperation>;

    fn find_builtin_query(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinQuery>;

    /// Returns an option so a more general CustomSyntax can be reused for multiple semantics.
    fn convert_node_type(
        syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractNodeType,
    ) -> Option<Self::NodeAbstract>;
    /// Returns an option so a more general CustomSyntax can be reused for multiple semantics.
    fn convert_edge_type(
        syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractEdgeType,
    ) -> Option<Self::EdgeAbstract>;
}
