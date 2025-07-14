mod experimental;
pub mod graph;
pub mod operation;
pub mod semantics;
pub mod util;
#[cfg(feature = "serde")]
mod serde;

use ::serde::{Deserialize, Serialize};
use derive_more::From;
pub use graph::DotCollector;
pub use graph::EdgeInsertionOrder;
pub use graph::EdgeKey;
pub use graph::Graph;
pub use graph::NodeKey;
pub use graph::OperationContext;
pub use graph::OperationId;
use internment::Intern;
pub use semantics::Semantics;
use crate::util::MyInternString;

/// A marker for substitution in the graph.
///
/// Useful for programmatically defined operations to know the substitution of their input pattern.
#[derive(derive_more::Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
#[debug("P({_0})")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SubstMarker(pub MyInternString);
interned_string_newtype!(SubstMarker);

// TODO: maybe we could have an input builder? basically we want to have one connected component per input.
// then we allow building an input graph with the builder, but the finalize method checks that we have exactly one input node
// (actually we could enforce that statically via it being the entry point) and that it is in fact weakly connected (ie ignoring edge direction)
// The input pattern for the Operation would then instead be a Vec of those input connected component patterns.

// TODO: What if two separate connected components overlap in the substitution? this leads to 'node references' to some degree.
// Probably only really bad if the 'shape' of that node changes while another reference to it expects something else. eg deleting the node or changing its type

#[cfg(test)]
mod tests {
    use super::*;
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
