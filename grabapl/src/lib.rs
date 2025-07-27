//! A library for **gra**ph-**ba**sed **p**rogramming **l**anguages with static analysis.
//!
//! Playground: [https://skius.github.io/grabapl/](https://skius.github.io/grabapl/)
//!
//! # Main Features
//! * The program state is a single global, directed graph.
//! * The type system is a shape-based type system (i.e., existence and absence of nodes and edges) composed
//!   with an arbitrary client-defined type system for node and edge values.
//!     * Nodes and edges can hold arbitrary values of arbitrary types.
//!     * See [`grabapl_template_semantics`] for an example client.
//! * No explicit loops, only recursion.
//! * Statically visible nodes and edges are guaranteed to exist at runtime. No nulls.
//! * Frontend-agnostic with a focus on intermediate abstract states:
//!     * The fundamental building blocks of programs are "instructions" that can stem from any source.
//!     * For example, a frontend may decide to be visual-first by visualizing intermediate states and
//!       turning interactive actions into instructions.
//!     * A text-based frontend is provided with [`grabapl_syntax`],
//!       supporting a Rust-like syntax with pluggable client-defined parsing rules.
//!
//! # Example
//! Using the [`grabapl_syntax`] frontend as example with the example node and edge type system from
//! [`grabapl_template_semantics`], here is an implementation of in-place bubble sort on a linked list:
//!
//! ```rust,ignore
//!
//! ```
//!
//! [`grabapl_template_semantics`]: https://crates.io/crates/grabapl_template_semantics
//! [`grabapl_syntax`]: https://crates.io/crates/grabapl_syntax

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::dot::DotCollector;
    // #[test]
    // this is an old test with old test matching behavior.
    fn child_order() {
        let mut collector = DotCollector::new();
        let mut graph = Graph::<&str, ()>::new();

        macro_rules! c {
            () => {
                collector.collect(&graph);
            };
        }

        c!();
        let a = graph.add_node("hello");
        c!();
        let b = graph.add_node("world");
        c!();

        graph.add_edge(a, b, ());
        c!();

        let next_edge = graph.next_outgoing_edge(a, (a, b));
        assert_eq!(next_edge, (a, b));

        let prev_edge = graph.prev_outgoing_edge(a, (a, b));
        assert_eq!(prev_edge, (a, b));

        let c = graph.add_node("foo");
        c!();
        graph.add_edge(a, c, ());
        c!();
        let next_edge = graph.next_outgoing_edge(a, (a, b));
        assert_eq!(next_edge, (a, c));
        let prev_edge = graph.prev_outgoing_edge(a, (a, c));
        assert_eq!(prev_edge, (a, b));
        let prev_edge = graph.prev_outgoing_edge(a, (a, b));
        assert_eq!(prev_edge, (a, c));

        let d = graph.add_node("bar");
        c!();
        graph.add_edge(a, d, ());
        c!();
        let next_edge = graph.next_outgoing_edge(a, (a, c));
        assert_eq!(next_edge, (a, d));
        let next_edge = graph.next_outgoing_edge(a, (a, d));
        assert_eq!(next_edge, (a, b));
        let prev_edge = graph.prev_outgoing_edge(a, (a, b));
        assert_eq!(prev_edge, (a, d));

        let before_b = graph.add_node("before_b");
        c!();
        graph.add_edge_ordered(
            a,
            before_b,
            (),
            EdgeInsertionOrder::Prepend,
            EdgeInsertionOrder::Append,
        );
        c!();

        let after_d = graph.add_node("after_d");
        c!();
        graph.add_edge_ordered(
            a,
            after_d,
            (),
            EdgeInsertionOrder::Append,
            EdgeInsertionOrder::Append,
        );
        c!();

        let next_edge = graph.next_outgoing_edge(a, (a, b));
        assert_eq!(next_edge, (a, c));
        let prev_edge = graph.prev_outgoing_edge(a, (a, b));
        assert_eq!(prev_edge, (a, before_b));
        let next_edge = graph.next_outgoing_edge(a, (a, d));
        assert_eq!(next_edge, (a, after_d));
        let prev_edge = graph.prev_outgoing_edge(a, (a, before_b));
        assert_eq!(prev_edge, (a, after_d));

        let dot = collector.finalize();
        println!("{}", dot);
        assert!(false);
    }
}
