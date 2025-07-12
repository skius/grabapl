use crate::Graph;
use crate::graph::{EdgeAttribute, NodeAttribute};
use crate::operation::BuiltinOperation;
use crate::operation::query::BuiltinQuery;
use petgraph::data::Build;
// /// Returns the corresponding abstract value/type for a given concrete value.
// pub trait ToAbstract {
//     type Abstract;
//
//     fn to_abstract(&self) -> Self::Abstract;
// }

/// This matcher always returns true.
#[derive(Default)]
pub struct AnyMatcher<A> {
    phantom_data: std::marker::PhantomData<A>,
}

impl<A> AbstractMatcher for AnyMatcher<A> {
    type Abstract = A;

    fn matches(_argument: &Self::Abstract, _parameter: &Self::Abstract) -> bool {
        true
    }
}

pub trait AbstractMatcher {
    /// The type this matcher operates on.
    type Abstract;

    /// Decides if the argument type can be assigned to the parameter type.
    /// In other words, it checks if `argument` is a subtype of `parameter`.
    // TODO: rename "arg_matches_param"?
    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool;
}

/// A basic AbstractJoin that can join a type and a supertype into the supertype.
///
/// For basic cases this is enough, but as soon as you have a more complex type system (i.e.,
/// one where you have incomparable types), this Join is too simplistic and will not give you the
/// best performance.
///
/// # Example
/// If you have a type system with two types, `a` and `b`, where `a <: b`, then this Join will
/// return `a` when joining `a` and `a`, and `b` when joining `a` and `b`.
///
/// However, if you have a third type `c` with `c <: b`, i.e., `c` is not comparable to `a`, then
/// the join will not be able to join `a` and `c`, even though `b` would be a valid join.
#[derive(Default)]
pub struct MatchJoiner<M: AbstractMatcher<Abstract: Clone>> {
    phantom: std::marker::PhantomData<M>,
}

impl<M: AbstractMatcher<Abstract: Clone>> AbstractJoin for MatchJoiner<M> {
    type Abstract = M::Abstract;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        if M::matches(a, b) {
            // a <: b, so we return b
            Some(b.clone())
        } else {
            if M::matches(b, a) {
                // b <: a, so we return a
                Some(a.clone())
            } else {
                None
            }
        }
    }
}

pub trait AbstractJoin {
    /// The type this join operates on.
    type Abstract;

    /// Returns the abstract type that is the join of the two abstract types, i.e.,
    /// the most specific type that is a supertype of both, if it exists.
    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        // Default implementation returns None, meaning no join exists.
        // Note that this is probably a bit absurd, as in the very least if two nodes are equal
        // (either via Eq or via mathes(a,b) and matches(b,a)), then the join is the same node.
        // But this would induce a Clone requirement which I don't want to have just yet.
        // TODO: revisit if Abstract: Clone is useful.
        // ==> we have MatchJoiner now. If we had Clone, we could add a type default to the Semantics trait for type NodeJoin = MatchJoiner<Self::NodeMatcher>;
        None
    }
}

/// Defines the semantics of a client implementation.
pub trait Semantics {
    /// A data graph's nodes contain values of this type.
    /// PL analogy: values.
    type NodeConcrete: Clone;
    /// An operation can define patterns for nodes using this type.
    /// PL analogy: types.
    type NodeAbstract: Clone;
    /// A data graph's edges contain values of this type.
    /// PL analogy: values.
    type EdgeConcrete: Clone;
    /// An operation can define patterns for edges using this type.
    /// PL analogy: types.
    type EdgeAbstract: Clone;
    /// The specific matching process for nodes.
    type NodeMatcher: AbstractMatcher<Abstract = Self::NodeAbstract>;
    /// The specific matching process for edges.
    type EdgeMatcher: AbstractMatcher<Abstract = Self::EdgeAbstract>;
    /// The specific join process for nodes.
    type NodeJoin: AbstractJoin<Abstract = Self::NodeAbstract>;
    /// The specific join process for edges.
    type EdgeJoin: AbstractJoin<Abstract = Self::EdgeAbstract>;

    // TODO: does the inverse make sense?
    // Could we somehow benefit from something that takes (abstract, concrete) and returns an 'unwrapped' concrete?
    // eg: abstract is enum { i32, String }, and concrete is enum { i32(i32), String(String) }, then
    // the function would unwrap the specific type we need. but I don't think there is a way in rust for this to
    // help us statically.
    type NodeConcreteToAbstract: ConcreteToAbstract<Concrete = Self::NodeConcrete, Abstract = Self::NodeAbstract>;
    type EdgeConcreteToAbstract: ConcreteToAbstract<Concrete = Self::EdgeConcrete, Abstract = Self::EdgeAbstract>;

    /// Builtin operations are of this type.
    type BuiltinOperation: BuiltinOperation<S = Self>;
    /// Queries are of this type
    type BuiltinQuery: BuiltinQuery<S = Self>;

    fn new_concrete_graph() -> ConcreteGraph<Self> {
        Graph::new()
    }

    fn new_abstract_graph() -> AbstractGraph<Self> {
        Graph::new()
    }

    fn join_edges(a: &Self::EdgeAbstract, b: &Self::EdgeAbstract) -> Option<Self::EdgeAbstract> {
        Self::EdgeJoin::join(a, b)
    }

    fn join_nodes(a: &Self::NodeAbstract, b: &Self::NodeAbstract) -> Option<Self::NodeAbstract> {
        Self::NodeJoin::join(a, b)
    }

    // TODO: Assert that the node keys are the same
    fn concrete_to_abstract(c: &ConcreteGraph<Self>) -> AbstractGraph<Self> {
        let mut abstract_graph = Graph::new();
        for (node_key, node_concrete) in c.nodes() {
            let node_abstract = Self::NodeConcreteToAbstract::concrete_to_abstract(&node_concrete);
            // TODO: make this better (don't depend on Graph internals)
            abstract_graph.graph.add_node(node_key);
            abstract_graph
                .node_attr_map
                .insert(node_key, NodeAttribute::new(node_abstract));
        }
        abstract_graph.max_node_key = c.max_node_key;

        for (src, dst, weight) in c.graph.all_edges() {
            let edge_abstract = Self::EdgeConcreteToAbstract::concrete_to_abstract(weight.attr());
            // TODO: make this better (don't depend on Graph internals)
            let new_edge_attr = weight.with(edge_abstract);
            abstract_graph.graph.add_edge(src, dst, new_edge_attr);
        }

        abstract_graph
    }
}

pub type ConcreteGraph<S: Semantics> =
    Graph<<S as Semantics>::NodeConcrete, <S as Semantics>::EdgeConcrete>;

pub type AbstractGraph<S: Semantics> =
    Graph<<S as Semantics>::NodeAbstract, <S as Semantics>::EdgeAbstract>;

pub trait ConcreteToAbstract {
    type Concrete;
    type Abstract;
    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract;
}
