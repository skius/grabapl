pub mod sample_user_defined_operations;

use grabapl::graph::operation::query::{
    AbstractQueryChange, AbstractQueryOutput, BuiltinQuery as BuiltinQueryTrait,
    ConcreteQueryOutput, EdgeChange, NodeChange,
};
use grabapl::graph::operation::run_operation;
use grabapl::graph::pattern::{
    OperationArgument, OperationOutput, OperationParameter, ParameterSubstitution,
};
use grabapl::graph::semantics::{
    AbstractGraph, AbstractMatcher, AnyMatcher, ConcreteGraph, ConcreteToAbstract, Semantics,
};
use grabapl::{DotCollector, EdgeInsertionOrder, OperationContext, WithSubstMarker};
use std::collections::HashMap;
use std::fmt::Debug;

pub struct SimpleSemantics;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub enum BuiltinQuery {
    HasChild,
    IsValueGt(i32),
    IsValueEq(i32),
    ValuesEqual,
    FirstGtSnd,
}

impl BuiltinQueryTrait for BuiltinQuery {
    type S = SimpleSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        match self {
            BuiltinQuery::HasChild => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinQuery::IsValueGt(_) => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinQuery::IsValueEq(_) => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinQuery::ValuesEqual => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
            BuiltinQuery::FirstGtSnd => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
        }
    }

    fn apply_abstract(
        &self,
        g: &mut AbstractGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) {
        match self {
            BuiltinQuery::HasChild => {
                // let parent = substitution.mapping[&0];
                // let child = g.add_node(());
                // g.add_edge_ordered(
                //     parent,
                //     child,
                //     EdgePattern::Wildcard,
                //     EdgeInsertionOrder::Append,
                //     EdgeInsertionOrder::Append,
                // );
                // changes.push(AbstractQueryChange::ExpectNode(NodeChange::NewNode(1, ())));
                // changes.push(AbstractQueryChange::ExpectEdge(
                //     EdgeChange::ChangeEdgeValue {
                //         from: 0,
                //         to: 1,
                //         edge: EdgePattern::Wildcard,
                //     },
                // ));
                // TODO: how to handle this? probably not needed.
            }
            BuiltinQuery::IsValueGt(val) => {
                // No abstract changes if the value is equal, since our type system cannot represent exact values.
            }
            _ => {
                // TODO decide what this method even does
                //  Imo, it should not necessarily mutate, so it would be fine to just get rid of it entirely.
            }
        }
    }

    fn query(
        &self,
        g: &mut ConcreteGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) -> ConcreteQueryOutput {
        let mut taken = false;
        match self {
            BuiltinQuery::HasChild => {
                todo!(
                    "TODO: how to handle this? we need a notion of the current 'known' graph in order to tell whether there really is a new child or not"
                )
            }
            BuiltinQuery::IsValueGt(val) => {
                let a = substitution.mapping[&0];
                if *g.get_node_attr(a).unwrap() > *val {
                    taken = true;
                }
            }
            BuiltinQuery::IsValueEq(val) => {
                let a = substitution.mapping[&0];
                if *g.get_node_attr(a).unwrap() == *val {
                    taken = true;
                }
            }
            BuiltinQuery::ValuesEqual => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                if g.get_node_attr(a) == g.get_node_attr(b) {
                    taken = true;
                }
            }
            BuiltinQuery::FirstGtSnd => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                if g.get_node_attr(a) > g.get_node_attr(b) {
                    taken = true;
                }
            }
        }
        ConcreteQueryOutput { taken }
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
    type BuiltinQuery = BuiltinQuery;
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
    SetEdgeValue(String),
    SetNodeValue(i32),
    CopyNodeValueTo,
    Decrement,
    Increment,
    DeleteNode,
    // TODO: 3-argument max: c <- max(a,b) would need to support aliasing of parameters...
    SetSndToMaxOfFstSnd,
}

