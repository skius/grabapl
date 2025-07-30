#![doc = include_str!("../README.md")]

extern crate core;

mod experimental;
pub mod graph;
pub mod operation;
pub mod semantics;
#[cfg(feature = "serde")]
mod serde;
pub mod util;

use crate::util::InternString;
use ::serde::{Deserialize, Serialize};
use derive_more::From;
pub use graph::EdgeInsertionOrder;
pub use graph::EdgeKey;
pub use graph::Graph;
pub use graph::NodeKey;
pub use semantics::Semantics;

/// A marker for substitution in the graph.
///
/// Useful for programmatically defined operations to know the substitution of their input pattern.
#[derive(derive_more::Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
#[debug("P({_0})")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SubstMarker(pub InternString);
interned_string_newtype!(SubstMarker);

// TODO: maybe we could have an input builder? basically we want to have one connected component per input.
// then we allow building an input graph with the builder, but the finalize method checks that we have exactly one input node
// (actually we could enforce that statically via it being the entry point) and that it is in fact weakly connected (ie ignoring edge direction)
// The input pattern for the Operation would then instead be a Vec of those input connected component patterns.

// TODO: What if two separate connected components overlap in the substitution? this leads to 'node references' to some degree.
// Probably only really bad if the 'shape' of that node changes while another reference to it expects something else. eg deleting the node or changing its type

pub mod prelude {
    pub use super::SubstMarker;
    pub use crate::graph::{Graph, NodeKey};
    pub use crate::operation::builder::{BuilderOpLike, OperationBuilder};
    pub use crate::operation::builtin::LibBuiltinOperation;
    pub use crate::operation::signature::OperationSignature;
    pub use crate::operation::signature::parameter::{GraphWithSubstitution, OperationParameter};
    pub use crate::operation::signature::parameterbuilder::OperationParameterBuilder;
    pub use crate::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
    pub use crate::operation::{
        BuiltinOperation, Operation, OperationContext, OperationId, run_from_concrete,
    };
    pub use crate::semantics::{
        AbstractGraph, AbstractJoin, AbstractMatcher, ConcreteGraph, ConcreteToAbstract, Semantics,
    };
}
