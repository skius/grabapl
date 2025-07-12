use grabapl::operation::BuiltinOperation;
use grabapl::operation::signature::parameterbuilder::OperationParameterBuilder;
use grabapl::operation::query::{BuiltinQuery, ConcreteQueryOutput};
use grabapl::operation::signature::parameter::{
    AbstractOperationOutput, GraphWithSubstitution, OperationOutput, OperationParameter,
};
use grabapl::semantics::{
    AbstractGraph, AbstractJoin, AbstractMatcher, ConcreteGraph, ConcreteToAbstract,
};
use grabapl::{Semantics, SubstMarker};
use std::collections::HashMap;

pub mod interval {
    /// Inclusive i32 interval. Empty interval is represented by start > end.
    #[derive(Clone, Copy, derive_more::Debug, PartialEq, Eq)]
    #[debug("[{start}, {end}]")]
    pub struct Interval {
        pub start: i32,
        pub end: i32,
    }

    impl Interval {
        pub fn any() -> Self {
            Interval {
                start: i32::MIN,
                end: i32::MAX,
            }
        }

        pub fn new_singleton(value: i32) -> Self {
            Interval {
                start: value,
                end: value,
            }
        }

        pub fn new(start: i32, end: i32) -> Self {
            Interval { start, end }
        }

        pub fn union(&self, other: &Self) -> Self {
            Interval {
                start: self.start.min(other.start),
                end: self.end.max(other.end),
            }
        }

        pub fn intersection(&self, other: &Self) -> Self {
            Interval {
                start: self.start.max(other.start),
                end: self.end.min(other.end),
            }
        }

        pub fn contains(&self, value: i32) -> bool {
            self.start <= value && value <= self.end
        }

        pub fn contains_interval(&self, other: &Self) -> bool {
            self.start <= other.start && self.end >= other.end
        }

        pub fn is_empty(&self) -> bool {
            self.start > self.end
        }
    }
}

pub struct IntervalSemantics;

pub struct NodeMatcher;
impl AbstractMatcher for NodeMatcher {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        parameter.0.contains_interval(&argument.0)
    }
}

pub struct EdgeMatcher;
impl AbstractMatcher for EdgeMatcher {
    type Abstract = EdgeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        true
    }
}

pub struct NodeJoiner;
impl AbstractJoin for NodeJoiner {
    type Abstract = NodeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        Some(NodeType(a.0.union(&b.0)))
    }
}

pub struct EdgeJoiner;
impl AbstractJoin for EdgeJoiner {
    type Abstract = EdgeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        Some(EdgeType)
    }
}

pub struct NodeConcreteToAbstract;
impl ConcreteToAbstract for NodeConcreteToAbstract {
    type Concrete = NodeValue;
    type Abstract = NodeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        NodeType(interval::Interval::new_singleton(c.0))
    }
}

pub struct EdgeConcreteToAbstract;
impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = EdgeValue;
    type Abstract = EdgeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        EdgeType
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeType(pub interval::Interval);

impl NodeType {
    pub fn any() -> Self {
        NodeType(interval::Interval::any())
    }

    pub fn new_singleton(value: i32) -> Self {
        NodeType(interval::Interval::new_singleton(value))
    }

