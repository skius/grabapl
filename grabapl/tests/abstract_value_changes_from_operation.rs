use grabapl::graph::operation::{run_from_concrete, BuiltinOperation};
use grabapl::graph::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::graph::operation::parameterbuilder::OperationParameterBuilder;
use grabapl::graph::operation::query::{BuiltinQuery, ConcreteQueryOutput};
use grabapl::graph::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::graph::pattern::{GraphWithSubstitution, OperationOutput, OperationParameter, ParameterSubstitution};
use grabapl::graph::semantics::{
    AbstractGraph, AbstractJoin, AbstractMatcher, ConcreteGraph, ConcreteToAbstract,
};
use grabapl::{Graph, OperationContext, Semantics};
use std::collections::{HashMap, HashSet};

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
    AddNode {
        node_type: NodeType,
        value: NodeValue,
    },
    CopyValueFromTo,
    DeleteNode,
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
            TestOperation::AddNode {
                node_type,
                value,
            } => {}
            TestOperation::CopyValueFromTo => {
                param_builder
                    .expect_explicit_input_node(0, NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node(1, NodeType::Object)
                    .unwrap();
            }
            TestOperation::DeleteNode => {
                param_builder
                    .expect_explicit_input_node(0, NodeType::Object)
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> OperationOutput {
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
                g.set_node_value(0, *target_typ).unwrap();
            }
            TestOperation::AddNode {
                node_type,
                value,
            } => {
                // Add a new node with the specified type and value.
                g.add_node(0, node_type.clone());
                new_node_names.insert(0, "new".into());
            }
            TestOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(0).unwrap();
                g.set_node_value(1, value.clone()).unwrap();
            }
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(0).unwrap();
            }
        }
        g.get_concrete_output(new_node_names)
    }

    fn apply(
        &self,
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>,
    ) -> OperationOutput {
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
                g.set_node_value(0, value.clone()).unwrap();
            }
            TestOperation::AddNode {
                node_type,
                value,
            } => {
                // Add a new node with the specified type and value.
                g.add_node(0, value.clone());
                new_node_names.insert(0, "new".into());
            }
            TestOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(0).unwrap();
                g.set_node_value(1, value.clone()).unwrap();
            }
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(0).unwrap();
            }
        }
        g.get_concrete_output(new_node_names)
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

    fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>) {
        // does nothing, not testing side-effect-ful queries here
    }

    fn query(
        &self,
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>,
    ) -> ConcreteQueryOutput {
        match self {
            TestQuery::ValuesEqual => {
                let value1 = g.get_node_value(0).unwrap();
                let value2 = g.get_node_value(1).unwrap();
                ConcreteQueryOutput {
                    taken: value1 == value2,
                }
            }
            TestQuery::ValueEqualTo(value) => {
                let node_value = g.get_node_value(0).unwrap();
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

fn get_abstract_value_changing_operation_no_branches() -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Object).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);
    // Add an operation that changes the abstract value
    builder
        .add_operation(BuilderOpLike::Builtin(TestOperation::SetTo {
            op_typ: NodeType::Object,
            // we *set* to the same type, which is not the same as a noop.
            target_typ: NodeType::Object,
            value: NodeValue::String("Changed".to_string()),
        }), vec![p0])
        .unwrap();
    builder.build(0).unwrap()
}

#[test]
fn modifications_change_abstract_value_even_if_same_internal_type_for_custom() {
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


#[test]
fn modifications_change_abstract_value_even_if_same_internal_type_for_builtin() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx);

    builder.expect_parameter_node(0, NodeType::Integer).unwrap();
    let a = AbstractNodeId::ParameterMarker(0);
    let state_before = builder.show_state().unwrap();

    // Add an operation that changes the abstract value
    builder
        .add_operation(BuilderOpLike::Builtin(TestOperation::SetTo {
            op_typ: NodeType::Object,
            // we *set* to the same type, which is not the same as a noop.
            target_typ: NodeType::Object,
            value: NodeValue::String("Changed".to_string()),
        }), vec![a])
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

#[test]
fn modifications_change_abstract_value_even_if_same_internal_type_for_custom_with_builtin() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_abstract_value_changing_operation_no_branches());
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

fn get_custom_op_new_node_in_regular_query_branches() -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Object).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);

    // Start a query that will create a new node in both branches
    builder.start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![p0]).unwrap();

    // True branch
    builder.enter_true_branch().unwrap();
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::String,
                value: NodeValue::String("x".to_string()),
            }),
            vec![],
        )
        .unwrap();

    // False branch
    builder.enter_false_branch().unwrap();
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::Integer,
                value: NodeValue::Integer(42),
            }),
            vec![],
        )
        .unwrap();

    builder.end_query().unwrap();

    // TODO: define the new node to be visible in the output
    let output_aid = AbstractNodeId::DynamicOutputMarker("new".into(), "new".into());
    builder.return_node(output_aid, "output".into(), NodeType::Object).unwrap();

    builder.build(0).unwrap()
}

fn get_custom_op_new_node_in_shape_query_branches() -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Object).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);

    // Start a query that will create a new node in both branches
    builder.start_shape_query("new".into()).unwrap();
    builder.expect_shape_node("new".into(), NodeType::String).unwrap();

    // True branch
    builder.enter_true_branch().unwrap();
    // TODO: rename

    // False branch
    builder.enter_false_branch().unwrap();
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::Integer,
                value: NodeValue::Integer(42),
            }),
            vec![],
        )
        .unwrap();

    builder.end_query().unwrap();

    // TODO: define the new node to be visible in the output
    //  or try to? I guess the builder should ensure that's not the case and fail
    let output_aid = AbstractNodeId::DynamicOutputMarker("new".into(), "new".into());
    builder.return_node(output_aid, "output".into(), NodeType::Object).unwrap();

    builder.build(0).unwrap()
}

