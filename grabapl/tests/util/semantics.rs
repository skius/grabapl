use std::collections::HashMap;
use grabapl::graph::operation::BuiltinOperation;
use grabapl::graph::operation::parameterbuilder::OperationParameterBuilder;
use grabapl::graph::operation::query::{BuiltinQuery, ConcreteQueryOutput};
use grabapl::graph::pattern::{AbstractOperationOutput, GraphWithSubstitution, OperationOutput, OperationParameter};
use grabapl::graph::semantics::{AbstractGraph, AbstractJoin, AbstractMatcher, ConcreteGraph, ConcreteToAbstract};
use grabapl::{Semantics, SubstMarker};

pub struct TestSemantics;

pub struct NodeMatcher;
impl AbstractMatcher for NodeMatcher {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            (_, NodeType::Object) => true,
            _ => argument == parameter,
        }
    }
}

pub struct EdgeMatcher;
impl AbstractMatcher for EdgeMatcher {
    type Abstract = EdgeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            (_, EdgeType::Wildcard) => true,
            (EdgeType::Exact(a), EdgeType::Exact(b)) => a == b,
            _ => false,
        }
    }
}

pub struct NodeJoiner;
impl AbstractJoin for NodeJoiner {
    type Abstract = NodeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        if a == b {
            Some(a.clone())
        } else {
            Some(NodeType::Object)
        }
    }
}

pub struct EdgeJoiner;
impl AbstractJoin for EdgeJoiner {
    type Abstract = EdgeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        match (a, b) {
            (EdgeType::Exact(a), EdgeType::Exact(b)) if a == b => Some(EdgeType::Exact(a.clone())),
            _ => Some(EdgeType::Wildcard),
        }
    }
}

pub struct NodeConcreteToAbstract;
impl ConcreteToAbstract for NodeConcreteToAbstract {
    type Concrete = NodeValue;
    type Abstract = NodeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        match c {
            NodeValue::String(_) => NodeType::String,
            NodeValue::Integer(_) => NodeType::Integer,
        }
    }
}

pub struct EdgeConcreteToAbstract;
impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = String;
    type Abstract = EdgeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        EdgeType::Exact(c.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NodeType {
    String,
    Integer,
    /// Top type.
    #[default]
    Object,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeValue {
    String(String),
    Integer(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeType {
    Wildcard,
    Exact(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestOperation {
    NoOp,
    SetTo {
        op_typ: NodeType,
        target_typ: NodeType,
        value: NodeValue,
    },
    SetEdgeTo {
        node_typ: NodeType,
        param_typ: EdgeType,
        target_typ: EdgeType,
        value: String,
    },
    AddEdge {
        node_typ: NodeType,
        param_typ: EdgeType,
        target_typ: EdgeType,
        value: String,
    },
    AddNode {
        node_type: NodeType,
        value: NodeValue,
    },
    CopyValueFromTo,
    DeleteNode,
    DeleteEdge,
}

impl BuiltinOperation for TestOperation {
    type S = TestSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestOperation::NoOp => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::Object)
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
            TestOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("src", *node_typ)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", *node_typ)
                    .unwrap();
                param_builder
                    .expect_edge(
                        SubstMarker::from("src"),
                        SubstMarker::from("dst"),
                        op_typ.clone(),
                    )
                    .unwrap();
            }
            TestOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
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
                    .expect_explicit_input_node("source", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::Object)
                    .unwrap();
            }
            TestOperation::DeleteNode => {
                param_builder
                    .expect_explicit_input_node("target", NodeType::Object)
                    .unwrap();
            }
            TestOperation::DeleteEdge => {
                param_builder
                    .expect_explicit_input_node("src", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_edge(
                        SubstMarker::from("src"),
                        SubstMarker::from("dst"),
                        EdgeType::Wildcard,
                    )
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
            TestOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Set the edge from source to destination with the specified type.
                g.set_edge_value(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    target_typ.clone(),
                )
                    .unwrap();
            }
            TestOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Add an edge from source to destination with the specified type.
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    target_typ.clone(),
                );
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
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            TestOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
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
            TestOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Set the edge from source to destination with the specified value.
                g.set_edge_value(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    value.clone(),
                )
                    .unwrap();
            }
            TestOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Add an edge from source to destination with the specified value.
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    value.clone(),
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
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            TestOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
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
}

impl BuiltinQuery for TestQuery {
    type S = TestSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestQuery::ValuesEqual => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::Object)
                    .unwrap();
            }
            TestQuery::ValueEqualTo(_) => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
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
        }
    }
}

impl Semantics for TestSemantics {
    type NodeConcrete = NodeValue;
    type NodeAbstract = NodeType;
    type EdgeConcrete = String;
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