pub mod sample_user_defined_operations;

use grabapl::operation::query::{
    AbstractQueryChange, AbstractQueryOutput, BuiltinQuery as BuiltinQueryTrait,
    ConcreteQueryOutput, EdgeChange, NodeChange,
};
use grabapl::operation::run_operation;
use grabapl::operation::signature::parameter::{
    AbstractOperationOutput, GraphWithSubstitution, NewNodeMarker, OperationArgument,
    OperationOutput, OperationParameter, ParameterSubstitution,
};
use grabapl::operation::signature::parameterbuilder::OperationParameterBuilder;
use grabapl::semantics::{
    AbstractGraph, AbstractMatcher, AnyMatcher, ConcreteGraph, ConcreteToAbstract, MatchJoiner,
    Semantics,
};
use grabapl::{DotCollector, EdgeInsertionOrder, OperationContext, SubstMarker};
use std::collections::HashMap;
use std::convert::Into;
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
impl grabapl::semantics::AbstractJoin for EdgeJoiner {
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

impl BuiltinQuery {
    const HAS_CHILD_INPUT: &'static str = "parent";

    const IS_VALUE_GT_INPUT: &'static str = "node";

    const IS_VALUE_EQ_INPUT: &'static str = "node";

    const VALUES_EQUAL_FIRST: &'static str = "first";
    const VALUES_EQUAL_SECOND: &'static str = "second";

    const FIRST_GT_SND_FIRST: &'static str = "first";
    const FIRST_GT_SND_SECOND: &'static str = "second";
}

impl BuiltinQueryTrait for BuiltinQuery {
    type S = SimpleSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut builder = OperationParameterBuilder::new();
        match self {
            BuiltinQuery::HasChild => {
                builder
                    .expect_explicit_input_node(Self::HAS_CHILD_INPUT, ())
                    .unwrap();
            }
            BuiltinQuery::IsValueGt(_) => {
                builder
                    .expect_explicit_input_node(Self::IS_VALUE_GT_INPUT, ())
                    .unwrap();
            }
            BuiltinQuery::IsValueEq(_) => {
                builder
                    .expect_explicit_input_node(Self::IS_VALUE_EQ_INPUT, ())
                    .unwrap();
            }
            BuiltinQuery::ValuesEqual => {
                builder
                    .expect_explicit_input_node(Self::VALUES_EQUAL_FIRST, ())
                    .unwrap();
                builder
                    .expect_explicit_input_node(Self::VALUES_EQUAL_SECOND, ())
                    .unwrap();
            }
            BuiltinQuery::FirstGtSnd => {
                builder
                    .expect_explicit_input_node(Self::FIRST_GT_SND_FIRST, ())
                    .unwrap();
                builder
                    .expect_explicit_input_node(Self::FIRST_GT_SND_SECOND, ())
                    .unwrap();
            }
        }
        builder.build().unwrap()
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