#[test]
fn new_node_from_both_branches_is_visible_for_regular_query() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_custom_op_new_node_in_regular_query_branches());
    let mut builder = OperationBuilder::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Integer).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);
    let state_before = builder.show_state().unwrap();
    // Add an operation that creates a new node in both branches
    builder
        .add_named_operation("helper".into(), BuilderOpLike::FromOperationId(0), vec![p0])
        .unwrap();
    let state_after = builder.show_state().unwrap();
    let num_before = state_before.graph.nodes().count();
    let num_after = state_after.graph.nodes().count();
    assert_eq!(
        num_after,
        num_before + 1,
        "Expected a new node to be visible"
    );

    let returned_node = AbstractNodeId::DynamicOutputMarker("helper".into(), "output".into());

    // test that I can actually use the returned node
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::CopyValueFromTo), vec![returned_node, p0]).unwrap();
    let operation = builder.build(1).unwrap();
    op_ctx.add_custom_operation(1, operation);

    let mut concrete_graph = ConcreteGraph::<TestSemantics>::new();
    let p0_key = concrete_graph.add_node(NodeValue::Integer(0));
    run_from_concrete(&mut concrete_graph, &op_ctx, 1, vec![p0_key]).unwrap();
    let new_node_value = concrete_graph.get_node_attr(p0_key).unwrap();
    assert_eq!(
        new_node_value,
        &NodeValue::String("x".to_string()),
        "Expected the new node to have the value 'x'"
    );
}

#[test]
fn new_node_from_both_branches_is_invisible_for_shape_query() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_custom_op_new_node_in_shape_query_branches());
    let mut builder = OperationBuilder::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Integer).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);
    let state_before = builder.show_state().unwrap();
    // Add an operation that creates a new node in both branches
    builder
        .add_named_operation("helper".into(), BuilderOpLike::FromOperationId(0), vec![p0])
        .unwrap();
    let state_after = builder.show_state().unwrap();
    let num_before = state_before.graph.nodes().count();
    let num_after = state_after.graph.nodes().count();
    assert_eq!(
        num_after,
        num_before,
        "Expected no new nodes to be visible"
    );
}

#[test]
fn return_node_partially_from_shape_query_fails() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let helper_op = {
        let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx);
        builder.expect_parameter_node(0, NodeType::Integer).unwrap();
        let p0 = AbstractNodeId::ParameterMarker(0);
        // Start a shape query to check if p0 has a child with edge 'child'
        builder.start_shape_query("child".into()).unwrap();
        builder.expect_shape_node("new".into(), NodeType::String).unwrap();
        let child_aid = AbstractNodeId::DynamicOutputMarker("child".into(), "new".into());
        builder.expect_shape_edge(p0, child_aid, EdgeType::Exact("child".to_string())).unwrap();
        builder.enter_false_branch().unwrap();
        // if we don't have a child node, create one
        builder
            .add_named_operation(
                "child".into(),
                BuilderOpLike::Builtin(TestOperation::AddNode {
                    node_type: NodeType::String,
                    value: NodeValue::String("x".to_string()),
                }),
                vec![],
            )
            .unwrap();
        builder.end_query().unwrap();

        // Return the child node
        let res = builder.return_node(child_aid, "child".into(), NodeType::String);
        assert!(res.is_err(), "Expected returning a node partially originating from a shape query to fail");
        builder.build(0).unwrap()
    };
    op_ctx.add_custom_operation(0, helper_op);

    // now see what happens if we try to run this in a builder
    let mut builder = OperationBuilder::new(&op_ctx);
    builder.expect_parameter_node(0, NodeType::Integer).unwrap();
    let p0 = AbstractNodeId::ParameterMarker(0);
    builder.expect_context_node(1, NodeType::String).unwrap();
    let c0 = AbstractNodeId::ParameterMarker(1);
    builder.expect_parameter_edge(0, 1, EdgeType::Exact("child".to_string())).unwrap();
    let state_before = builder.show_state().unwrap();
    builder.add_named_operation("helper".into(), BuilderOpLike::FromOperationId(0), vec![p0]).unwrap();
    let state_after = builder.show_state().unwrap();
    let aids_before = state_before.node_keys_to_aid.right_values().collect::<HashSet<_>>();
    let aids_after = state_after.node_keys_to_aid.right_values().collect::<HashSet<_>>();
    assert_eq!(
        aids_before, aids_after,
        "Expected no new nodes to be created in the graph"
    );

    // for fun, see what happens when we delete the returned node and then try to use c0
    let returned_node = AbstractNodeId::DynamicOutputMarker("helper".into(), "child".into());
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::DeleteNode), vec![returned_node]).unwrap();
    // now use c0 to copy from c0 to p0
    // note: this is the operation that would crash (the concrete graph would not have the node) if we were allowed to return the node.
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::CopyValueFromTo), vec![c0, p0]).unwrap();
    let operation = builder.build(1).unwrap();
    op_ctx.add_custom_operation(1, operation);

    let mut concrete_graph = ConcreteGraph::<TestSemantics>::new();
    let p0_key = concrete_graph.add_node(NodeValue::Integer(0));
    let c0_key = concrete_graph.add_node(NodeValue::String("context".to_string()));
    concrete_graph.add_edge(p0_key, c0_key, "child".to_string());

    run_from_concrete(&mut concrete_graph, &op_ctx, 1, vec![p0_key]).unwrap();
}