impl Debug for BuiltinOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuiltinOperation::AddNode => write!(f, "AddNode"),
            BuiltinOperation::AppendChild => write!(f, "AppendChild"),
            BuiltinOperation::IndexCycle => write!(f, "IndexCycle"),
            BuiltinOperation::SetValue(_) => write!(f, "SetValue"),
            BuiltinOperation::AddEdge => write!(f, "AddEdge"),
            BuiltinOperation::SetEdgeValueToCycle => write!(f, "SetEdgeValueToCycle"),
            BuiltinOperation::SetEdgeValue(val) => write!(f, "SetEdgeValue({})", val),
            BuiltinOperation::SetNodeValue(val) => write!(f, "SetNodeValue({})", val),
            BuiltinOperation::CopyNodeValueTo => write!(f, "CopyNodeValueTo"),
            BuiltinOperation::Decrement => write!(f, "Decrement"),
            BuiltinOperation::Increment => write!(f, "Increment"),
            BuiltinOperation::DeleteNode => write!(f, "DeleteNode"),
            BuiltinOperation::SetSndToMaxOfFstSnd => write!(f, "SetSndToMaxOfFstSnd"),
        }
    }
}

impl Clone for BuiltinOperation {
    fn clone(&self) -> Self {
        match self {
            BuiltinOperation::AddNode => BuiltinOperation::AddNode,
            BuiltinOperation::AppendChild => BuiltinOperation::AppendChild,
            BuiltinOperation::IndexCycle => BuiltinOperation::IndexCycle,
            BuiltinOperation::SetValue(f) => BuiltinOperation::SetNodeValue(0), // TODO: fix?
            BuiltinOperation::AddEdge => BuiltinOperation::AddEdge,
            BuiltinOperation::SetEdgeValueToCycle => BuiltinOperation::SetEdgeValueToCycle,
            BuiltinOperation::SetEdgeValue(val) => BuiltinOperation::SetEdgeValue(val.clone()),
            BuiltinOperation::SetNodeValue(val) => BuiltinOperation::SetNodeValue(*val),
            BuiltinOperation::CopyNodeValueTo => BuiltinOperation::CopyNodeValueTo,
            BuiltinOperation::Decrement => BuiltinOperation::Decrement,
            BuiltinOperation::Increment => BuiltinOperation::Increment,
            BuiltinOperation::DeleteNode => BuiltinOperation::DeleteNode,
            BuiltinOperation::SetSndToMaxOfFstSnd => BuiltinOperation::SetSndToMaxOfFstSnd,
        }
    }
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
                    node_keys_to_subst: HashMap::new(),
                }
            }
            BuiltinOperation::AppendChild => {
                // Expects a child
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    // TODO: this is scary, because NodeKeys are not a newtype yet, and neither are SubstMarkers.
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinOperation::IndexCycle => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                let c = g.add_node(());
                g.add_edge(a, b, EdgePattern::Wildcard);
                g.add_edge(b, c, EdgePattern::Wildcard);
                g.add_edge(c, a, EdgePattern::Exact("cycle".to_string()));
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b), (2, c)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1), (c, 2)]),
                }
            }
            BuiltinOperation::SetValue(_) => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinOperation::AddEdge => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
            BuiltinOperation::SetEdgeValueToCycle => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                g.add_edge(a, b, EdgePattern::Wildcard);
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
            BuiltinOperation::SetEdgeValue(_) => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                g.add_edge(a, b, EdgePattern::Wildcard);
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
            BuiltinOperation::SetNodeValue(_) => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinOperation::CopyNodeValueTo => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
            BuiltinOperation::Decrement => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinOperation::Increment => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinOperation::DeleteNode => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a)]),
                    node_keys_to_subst: HashMap::from([(a, 0)]),
                }
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(());
                let b = g.add_node(());
                OperationParameter {
                    explicit_input_nodes: vec![0, 1],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b)]),
                    node_keys_to_subst: HashMap::from([(a, 0), (b, 1)]),
                }
            }
        }
    }

    fn apply_abstract(
        &self,
        g: &mut AbstractGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) -> OperationOutput {
        let mut new_nodes = HashMap::new();
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
                g.add_edge_ordered(
                    parent,
                    child,
                    EdgePattern::Exact("".to_string()),
                    EdgeInsertionOrder::Append,
                    EdgeInsertionOrder::Append,
                );
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
                g.add_edge_ordered(
                    a,
                    b,
                    EdgePattern::Exact("".to_string()),
                    EdgeInsertionOrder::Append,
                    EdgeInsertionOrder::Append,
                );
            }
            BuiltinOperation::SetEdgeValueToCycle => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *g.get_mut_edge_attr((a, b)).unwrap() = EdgePattern::Exact("cycle".to_string());
            }
            BuiltinOperation::SetEdgeValue(val) => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *g.get_mut_edge_attr((a, b)).unwrap() = EdgePattern::Exact(val.clone());
            }
            BuiltinOperation::SetNodeValue(val) => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::CopyNodeValueTo => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                // Noop as long as the abstract value is just the unit type...
                *g.get_mut_node_attr(b).unwrap() = *g.get_node_attr(a).unwrap();
            }
            BuiltinOperation::Decrement => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::Increment => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::DeleteNode => {
                let a = substitution.mapping[&0];
                g.remove_node(a);
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
        }
        OperationOutput { new_nodes: todo!("abstract operation output") }
    }

    fn apply(
        &self,
        graph: &mut ConcreteGraph<SimpleSemantics>,
        substitution: &ParameterSubstitution,
    ) -> OperationOutput {
        let mut new_nodes = HashMap::new();
        match self {
            BuiltinOperation::AddNode => {
                let new_concrete_node = graph.add_node(0);
                new_nodes.insert("new".into(), new_concrete_node);
            }
            BuiltinOperation::AppendChild => {
                let parent = substitution.mapping[&0];
                let child = graph.add_node(0);
                graph.add_edge_ordered(
                    parent,
                    child,
                    "".to_string(),
                    EdgeInsertionOrder::Append,
                    EdgeInsertionOrder::Append,
                );
                new_nodes.insert("child".into(), child);
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
                graph.add_edge_ordered(
                    a,
                    b,
                    "".to_string(),
                    EdgeInsertionOrder::Append,
                    EdgeInsertionOrder::Append,
                );
            }
            // TODO: make this generic over its value
            BuiltinOperation::SetEdgeValueToCycle => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *graph.get_mut_edge_attr((a, b)).unwrap() = "cycle".to_string();
            }
            BuiltinOperation::SetEdgeValue(val) => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *graph.get_mut_edge_attr((a, b)).unwrap() = val.clone();
            }
            BuiltinOperation::SetNodeValue(val) => {
                let a = substitution.mapping[&0];
                *graph.get_mut_node_attr(a).unwrap() = *val;
            }
            BuiltinOperation::CopyNodeValueTo => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                *graph.get_mut_node_attr(b).unwrap() = *graph.get_node_attr(a).unwrap();
            }
            BuiltinOperation::Decrement => {
                let a = substitution.mapping[&0];
                let val = graph.get_node_attr(a).unwrap();
                *graph.get_mut_node_attr(a).unwrap() = val - 1;
            }
            BuiltinOperation::Increment => {
                let a = substitution.mapping[&0];
                let val = graph.get_node_attr(a).unwrap();
                *graph.get_mut_node_attr(a).unwrap() = val + 1;
            }
            BuiltinOperation::DeleteNode => {
                let a = substitution.mapping[&0];
                graph.remove_node(a);
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                let fst = graph.get_node_attr(a).unwrap();
                let snd = graph.get_node_attr(b).unwrap();
                *graph.get_mut_node_attr(b).unwrap() = std::cmp::max(*fst, *snd);
            }
        }

        OperationOutput { new_nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