    fn query(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> ConcreteQueryOutput {
        let mut taken = false;
        match self {
            BuiltinQuery::HasChild => {
                todo!(
                    "TODO: how to handle this? we need a notion of the current 'known' graph in order to tell whether there really is a new child or not"
                )
            }
            BuiltinQuery::IsValueGt(val) => {
                if *g
                    .get_node_value(SubstMarker::from(Self::IS_VALUE_GT_INPUT))
                    .unwrap()
                    > *val
                {
                    taken = true;
                }
            }
            BuiltinQuery::IsValueEq(val) => {
                if *g
                    .get_node_value(SubstMarker::from(Self::IS_VALUE_EQ_INPUT))
                    .unwrap()
                    == *val
                {
                    taken = true;
                }
            }
            BuiltinQuery::ValuesEqual => {
                let first = SubstMarker::from(Self::VALUES_EQUAL_FIRST);
                let second = SubstMarker::from(Self::VALUES_EQUAL_SECOND);
                if g.get_node_value(first) == g.get_node_value(second) {
                    taken = true;
                }
            }
            BuiltinQuery::FirstGtSnd => {
                let first = SubstMarker::from(Self::FIRST_GT_SND_FIRST);
                let second = SubstMarker::from(Self::FIRST_GT_SND_SECOND);
                if g.get_node_value(first) > g.get_node_value(second) {
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

impl BuiltinOperation {
    const APPEND_CHILD_INPUT: &'static str = "parent";
    const INDEX_CYCLE_INPUT_A: &'static str = "a";
    const INDEX_CYCLE_INPUT_B: &'static str = "b";
    const INDEX_CYCLE_INPUT_C: &'static str = "c";
    const SET_VALUE_INPUT: &'static str = "target";
    const ADD_EDGE_INPUT_SRC: &'static str = "src";
    const ADD_EDGE_INPUT_DST: &'static str = "dst";
    const SET_EDGE_VALUE_INPUT_SRC: &'static str = "src";
    const SET_EDGE_VALUE_INPUT_DST: &'static str = "dst";
    const SET_NODE_VALUE_INPUT: &'static str = "target";
    const COPY_NODE_VALUE_TO_INPUT_SRC: &'static str = "src";
    const COPY_NODE_VALUE_TO_INPUT_DST: &'static str = "dst";
    const DECREMENT_INPUT: &'static str = "target";
    const INCREMENT_INPUT: &'static str = "target";
    const DELETE_NODE_INPUT: &'static str = "target";
    const SET_SND_TO_MAX_OF_FST_SND_INPUT_FST: &'static str = "fst";
    const SET_SND_TO_MAX_OF_FST_SND_INPUT_SND: &'static str = "snd";
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

impl grabapl::operation::BuiltinOperation for BuiltinOperation {
    type S = SimpleSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut builder = OperationParameterBuilder::new();
        match self {
            BuiltinOperation::AddNode => {
                // empty graph
            }
            BuiltinOperation::AppendChild => {
                builder
                    .expect_explicit_input_node(Self::APPEND_CHILD_INPUT, ())
                    .unwrap();
            }
            BuiltinOperation::IndexCycle => {
                builder
                    .expect_explicit_input_node(Self::INDEX_CYCLE_INPUT_A, ())
                    .unwrap();
                builder
                    .expect_context_node(Self::INDEX_CYCLE_INPUT_B, ())
                    .unwrap();
                builder
                    .expect_context_node(Self::INDEX_CYCLE_INPUT_C, ())
                    .unwrap();
                builder
                    .expect_edge(
                        Self::INDEX_CYCLE_INPUT_A,
                        Self::INDEX_CYCLE_INPUT_B,
                        EdgePattern::Wildcard,
                    )
                    .unwrap();
                builder
                    .expect_edge(
                        Self::INDEX_CYCLE_INPUT_B,
                        Self::INDEX_CYCLE_INPUT_C,
                        EdgePattern::Wildcard,
                    )
                    .unwrap();
                builder
                    .expect_edge(
                        Self::INDEX_CYCLE_INPUT_C,
                        Self::INDEX_CYCLE_INPUT_A,
                        EdgePattern::Exact("cycle".to_string()),
                    )
                    .unwrap();
            }
            BuiltinOperation::SetValue(_) => {
                builder
                    .expect_explicit_input_node(Self::SET_VALUE_INPUT, ())
                    .unwrap();
            }
            BuiltinOperation::AddEdge => {
                builder
                    .expect_explicit_input_node(Self::ADD_EDGE_INPUT_SRC, ())
                    .unwrap();
                builder
                    .expect_explicit_input_node(Self::ADD_EDGE_INPUT_DST, ())
                    .unwrap();
            }
            BuiltinOperation::SetEdgeValue(_) => {
                builder
                    .expect_explicit_input_node(Self::SET_EDGE_VALUE_INPUT_SRC, ())
                    .unwrap();
                builder
                    .expect_explicit_input_node(Self::SET_EDGE_VALUE_INPUT_DST, ())
                    .unwrap();
                builder
                    .expect_edge(
                        Self::SET_EDGE_VALUE_INPUT_SRC,
                        Self::SET_EDGE_VALUE_INPUT_DST,
                        EdgePattern::Wildcard,
                    )
                    .unwrap();
            }
            BuiltinOperation::SetNodeValue(_) => {
                builder
                    .expect_explicit_input_node(Self::SET_NODE_VALUE_INPUT, ())
                    .unwrap();
            }
            BuiltinOperation::CopyNodeValueTo => {
                builder
                    .expect_explicit_input_node(Self::COPY_NODE_VALUE_TO_INPUT_SRC, ())
                    .unwrap();
                builder
                    .expect_explicit_input_node(Self::COPY_NODE_VALUE_TO_INPUT_DST, ())
                    .unwrap();
            }
            BuiltinOperation::Decrement => {
                builder
                    .expect_explicit_input_node(Self::DECREMENT_INPUT, ())
                    .unwrap();
            }
            BuiltinOperation::Increment => {
                builder
                    .expect_explicit_input_node(Self::INCREMENT_INPUT, ())
                    .unwrap();
            }
            BuiltinOperation::DeleteNode => {
                builder
                    .expect_explicit_input_node(Self::DELETE_NODE_INPUT, ())
                    .unwrap();
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                builder
                    .expect_explicit_input_node(Self::SET_SND_TO_MAX_OF_FST_SND_INPUT_FST, ())
                    .unwrap();
                builder
                    .expect_explicit_input_node(Self::SET_SND_TO_MAX_OF_FST_SND_INPUT_SND, ())
                    .unwrap();
            }
        }
        builder.build().unwrap()
    }

    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> AbstractOperationOutput<Self::S> {
        let mut new_nodes = HashMap::new();
        match self {
            BuiltinOperation::AddNode => {
                const NEW_NODE: &'static str = "new";
                g.add_node(NEW_NODE, ());
                new_nodes.insert(NEW_NODE.into(), "new".into());
            }
            BuiltinOperation::AppendChild => {
                const CHILD: &'static str = "child";
                g.add_node(CHILD, ());
                // TODO: this EdgePattern is weird.
                //  On the one hand, we know for a fact this is an exact "" that will be added, so in type-theory, we correctly add the most precise type (Exact instead of Wildcard)
                //  But if this ever used as a _pattern_ (parameter), it is a *decision* we're making here. Exact will permit fewer matches.
                //  Realistically this is not a problem, because we don't run builtin operations on parameters. But we should be careful.
                g.add_edge(
                    SubstMarker::from(Self::APPEND_CHILD_INPUT),
                    NewNodeMarker::from(CHILD),
                    EdgePattern::Exact("".to_string()),
                );
                new_nodes.insert(CHILD.into(), "child".into());
            }
            BuiltinOperation::IndexCycle => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::SetValue(_) => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::AddEdge => {
                let src = SubstMarker::from(Self::ADD_EDGE_INPUT_SRC);
                let dest = SubstMarker::from(Self::ADD_EDGE_INPUT_DST);
                g.add_edge(src, dest, EdgePattern::Exact("".to_string()));
            }
            BuiltinOperation::SetEdgeValue(val) => {
                let src = SubstMarker::from(Self::SET_EDGE_VALUE_INPUT_SRC);
                let dst = SubstMarker::from(Self::SET_EDGE_VALUE_INPUT_DST);
                g.set_edge_value(src, dst, EdgePattern::Exact(val.clone()));
            }
            BuiltinOperation::SetNodeValue(val) => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::CopyNodeValueTo => {
                let src = SubstMarker::from(Self::COPY_NODE_VALUE_TO_INPUT_SRC);
                let dst = SubstMarker::from(Self::COPY_NODE_VALUE_TO_INPUT_DST);
                // Noop as long as the abstract value is just the unit type...
                let src_value = g.get_node_value(src).unwrap();
                g.set_node_value(dst, *src_value);
            }
            BuiltinOperation::Decrement => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::Increment => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
            BuiltinOperation::DeleteNode => {
                g.delete_node(SubstMarker::from(Self::DELETE_NODE_INPUT));
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
        }
        g.get_abstract_output(new_nodes)
    }

    fn apply(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> OperationOutput {
        let mut new_nodes = HashMap::new();
        match self {
            BuiltinOperation::AddNode => {
                const NEW_NODE: &'static str = "new";
                g.add_node(NEW_NODE, 0);
                new_nodes.insert(NEW_NODE.into(), "new".into());
            }
            BuiltinOperation::AppendChild => {
                const CHILD: &'static str = "child";
                g.add_node(CHILD, 0);
                g.add_edge(
                    SubstMarker::from(Self::APPEND_CHILD_INPUT),
                    NewNodeMarker::from(CHILD),
                    "".to_string(),
                );
                new_nodes.insert(CHILD.into(), "child".into());
            }
            BuiltinOperation::IndexCycle => {
                let a = SubstMarker::from(Self::INDEX_CYCLE_INPUT_A);
                let b = SubstMarker::from(Self::INDEX_CYCLE_INPUT_B);
                let c = SubstMarker::from(Self::INDEX_CYCLE_INPUT_C);
                g.set_node_value(a, 1);
                g.set_node_value(b, 2);
                g.set_node_value(c, 3);
            }
            BuiltinOperation::SetValue(f) => {
                let a = SubstMarker::from(Self::SET_VALUE_INPUT);
                g.set_node_value(a, f());
            }
            BuiltinOperation::AddEdge => {
                let src = SubstMarker::from(Self::ADD_EDGE_INPUT_SRC);
                let dst = SubstMarker::from(Self::ADD_EDGE_INPUT_DST);
                g.add_edge(src, dst, "".to_string());
            }
            BuiltinOperation::SetEdgeValue(val) => {
                let src = SubstMarker::from(Self::SET_EDGE_VALUE_INPUT_SRC);
                let dst = SubstMarker::from(Self::SET_EDGE_VALUE_INPUT_DST);
                g.set_edge_value(src, dst, val.clone());
            }
            BuiltinOperation::SetNodeValue(val) => {
                let a = SubstMarker::from(Self::SET_NODE_VALUE_INPUT);
                g.set_node_value(a, *val);
            }
            BuiltinOperation::CopyNodeValueTo => {
                let src = SubstMarker::from(Self::COPY_NODE_VALUE_TO_INPUT_SRC);
                let dst = SubstMarker::from(Self::COPY_NODE_VALUE_TO_INPUT_DST);
                let src_value = g.get_node_value(src).unwrap();
                g.set_node_value(dst, *src_value);
            }
            BuiltinOperation::Decrement => {
                let a = SubstMarker::from(Self::DECREMENT_INPUT);
                let val = g.get_node_value(a.clone()).unwrap();
                g.set_node_value(a, val - 1);
            }
            BuiltinOperation::Increment => {
                let a = SubstMarker::from(Self::INCREMENT_INPUT);
                let val = g.get_node_value(a.clone()).unwrap();
                g.set_node_value(a, val + 1);
            }
            BuiltinOperation::DeleteNode => {
                let node_to_delete = SubstMarker::from(Self::DELETE_NODE_INPUT);
                g.delete_node(node_to_delete);
            }
            BuiltinOperation::SetSndToMaxOfFstSnd => {
                let fst = SubstMarker::from(Self::SET_SND_TO_MAX_OF_FST_SND_INPUT_FST);
                let snd = SubstMarker::from(Self::SET_SND_TO_MAX_OF_FST_SND_INPUT_SND);
                let fst_value = g.get_node_value(fst).unwrap();
                let snd_value = g.get_node_value(snd.clone()).unwrap();
                let max_value = std::cmp::max(*fst_value, *snd_value);
                g.set_node_value(snd, max_value);
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