    pub fn new(start: i32, end: i32) -> Self {
        NodeType(interval::Interval::new(start, end))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeValue(pub i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EdgeType;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EdgeValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestOperation {
    NoOp,
    SetTo {
        op_typ: NodeType,
        target_typ: NodeType,
        value: NodeValue,
    },
    AddEdge {
        node_typ: NodeType,
    },
    AddNode {
        node_type: NodeType,
        value: NodeValue,
    },
    CopyValueFromTo,
    SwapValues,
    DeleteNode,
    DeleteEdge,
    AddInteger(i32),
}

impl BuiltinOperation for TestOperation {
    type S = IntervalSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestOperation::NoOp => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::any())
                    .unwrap();
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("target", *op_typ)
                    .unwrap();
            }
            TestOperation::AddEdge { node_typ } => {
                param_builder
                    .expect_explicit_input_node("src", *node_typ)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", *node_typ)
                    .unwrap();
            }
            TestOperation::AddNode { node_type, value } => {}
            TestOperation::CopyValueFromTo => {
                param_builder
                    .expect_explicit_input_node("source", NodeType::any())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::any())
                    .unwrap();
            }
            TestOperation::SwapValues => {
                param_builder
                    .expect_explicit_input_node("source", NodeType::any())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::any())
                    .unwrap();
            }
            TestOperation::DeleteNode => {
                param_builder
                    .expect_explicit_input_node("target", NodeType::any())
                    .unwrap();
            }
            TestOperation::DeleteEdge => {
                param_builder
                    .expect_explicit_input_node("src", NodeType::any())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", NodeType::any())
                    .unwrap();
                param_builder
                    .expect_edge(SubstMarker::from("src"), SubstMarker::from("dst"), EdgeType)
                    .unwrap();
            }
            TestOperation::AddInteger(i) => {
                // TODO: expect a max of i32::MAX - 1 here? due to overflow
                param_builder
                    .expect_explicit_input_node("target", NodeType::any())
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> AbstractOperationOutput<Self::S> {
        let mut new_node_names = HashMap::new();
        match self {
            TestOperation::NoOp => {
                // No operation, so no changes to the abstract graph.
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the abstract value of the node to the specified type.
                g.set_node_value(SubstMarker::from("target"), *target_typ)
                    .unwrap();
            }
            TestOperation::AddEdge { node_typ } => {
                // Add an edge from source to destination with the specified type.
                g.add_edge(SubstMarker::from("src"), SubstMarker::from("dst"), EdgeType);
            }
            TestOperation::AddNode { node_type, value } => {
                // Add a new node with the specified type and value.
                g.add_node("new", node_type.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            TestOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(SubstMarker::from("source")).unwrap();
                g.set_node_value(SubstMarker::from("destination"), value.clone())
                    .unwrap();
            }
            TestOperation::SwapValues => {
                // Swap the values of two nodes.
                let value1 = g.get_node_value(SubstMarker::from("source")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("destination")).unwrap();
                let v1 = value1.clone();
                let v2 = value2.clone();
                g.set_node_value(SubstMarker::from("source"), v2).unwrap();
                g.set_node_value(SubstMarker::from("destination"), v1)
                    .unwrap();
                // TODO: talk about above problem in user defined op.
                //  Specifically: We take two objects as input, and swap them. Nothing guarantees that the most precise
                //  types of the two nodes are the same, hence the valid inferred signature (if this were an user defined op) would be
                //  that both nodes end up as type Object. However, because this is builtin, it can actually in a sense
                //  "look at" the real, most precise values of the nodes, and just say that it swaps those.
                //  If we didn't have this, we'd need monomorphized swapvaluesInt etc operations, or support for generics.
            }
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            TestOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
                    .unwrap();
            }
            TestOperation::AddInteger(i) => {
                // Add an integer to the node.
                let &NodeType(old_interval) =
                    g.get_node_value(SubstMarker::from("target")).unwrap();
                let new_type = NodeType(interval::Interval::new(
                    old_interval.start + *i,
                    old_interval.end + *i,
                ));
                g.set_node_value(SubstMarker::from("target"), new_type)
                    .unwrap();
            }
        }
        g.get_abstract_output(new_node_names)
    }

    fn apply(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> OperationOutput {
        let mut new_node_names = HashMap::new();
        match self {
            TestOperation::NoOp => {
                // No operation, so no changes to the concrete graph.
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the concrete value of the node to the specified value.
                g.set_node_value(SubstMarker::from("target"), value.clone())
                    .unwrap();
            }
            TestOperation::AddEdge { node_typ } => {
                // Add an edge from source to destination with the specified value.
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    EdgeValue,
                );
            }
            TestOperation::AddNode { node_type, value } => {
                // Add a new node with the specified type and value.
                g.add_node("new", value.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            TestOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(SubstMarker::from("source")).unwrap();
                g.set_node_value(SubstMarker::from("destination"), value.clone())
                    .unwrap();
            }
            TestOperation::SwapValues => {
                // Swap the values of two nodes.
                let value1 = g.get_node_value(SubstMarker::from("source")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("destination")).unwrap();
                let v1 = value1.clone();
                let v2 = value2.clone();
                g.set_node_value(SubstMarker::from("source"), v2).unwrap();
                g.set_node_value(SubstMarker::from("destination"), v1)
                    .unwrap();
            }
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            TestOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
                    .unwrap();
            }
            TestOperation::AddInteger(i) => {
                // Add an integer to the node.
                let &NodeValue(old_value) = g.get_node_value(SubstMarker::from("target")).unwrap();
                let new_value = NodeValue(*i + old_value);
                g.set_node_value(SubstMarker::from("target"), new_value)
                    .unwrap();
            }
        }
        g.get_concrete_output(new_node_names)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestQuery {
    ValuesEqual,
    ValueEqualTo(NodeValue),
    CmpFstSnd(std::cmp::Ordering),
}

impl BuiltinQuery for TestQuery {
    type S = IntervalSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestQuery::ValuesEqual => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::any())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::any())
                    .unwrap();
            }
            TestQuery::ValueEqualTo(_) => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::any())
                    .unwrap();
            }
            TestQuery::CmpFstSnd(_) => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::any())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::any())
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>) {
        // does nothing, not testing side-effect-ful queries here
    }

    fn query(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> ConcreteQueryOutput {
        match self {
            TestQuery::ValuesEqual => {
                let value1 = g.get_node_value(SubstMarker::from("a")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("b")).unwrap();
                ConcreteQueryOutput {
                    taken: value1 == value2,
                }
            }
            TestQuery::ValueEqualTo(value) => {
                let node_value = g.get_node_value(SubstMarker::from("a")).unwrap();
                ConcreteQueryOutput {
                    taken: node_value == value,
                }
            }
            TestQuery::CmpFstSnd(ordering) => {
                let value1 = g.get_node_value(SubstMarker::from("a")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("b")).unwrap();
                let cmp_result = match (value1, value2) {
                    (NodeValue(a), NodeValue(b)) => a.cmp(&b),
                };
                ConcreteQueryOutput {
                    taken: cmp_result == *ordering,
                }
            }
        }
    }
}

impl Semantics for IntervalSemantics {
    type NodeConcrete = NodeValue;
    type NodeAbstract = NodeType;
    type EdgeConcrete = EdgeValue;
    type EdgeAbstract = EdgeType;
    type NodeMatcher = NodeMatcher;
    type EdgeMatcher = EdgeMatcher;
    type NodeJoin = NodeJoiner;
    type EdgeJoin = EdgeJoiner;
    type NodeConcreteToAbstract = NodeConcreteToAbstract;
    type EdgeConcreteToAbstract = EdgeConcreteToAbstract;
    type BuiltinOperation = TestOperation;
    type BuiltinQuery = TestQuery;
}
