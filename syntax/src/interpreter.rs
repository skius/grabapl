use grabapl::prelude::*;
use crate::{CustomSyntax, MacroArgs, Program, Spanned};

pub trait SemanticsWithCustomSyntax: Semantics {
    type CS: CustomSyntax;

    fn find_builtin_op(name: &str, args: Option<MacroArgs<Self::CS>>) -> Option<Self::BuiltinOperation>;

    fn find_builtin_query(name: &str, args: Option<MacroArgs<Self::CS>>) -> Option<Self::BuiltinQuery>;

    fn convert_node_type(syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractNodeType) -> Self::NodeAbstract;
    fn convert_edge_type(syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractEdgeType) -> Self::EdgeAbstract;
}

pub fn interpret<S: SemanticsWithCustomSyntax>(prog: Spanned<Program<S::CS>>) {

}