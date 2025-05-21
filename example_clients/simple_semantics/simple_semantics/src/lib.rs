use std::collections::HashMap;
use grabapl::graph::semantics::{AbstractGraph, AbstractMatcher, AnyMatcher, ConcreteGraph, ConcreteToAbstract, Semantics};
use grabapl::{DotCollector, EdgeInsertionOrder, OperationContext, WithSubstMarker};
use grabapl::graph::operation::run_operation;
use grabapl::graph::pattern::{OperationArgument, OperationOutput, OperationParameter, ParameterSubstition};

pub struct SimpleSemantics;

pub enum EdgePattern {
    Wildcard,
    Exact(String),
}

pub struct EdgeMatcher;
impl AbstractMatcher for EdgeMatcher {
    type Abstract = EdgePattern;
    fn matches(arg: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (arg, parameter) {
            (_, EdgePattern::Wildcard) => true,
            (EdgePattern::Exact(a), EdgePattern::Exact(b)) => a == b,
            (_, _) => false,
        }
    }
}

pub struct NodeConcreteToAbstract;
pub struct EdgeConcreteToAbstract;

impl ConcreteToAbstract for NodeConcreteToAbstract {
    type Concrete = i32;
    type Abstract = ();

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        ()
    }
}

impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = String;
    type Abstract = EdgePattern;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        EdgePattern::Exact(c.clone())
    }
}

impl Semantics for SimpleSemantics {
    type NodeConcrete = i32;
    type NodeAbstract = ();
    type EdgeConcrete = String;
    type EdgeAbstract = EdgePattern;
    type NodeMatcher = AnyMatcher<()>;
    type EdgeMatcher = EdgeMatcher;

    type NodeConcreteToAbstract = NodeConcreteToAbstract;
    type EdgeConcreteToAbstract = EdgeConcreteToAbstract;

    type BuiltinOperation = BuiltinOperation;
}

pub enum BuiltinOperation {
    AddNode,
    AppendChild,
    /// Labels nodes of a three-cycle with 1,2,3, and requires the edge between 3 and 1 to be labelled "cycle"
    /// Only the first node is used as explicit input, the others are inferred.
    IndexCycle,
    SetValue(Box<dyn Fn() -> i32>),
    AddEdge,
    SetEdgeValueToCycle,
}

impl grabapl::graph::operation::BuiltinOperation for BuiltinOperation {
    type S = SimpleSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        match self {
            BuiltinOperation::AddNode => {
                let mut g = grabapl::graph::Graph::new();
                OperationParameter {
                    explicit_input_nodes: vec![],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::new(),
                }
            }
            BuiltinOperation::AppendChild => {
                // Expects a child
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    // TODO: this is scary, because NodeKeys are not a newtype yet, and neither are SubstMarkers.
                    subst_to_node_keys: HashMap::from([(0, a)]),
                }
            }
            BuiltinOperation::IndexCycle => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                let b = g.add_node(WithSubstMarker::new(1, ()));
                let c = g.add_node(WithSubstMarker::new(2, ()));
                g.add_edge(a, b, EdgePattern::Wildcard);
                g.add_edge(b, c, EdgePattern::Wildcard);
                g.add_edge(c, a, EdgePattern::Exact("cycle".to_string()));
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b), (2, c)]),
                }
            }
            BuiltinOperation::SetValue(_) => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                }
            }
            BuiltinOperation::AddEdge => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                let b = g.add_node(WithSubstMarker::new(1, ()));
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                }
            }
            BuiltinOperation::SetEdgeValueToCycle => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                let b = g.add_node(WithSubstMarker::new(1, ()));
                g.add_edge(a, b, EdgePattern::Wildcard);
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                }
            }
        }
    }

    fn apply_abstract(&self, g: &mut AbstractGraph<Self::S>, argument: OperationArgument, substitution: &ParameterSubstition) {
        match self {
            BuiltinOperation::AddNode => {
                g.add_node(());
            }
            BuiltinOperation::AppendChild => {
                let parent = substitution.mapping[&0];
                let child = g.add_node(());
                // TODO: this EdgePattern is weird.
                //  On the one hand, we know for a fact this is an exact "" that will be added, so in type-theory, we correctly add the most precise type (Exact instead of Wildcard)
                //  But if this ever used as a _pattern_ (parameter), it is a *decision* we're making here. Exact will permit fewer matches.
                //  Realistically this is not a problem, because we don't run builtin operations on parameters. But we should be careful.
                g.add_edge_ordered(parent, child, EdgePattern::Exact("".to_string()), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
            }
            BuiltinOperation::IndexCycle => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::SetValue(_) => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::AddEdge => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                g.add_edge_ordered(a, b, EdgePattern::Exact("".to_string()), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
            }
            BuiltinOperation::SetEdgeValueToCycle => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *g.get_mut_edge_attr((a, b)).unwrap() = EdgePattern::Exact("cycle".to_string());
            }
        }
    }

    fn apply(
        &self,
        graph: &mut ConcreteGraph<SimpleSemantics>,
        argument: OperationArgument,
        substitution: &ParameterSubstition,
    ) -> OperationOutput {
        let mut new_nodes = HashMap::new();
        match self {
            BuiltinOperation::AddNode => {
                let new_concrete_node = graph.add_node(0);
                new_nodes.insert(0, new_concrete_node);
            }
            BuiltinOperation::AppendChild => {
                let parent = substitution.mapping[&0];
                let child = graph.add_node(0);
                graph.add_edge_ordered(parent, child, "".to_string(), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
                new_nodes.insert(0, child);
            }
            BuiltinOperation::IndexCycle => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                let c = substitution.mapping[&2];
                *graph.get_mut_node_attr(a).unwrap() = 1;
                *graph.get_mut_node_attr(b).unwrap() = 2;
                *graph.get_mut_node_attr(c).unwrap() = 3;
            }
            BuiltinOperation::SetValue(f) => {
                let a = substitution.mapping[&0];
                *graph.get_mut_node_attr(a).unwrap() = f();
            }
            BuiltinOperation::AddEdge => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                graph.add_edge_ordered(a, b, "".to_string(), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
            }
            // TODO: make this generic over its value
            BuiltinOperation::SetEdgeValueToCycle => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *graph.get_mut_edge_attr((a, b)).unwrap() = "cycle".to_string();
            }
        }

        OperationOutput {
            new_nodes,
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

    }
}
