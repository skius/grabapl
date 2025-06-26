pub mod sample_user_defined_operations;

use grabapl::graph::operation::query::{
    AbstractQueryChange, AbstractQueryOutput, BuiltinQuery as BuiltinQueryTrait,
    ConcreteQueryOutput, EdgeChange, NodeChange,
};
use grabapl::graph::operation::run_operation;
use grabapl::graph::pattern::{GraphWithSubstitution, OperationArgument, OperationOutput, OperationParameter, ParameterSubstitution};
use grabapl::graph::semantics::{
    AbstractGraph, AbstractMatcher, AnyMatcher, ConcreteGraph, ConcreteToAbstract, MatchJoiner,
    Semantics,
};
use grabapl::{DotCollector, EdgeInsertionOrder, OperationContext, SubstMarker, WithSubstMarker};
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

pub struct EdgeJoiner;
impl grabapl::graph::semantics::AbstractJoin for EdgeJoiner {
    type Abstract = EdgePattern;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        if EdgeMatcher::matches(a, b) {
            Some(b.clone())
        } else if EdgeMatcher::matches(b, a) {
            Some(a.clone())
        } else {
            Some(EdgePattern::Wildcard) // If they don't match, we return a wildcard edge.
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

    fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>) {
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
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>
    ) -> ConcreteQueryOutput {
        let mut taken = false;
        match self {
            BuiltinQuery::HasChild => {
                todo!(
                    "TODO: how to handle this? we need a notion of the current 'known' graph in order to tell whether there really is a new child or not"
                )
            }
            BuiltinQuery::IsValueGt(val) => {
                if *g.get_node_value(0).unwrap() > *val {
                    taken = true;
                }
            }
            BuiltinQuery::IsValueEq(val) => {
                if *g.get_node_value(0).unwrap() == *val {
                    taken = true;
                }
            }
            BuiltinQuery::ValuesEqual => {
                if g.get_node_value(0) == g.get_node_value(1) {
                    taken = true;
                }
            }
            BuiltinQuery::FirstGtSnd => {
                if g.get_node_value(0) > g.get_node_value(1) {
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
    type NodeJoin = MatchJoiner<Self::NodeMatcher>;
    type EdgeJoin = EdgeJoiner;

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
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> OperationOutput {
        let mut new_nodes = HashMap::new();
        match self {
            BuiltinOperation::AddNode => {
                const NEW_NODE: SubstMarker = 0;
                g.add_node(NEW_NODE, ());
                new_nodes.insert(NEW_NODE, "new".into());
            }
            BuiltinOperation::AppendChild => {
                const PARENT: SubstMarker = 0;
                const CHILD: SubstMarker = 1;
                g.add_node(CHILD, ());
                // TODO: this EdgePattern is weird.
                //  On the one hand, we know for a fact this is an exact "" that will be added, so in type-theory, we correctly add the most precise type (Exact instead of Wildcard)
                //  But if this ever used as a _pattern_ (parameter), it is a *decision* we're making here. Exact will permit fewer matches.
                //  Realistically this is not a problem, because we don't run builtin operations on parameters. But we should be careful.
                g.add_edge(
                    PARENT,
                    CHILD,
                    EdgePattern::Exact("".to_string()),
                );
                new_nodes.insert(CHILD, "child".into());
            }
            BuiltinOperation::IndexCycle => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::SetValue(_) => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::AddEdge => {
                const SRC: SubstMarker = 0;
                const DST: SubstMarker = 1;
                g.add_edge(
                    SRC,
                    DST,
                    EdgePattern::Exact("".to_string()),
                );
            }
            BuiltinOperation::SetEdgeValue(val) => {
                const SRC: SubstMarker = 0;
                const DST: SubstMarker = 1;
                g.set_edge_value(SRC, DST, EdgePattern::Exact(val.clone()));
            }
            BuiltinOperation::SetNodeValue(val) => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::CopyNodeValueTo => {
                const SRC: SubstMarker = 0;
                const DST: SubstMarker = 1;
                // Noop as long as the abstract value is just the unit type...
                let src_value = g.get_node_value(SRC).unwrap();
                g.set_node_value(DST, *src_value);
            }
            BuiltinOperation::Decrement => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::Increment => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::DeleteNode => {
                const NODE_TO_DELETE: SubstMarker = 0;
                g.delete_node(NODE_TO_DELETE);
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
        }
        g.get_concrete_output(new_nodes)
    }

    fn apply(
        &self,
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>
    ) -> OperationOutput {
        let mut new_nodes = HashMap::new();
        match self {
            BuiltinOperation::AddNode => {
                const NEW_NODE: SubstMarker = 0;
                g.add_node(NEW_NODE, 0);
                new_nodes.insert(NEW_NODE, "new".into());
            }
            BuiltinOperation::AppendChild => {
                const PARENT: SubstMarker = 0;
                const CHILD: SubstMarker = 1;
                g.add_node(CHILD, 0);
                g.add_edge(
                    PARENT,
                    CHILD,
                    "".to_string(),
                );
                new_nodes.insert(CHILD, "child".into());
            }
            BuiltinOperation::IndexCycle => {
                const A: SubstMarker = 0;
                const B: SubstMarker = 1;
                const C: SubstMarker = 2;
                g.set_node_value(A, 1);
                g.set_node_value(B, 2);
                g.set_node_value(C, 3);
            }
            BuiltinOperation::SetValue(f) => {
                const A: SubstMarker = 0;
                g.set_node_value(A, f());
            }
            BuiltinOperation::AddEdge => {
                const SRC: SubstMarker = 0;
                const DST: SubstMarker = 1;
                g.add_edge(
                    SRC,
                    DST,
                    "".to_string(),
                );
            }
            BuiltinOperation::SetEdgeValue(val) => {
                const SRC: SubstMarker = 0;
                const DST: SubstMarker = 1;
                g.set_edge_value(SRC, DST, val.clone());

            }
            BuiltinOperation::SetNodeValue(val) => {
                const A: SubstMarker = 0;
                g.set_node_value(A, *val);
            }
            BuiltinOperation::CopyNodeValueTo => {
                const SRC: SubstMarker = 0;
                const DST: SubstMarker = 1;
                let src_value = g.get_node_value(SRC).unwrap();
                g.set_node_value(DST, *src_value);
            }
            BuiltinOperation::Decrement => {
                const A: SubstMarker = 0;
                let val = g.get_node_value(A).unwrap();
                g.set_node_value(A, val - 1);
            }
            BuiltinOperation::Increment => {
                const A: SubstMarker = 0;
                let val = g.get_node_value(A).unwrap();
                g.set_node_value(A, val + 1);
            }
            BuiltinOperation::DeleteNode => {
                const NODE_TO_DELETE: SubstMarker = 0;
                g.delete_node(NODE_TO_DELETE);
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                const FST: SubstMarker = 0;
                const SND: SubstMarker = 1;
                let fst_value = g.get_node_value(FST).unwrap();
                let snd_value = g.get_node_value(SND).unwrap();
                let max_value = std::cmp::max(*fst_value, *snd_value);
                g.set_node_value(SND, max_value);
            }
        }

        g.get_concrete_output(new_nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
