use petgraph::data::Build;
use crate::Graph;
use crate::graph::{EdgeAttribute, NodeAttribute};
use crate::graph::operation::{BuiltinOperation};
use crate::graph::operation::query::BuiltinQuery;
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
    // TODO: rename "arg_matches_param"?
    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool;
}

/// Defines the semantics of a client implementation.
pub trait Semantics {
    /// A data graph's nodes contain values of this type.
    /// PL analogy: values.
    type NodeConcrete;
    /// An operation can define patterns for nodes using this type.
    /// PL analogy: types.
    type NodeAbstract;
    /// A data graph's edges contain values of this type.
    /// PL analogy: values.
    type EdgeConcrete;
    /// An operation can define patterns for edges using this type.
    /// PL analogy: types.
    type EdgeAbstract;
    /// The specific matching process for nodes.
    type NodeMatcher: AbstractMatcher<Abstract = Self::NodeAbstract>;
    /// The specific matching process for edges.
    type EdgeMatcher: AbstractMatcher<Abstract = Self::EdgeAbstract>;

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
}

// TODO: do we need this? it's just easier to use this than spell it out
pub trait SemanticsClone: Semantics<NodeConcrete: Clone, EdgeConcrete: Clone> {
    
    fn concrete_to_abstract(c: &ConcreteGraph<Self>) -> AbstractGraph<Self> {
        let mut abstract_graph = Graph::new();
        for (node_key, node_concrete) in c.nodes() {
            let node_abstract = Self::NodeConcreteToAbstract::concrete_to_abstract(&node_concrete);
            // TODO: make this better (don't depend on Graph internals)
            abstract_graph.graph.add_node(node_key);
            abstract_graph.node_attr_map.insert(node_key, NodeAttribute::new(node_abstract));
        }
        abstract_graph.max_node_key = c.max_node_key;

        for (src, dst, weight) in c.graph.all_edges() {
            let edge_abstract = Self::EdgeConcreteToAbstract::concrete_to_abstract(&weight.edge_attr);
            // TODO: make this better (don't depend on Graph internals)
            let new_edge_attr = EdgeAttribute::new(edge_abstract, weight.source_out_order, weight.target_in_order);
            abstract_graph.graph.add_edge(src, dst, new_edge_attr);
        }

        abstract_graph
    }
}
impl<S: Semantics> SemanticsClone for S
where
    S::NodeConcrete: Clone,
    S::EdgeConcrete: Clone,
{}

pub type ConcreteGraph<S: Semantics> = Graph<<S as Semantics>::NodeConcrete, <S as Semantics>::EdgeConcrete>;

pub type AbstractGraph<S: Semantics> = Graph<<S as Semantics>::NodeAbstract, <S as Semantics>::EdgeAbstract>;

// impl<NC: ToAbstract + Clone, EC: ToAbstract + Clone> Graph<NC, EC>
// {
//     pub(crate) fn to_abstract(&self) -> Graph<NC::Abstract, EC::Abstract> {
//         let mut abstract_graph = Graph::new();
//         for (node_key, node_concrete) in self.nodes() {
//             let node_abstract = node_concrete.to_abstract();
//             // TODO: make this better (don't depend on Graph internals)
//             abstract_graph.graph.add_node(node_key);
//             abstract_graph.node_attr_map.insert(node_key, NodeAttribute::new(node_abstract));
//         }
//         abstract_graph.max_node_key = self.max_node_key;
// 
//         for (src, dst, weight) in self.graph.all_edges() {
//             let edge_abstract = weight.edge_attr.to_abstract();
//             // TODO: make this better (don't depend on Graph internals)
//             let new_edge_attr = EdgeAttribute::new(edge_abstract, weight.source_out_order, weight.target_in_order);
//             abstract_graph.graph.add_edge(src, dst, new_edge_attr);
//         }
// 
//         abstract_graph
//     }
// }

pub trait ConcreteToAbstract {
    type Concrete;
    type Abstract;
    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract;
}