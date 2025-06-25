use grabapl::graph::operation::BuiltinOperation;
use grabapl::graph::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::graph::operation::parameterbuilder::OperationParameterBuilder;
use grabapl::graph::operation::query::{BuiltinQuery, ConcreteQueryOutput};
use grabapl::graph::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::graph::pattern::{OperationOutput, OperationParameter, ParameterSubstitution};
use grabapl::graph::semantics::{
    AbstractGraph, AbstractJoin, AbstractMatcher, ConcreteGraph, ConcreteToAbstract,
};
use grabapl::{Graph, OperationContext, Semantics};
use std::collections::HashMap;

struct TestSemantics;

struct NodeMatcher;
impl AbstractMatcher for NodeMatcher {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            (_, NodeType::Object) => true,
            _ => argument == parameter,
        }
    }
}

struct EdgeMatcher;
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

struct NodeJoiner;
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

struct EdgeJoiner;
impl AbstractJoin for EdgeJoiner {
    type Abstract = EdgeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        match (a, b) {
            (EdgeType::Exact(a), EdgeType::Exact(b)) if a == b => Some(EdgeType::Exact(a.clone())),
            _ => Some(EdgeType::Wildcard),
        }
    }
}

struct NodeConcreteToAbstract;
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

struct EdgeConcreteToAbstract;
impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = String;
    type Abstract = EdgeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        EdgeType::Exact(c.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum NodeType {
    String,
    Integer,
    /// Top type.
    #[default]
    Object,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NodeValue {
    String(String),
    Integer(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EdgeType {
    Wildcard,
    Exact(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestOperation {
    NoOp,
    SetTo {
        op_typ: NodeType,
        target_typ: NodeType,
        value: NodeValue,
    },
}

impl BuiltinOperation for TestOperation {
    type S = TestSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestOperation::NoOp => {
                param_builder
                    .expect_explicit_input_node(0, NodeType::Object)
                    .unwrap();
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node(0, *op_typ)
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(
        &self,
        g: &mut AbstractGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) -> OperationOutput {
        match self {
            TestOperation::NoOp => {
                // No operation, so no changes to the abstract graph.
                OperationOutput::no_changes()
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the abstract value of the node to the specified type.
                let node_id = substitution.mapping[&0];
                g.set_node_attr(node_id, *target_typ).unwrap();
                OperationOutput {
                    removed_nodes: vec![],
                    new_nodes: HashMap::new(),
                }
            }
        }
    }

    fn apply(
        &self,
        g: &mut ConcreteGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) -> OperationOutput {
        match self {
            TestOperation::NoOp => {
                // No operation, so no changes to the concrete graph.
                OperationOutput::no_changes()
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the concrete value of the node to the specified value.
                let node_id = substitution.mapping[&0];
                g.set_node_attr(node_id, value.clone()).unwrap();
                OperationOutput {
                    removed_nodes: vec![],
                    new_nodes: HashMap::new(),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestQuery {
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
                    .expect_explicit_input_node(0, NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node(1, NodeType::Object)
                    .unwrap();
            }
            TestQuery::ValueEqualTo(_) => {
                param_builder
                    .expect_explicit_input_node(0, NodeType::Object)
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(&self, g: &mut AbstractGraph<Self::S>, substitution: &ParameterSubstitution) {
        // does nothing, not testing side-effect-ful queries here
    }

    fn query(
        &self,
        g: &mut ConcreteGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) -> ConcreteQueryOutput {
        match self {
            TestQuery::ValuesEqual => {
                let node1 = substitution.mapping[&0];
                let node2 = substitution.mapping[&0];
                let value1 = g.get_node_attr(node1).unwrap();
                let value2 = g.get_node_attr(node2).unwrap();
                ConcreteQueryOutput {
                    taken: value1 == value2,
                }
            }
            TestQuery::ValueEqualTo(value) => {
                let node = substitution.mapping[&0];
                let node_value = g.get_node_attr(node).unwrap();
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

#[test]
fn no_modifications_dont_change_abstract_value() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx);

    builder.expect_parameter_node(0, NodeType::Integer).unwrap();
    let a = AbstractNodeId::ParameterMarker(0);
    let state_before = builder.show_state().unwrap();
    builder
        .add_operation(BuilderOpLike::Builtin(TestOperation::NoOp), vec![a])
        .unwrap();
    let state_after = builder.show_state().unwrap();

    let a_type_before = state_before.node_av_of_aid(&a).unwrap();
    let a_type_after = state_after.node_av_of_aid(&a).unwrap();
    assert_eq!(
        a_type_before, a_type_after,
        "Abstract value of node did not remain unchanged after no-op operation"
    );
    assert_eq!(
        a_type_after,
        &NodeType::Integer,
        "Abstract value of node should be Integer after no-op operation"
    );
}

fn get_abstract_value_changing_operation() -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Object).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![p0])
        .unwrap();
    builder.enter_true_branch().unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Object,
                target_typ: NodeType::String,
                value: NodeValue::String("Changed".to_string()),
            }),
            vec![p0],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Object,
                target_typ: NodeType::Integer,
                value: NodeValue::Integer(42),
            }),
            vec![p0],
        )
        .unwrap();
    builder.end_query().unwrap();
    builder.build(0).unwrap()
}

#[test]
fn modifications_change_abstract_value_even_if_same_internal_type() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_abstract_value_changing_operation());
    let mut builder = OperationBuilder::new(&op_ctx);

    builder.expect_parameter_node(0, NodeType::Integer).unwrap();
    let a = AbstractNodeId::ParameterMarker(0);
    let state_before = builder.show_state().unwrap();

    // Add an operation that changes the abstract value
    builder
        .add_operation(BuilderOpLike::FromOperationId(0), vec![a])
        .unwrap();

    let state_after = builder.show_state().unwrap();

    let a_type_before = state_before.node_av_of_aid(&a).unwrap();
    let a_type_after = state_after.node_av_of_aid(&a).unwrap();

    assert_ne!(
        a_type_before, a_type_after,
        "Abstract value of node should change after operation"
    );
    assert_eq!(
        a_type_after,
        &NodeType::Object,
        "Abstract value of node should be Object after operation"
    );
}
