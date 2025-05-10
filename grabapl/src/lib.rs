pub mod graph;
pub mod pattern_match;

use crate::graph::*;
use petgraph::algo::subgraph_isomorphisms_iter;
use petgraph::dot::Dot;
use petgraph::prelude::{DiGraphMap, GraphMap, StableDiGraph};
use petgraph::visit::{IntoEdgesDirected, IntoNeighborsDirected, NodeRef};
use petgraph::{Direction, dot};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::iter;

pub use graph::DotCollector;
pub use graph::EdgeInsertionOrder;
pub use graph::EdgeKey;
pub use graph::Graph;
pub use graph::NodeKey;
pub use graph::OperationContext;
pub use graph::OperationId;
pub use graph::semantics::Semantics;

// TODO: should we instead have an 'AbstractAttribute' as well, and the pattern matcher works on that?
// From every concrete graph you can get its abstract graph. That should be like the type.
// so a concrete i32 attr node (say '5') would for example get mapped into a 'i32' node.
// Hmm. Then you would have operations acting on both concrete values but also abstract values.
// For example, an operation might take i32 i32 ANY as input, and turn it into i32 i32 i32. (this is the example of arg3 <- arg1 + arg2)
// this should be statically describable?
// But queries also need a place here. A pattern query definitely returns a node with abstract values, since that's
// the same 'language' that operation inputs speak where patterns are also used, but how do we do a query like "has equal values"?
// such a query would need to be on the concrete level.
// Aah - this does not matter. Queries at runtime typically dont result in value changes, instead they influence the control flow.
// So, 'concrete' queries and 'pattern' queries are unified:
// 1. statically, a query takes as input some abstract graph. This needs to match its expected pattern, so it works exactly like operations.
//    * then, it can produce static changes to the abstract graph, per branch.
//    * This is 'typed', so like a match arm in rust.
// 2. at runtime, these inputs are then replaced by concrete values.
//    * the concrete values decide where the control flow goes and in case of match-arms, which concrete
//      values to bind.
// In other words, a query needs both a concrete and an abstract implementation. I think this is the same as operations: they need the concrete changes, and the abstract pattern + if they change any types
//
//  ** UPDATE: **
// Because we'll want to work abstractly with a pattern graph, we'll want the pattern type to be the type that pattern matches against.
// In other words, we want the pattern type to be the analogue of the PL-"type", with subtyping. eg. a wildcard is just the analogue of the Top type


/// A marker for substitution in the graph.
///
/// Useful for programmatically defined operations to know the substitution of their input pattern.
pub type SubstMarker = u32;

pub struct WithSubstMarker<T> {
    marker: SubstMarker,
    value: T,
}

// TODO: figure out what to do for PatternKind/PatternWrapper
pub enum PatternKind {
    Input,
    Derived,
}

pub struct PatternWrapper<P> {
    pattern: P,
    marker: SubstMarker,
    kind: PatternKind,
}

// TODO: maybe we could have an input builder? basically we want to have one connected component per input.
// then we allow building an input graph with the builder, but the finalize method checks that we have exactly one input node
// (actually we could enforce that statically via it being the entry point) and that it is in fact weakly connected (ie ignoring edge direction)
// The input pattern for the Operation would then instead be a Vec of those input connected component patterns.

// TODO: What if two separate connected components overlap in the substitution? this leads to 'node references' to some degree.
// Probably only really bad if the 'shape' of that node changes while another reference to it expects something else. eg deleting the node or changing its type

impl<P> PatternWrapper<P> {
    pub fn new_input(pattern: P, marker: SubstMarker) -> Self {
        PatternWrapper {
            pattern,
            marker,
            kind: PatternKind::Input,
        }
    }

    pub fn new_derived(pattern: P, marker: SubstMarker) -> Self {
        PatternWrapper {
            pattern,
            marker,
            kind: PatternKind::Derived,
        }
    }

    pub fn get_pattern(&self) -> &P {
        &self.pattern
    }

    pub fn get_marker(&self) -> SubstMarker {
        self.marker
    }

    pub fn get_kind(&self) -> &PatternKind {
        &self.kind
    }
}

impl<T> WithSubstMarker<T> {
    pub fn new(marker: SubstMarker, value: T) -> Self {
        WithSubstMarker { marker, value }
    }

    pub fn get_value(&self) -> &T {
        &self.value
    }
}

// pub struct InputPattern<NPA: PatternAttributeMatcher, EPA: PatternAttributeMatcher> {
//     pub parameter_nodes: Vec<SubstMarker>,
//     pub pattern_graph: Graph<WithSubstMarker<NPA::Pattern>, EPA::Pattern>,
//     subst_to_node_keys: HashMap<SubstMarker, NodeKey>,
// }
//
// pub struct OperationInput<NA, EA> {
//     pub selected_inputs: Vec<NodeKey>,
//     pub graph: Graph<NA, EA>,
// }
//
// /// A trait for graph operations.
// ///
// /// The operation requires graphs with the given node and edge attribute types.
// pub trait Operation<NPA: PatternAttributeMatcher, EPA: PatternAttributeMatcher> {
//     /// The pattern to match against the graph.
//     fn input_pattern(&self) -> InputPattern<NPA, EPA>;
//     fn apply(
//         &mut self,
//         input: &mut OperationInput<NPA::Attr, EPA::Attr>,
//         subst: &HashMap<SubstMarker, NodeKey>,
//     ) -> Result<(), String>;
// }
//
// impl<NA: Clone, EA: Clone> Graph<NA, EA> {
//     pub fn run_operation<O, NPA, EPA>(
//         &mut self,
//         selected_inputs: Vec<NodeKey>,
//         op: &mut O,
//     ) -> Result<(), String>
//     where
//         O: Operation<NPA, EPA>,
//         NPA: PatternAttributeMatcher<Attr = NA>,
//         EPA: PatternAttributeMatcher<Attr = EA>,
//     {
//         let subst = {
//             let pattern = op.input_pattern(); // TODO: rename a to pattern b to data or similar...
//             let mut nm = |a: &NodeKey, b: &NodeKey| {
//                 let a_attr = pattern.get_node_attr(*a).unwrap();
//                 let b_attr = self.get_node_attr(*b).unwrap();
//                 NPA::matches(b_attr, &a_attr.value)
//             };
//             let mut em = |a: &EdgeAttribute<EPA::Pattern>, b: &EdgeAttribute<EA>| {
//                 EPA::matches(&b.edge_attr, &a.edge_attr)
//             };
//             let Some(mut mappings) = self.match_to_pattern(&pattern, &mut nm, &mut em) else {
//                 return Err("No matching pattern found".to_string());
//             };
//             let mapping = mappings.next().ok_or("Internal Error: No mapping found")?;
//             mapping
//                 .iter()
//                 .map(|(src, target)| (pattern.get_node_attr(*src).unwrap().marker, *target))
//                 .collect::<HashMap<_, _>>()
//         };
//
//         let mut op_input = OperationInput {
//             selected_inputs,
//             // TODO: get rid of clone
//             graph: self.clone(),
//         };
//
//         op.apply(&mut op_input, &subst)?;
//         Ok(())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::algo::subgraph_isomorphisms_iter;

    // #[test]
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
