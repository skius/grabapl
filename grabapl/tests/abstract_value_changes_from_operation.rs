use grabapl::operation::builder::{BuilderOpLike, OperationBuilder, stack_based_builder};
use grabapl::operation::builtin::LibBuiltinOperation;
use grabapl::operation::run_from_concrete;
use grabapl::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::prelude::*;
use grabapl::semantics::ConcreteGraph;
use std::collections::{HashMap, HashSet};

mod util;
use util::semantics::*;

#[test]
fn no_modifications_dont_change_abstract_value() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);

    builder
        .expect_parameter_node("a", NodeType::Integer)
        .unwrap();
    let a = AbstractNodeId::ParameterMarker("a".into());
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
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::ParameterMarker("p0".into());
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
    builder.build().unwrap()
}

fn get_abstract_value_changing_operation_no_branches() -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // Add an operation that changes the abstract value
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Object,
                // we *set* to the same type, which is not the same as a noop.
                target_typ: NodeType::Object,
                value: NodeValue::String("Changed".to_string()),
            }),
            vec![p0],
        )
        .unwrap();
    builder.build().unwrap()
}

#[test]
fn modifications_change_abstract_value_even_if_same_internal_type_for_custom() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_abstract_value_changing_operation());
    let mut builder = OperationBuilder::new(&op_ctx, 1);

    builder
        .expect_parameter_node("a", NodeType::Integer)
        .unwrap();
    let a = AbstractNodeId::param("a");
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
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);

    builder
        .expect_parameter_node("a", NodeType::Integer)
        .unwrap();
    let a = AbstractNodeId::param("a");
    let state_before = builder.show_state().unwrap();

    // Add an operation that changes the abstract value
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Object,
                // we *set* to the same type, which is not the same as a noop.
                target_typ: NodeType::Object,
                value: NodeValue::String("Changed".to_string()),
            }),
            vec![a],
        )
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
    let mut builder = OperationBuilder::new(&op_ctx, 1);

    builder
        .expect_parameter_node("a", NodeType::Integer)
        .unwrap();
    let a = AbstractNodeId::param("a");
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
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");

    // Start a query that will create a new node in both branches
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![p0])
        .unwrap();

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

    let output_aid = AbstractNodeId::DynamicOutputMarker("new".into(), "new".into());
    builder
        .return_node(output_aid, "output".into(), NodeType::Object)
        .unwrap();

    builder.build().unwrap()
}

fn get_custom_op_new_node_in_shape_query_branches() -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let _p0 = AbstractNodeId::param("p0");

    // Start a query that will create a new node in both branches
    builder.start_shape_query("new").unwrap();
    builder
        .expect_shape_node("new".into(), NodeType::String)
        .unwrap();
    // TODO: we will need an edge here.

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

    let output_aid = AbstractNodeId::DynamicOutputMarker("new".into(), "new".into());
    let res = builder.return_node(output_aid, "output".into(), NodeType::Object);
    // UPDATE: we're allowed to return shape query nodes now
    // assert!(
    //     res.is_err(),
    //     "`output_aid` partially originates from a shape query, hence it may not be returned"
    // );
    assert!(res.is_ok());

    builder.build().unwrap()
}

#[test]
fn new_node_from_both_branches_is_visible_for_regular_query() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_custom_op_new_node_in_regular_query_branches());
    let mut builder = OperationBuilder::new(&op_ctx, 1);
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
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
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![returned_node, p0],
        )
        .unwrap();
    let operation = builder.build().unwrap();
    op_ctx.add_custom_operation(1, operation);

    let mut concrete_graph = ConcreteGraph::<TestSemantics>::new();
    let p0_key = concrete_graph.add_node(NodeValue::Integer(0));
    run_from_concrete(&mut concrete_graph, &op_ctx, 1, &[p0_key]).unwrap();
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
    let mut builder = OperationBuilder::new(&op_ctx, 1);
    let input_marker = SubstMarker::from("input");
    builder
        .expect_parameter_node(input_marker, NodeType::Integer)
        .unwrap();
    let input_aid = AbstractNodeId::ParameterMarker(input_marker);
    let state_before = builder.show_state().unwrap();
    // Add an operation that creates a new node in both branches
    builder
        .add_named_operation(
            "helper".into(),
            BuilderOpLike::FromOperationId(0),
            vec![input_aid],
        )
        .unwrap();
    let state_after = builder.show_state().unwrap();
    let num_before = state_before.graph.nodes().count();
    let num_after = state_after.graph.nodes().count();
    // UPDATE: we're allowed to return shape query nodes now, so the number of nodes should change by one
    // assert_eq!(num_after, num_before, "Expected no new nodes to be visible");
    assert_eq!(
        num_after,
        num_before + 1,
        "Expected a new node to be visible"
    );
}

// TODO: Add test for the "additive changes in the shape query true branch" header from problems-testcases.md
//  i.e., _new_ nodes and _new_ edges, not just matched, pre-existing nodes in the true branch.

#[test]
fn return_node_partially_from_shape_query_fails() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let helper_op = {
        let mut builder = OperationBuilder::<TestSemantics>::new(&op_ctx, 0);
        builder
            .expect_parameter_node("p0", NodeType::Integer)
            .unwrap();
        let p0 = AbstractNodeId::param("p0");
        // Start a shape query to check if p0 has a child with edge 'child'
        builder.start_shape_query("child").unwrap();
        builder
            .expect_shape_node("new".into(), NodeType::String)
            .unwrap();
        let child_aid = AbstractNodeId::dynamic_output("child", "new");
        builder
            .expect_shape_edge(p0, child_aid, EdgeType::Exact("child".to_string()))
            .unwrap();
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
        // UPDATE: returning a node from shape queries is now allowed
        // assert!(
        //     res.is_err(),
        //     "Expected returning a node partially originating from a shape query to fail"
        // );
        assert!(res.is_ok());

        builder.build().unwrap()
    };
    op_ctx.add_custom_operation(0, helper_op);

    // now see what happens if we try to run this in a builder
    let mut builder = OperationBuilder::new(&op_ctx, 1);
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    builder.expect_context_node("c0", NodeType::String).unwrap();
    let c0 = AbstractNodeId::param("c0");
    builder
        .expect_parameter_edge("p0", "c0", EdgeType::Exact("child".to_string()))
        .unwrap();
    let state_before = builder.show_state().unwrap();
    builder
        .add_named_operation("helper".into(), BuilderOpLike::FromOperationId(0), vec![p0])
        .unwrap();
    let state_after = builder.show_state().unwrap();
    let aids_before = state_before
        .node_keys_to_aid
        .right_values()
        .collect::<HashSet<_>>();
    let aids_after = state_after
        .node_keys_to_aid
        .right_values()
        .collect::<HashSet<_>>();
    // UPDATE: we're allowed to return shape query nodes now, so the number of nodes should change by one
    // assert_eq!(
    //     aids_before, aids_after,
    //     "Expected no new nodes to be created in the graph"
    // );
    assert_eq!(
        aids_after.len(),
        aids_before.len() + 1,
        "Expected a new node to be created in the graph"
    );

    if true {
        // NOTE: this only exhibits the desired crash if the problem this test is checking against is not fixed.
        // UPDATE: indeed ^ is fixed, since the shape query will not match against a 'visible' node. So the problem is fixed, and we can run this without crashes

        // for fun, see what happens when we delete the returned node and then try to use c0
        let returned_node = AbstractNodeId::DynamicOutputMarker("helper".into(), "child".into());
        builder
            .add_operation(
                BuilderOpLike::Builtin(TestOperation::DeleteNode),
                vec![returned_node],
            )
            .unwrap();
        // now use c0 to copy from c0 to p0
        // note: this is the operation that would crash (the concrete graph would not have the node) if we were allowed to return the node.
        builder
            .add_operation(
                BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
                vec![c0, p0],
            )
            .unwrap();
        let operation = builder.build().unwrap();
        op_ctx.add_custom_operation(1, operation);

        let mut concrete_graph = ConcreteGraph::<TestSemantics>::new();
        let p0_key = concrete_graph.add_node(NodeValue::Integer(0));
        let c0_key = concrete_graph.add_node(NodeValue::String("context".to_string()));
        concrete_graph.add_edge(p0_key, c0_key, "child".to_string());

        // crash, CopyValueFromTo doesn't find substmarker 0.
        run_from_concrete(&mut concrete_graph, &op_ctx, 1, &[p0_key]).unwrap();
    }
}

// Test that the full matrix of: [node types, edge types] x [set, delete, new] works as expected.
// In particular, set and delete should propagate information about the new type into the caller operation's signature.

#[test]
fn builder_infers_correct_signatures() {
    let param_instructions = |builder: &mut OperationBuilder<TestSemantics>| {
        builder
            .expect_parameter_node("p0", NodeType::Integer)
            .unwrap();
        builder
            .expect_parameter_node("p1", NodeType::Integer)
            .unwrap();
        builder
            .expect_parameter_node("p2", NodeType::Integer)
            .unwrap();
        builder.expect_context_node("c0", NodeType::Object).unwrap();
        builder.expect_context_node("c1", NodeType::Object).unwrap();
        builder
            .expect_parameter_edge("p0", "c0", EdgeType::Wildcard)
            .unwrap();
        builder
            .expect_parameter_edge("p2", "c1", EdgeType::Wildcard)
            .unwrap();
        builder
            .expect_parameter_edge("p0", "c1", EdgeType::Wildcard)
            .unwrap();
    };

    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    param_instructions(&mut builder);
    // param: p0->c0, p1, p2->c1, p0->c1
    // delete p1, delete c0 (which implies deletion of edge p0->c0), set p0, delete edge p2->c1, set c1, set p0->c1
    // and create new node n0 to return, and new edge p0->c1 to return.
    // new node n1 to not return, and new edge p0->n1 to not return.

    let p0 = AbstractNodeId::ParameterMarker("p0".into());
    let p1 = AbstractNodeId::ParameterMarker("p1".into());
    let p2 = AbstractNodeId::ParameterMarker("p2".into());
    let c0 = AbstractNodeId::ParameterMarker("c0".into());
    let c1 = AbstractNodeId::ParameterMarker("c1".into());
    let n0 = AbstractNodeId::DynamicOutputMarker("new".into(), "new".into());
    let n1 = AbstractNodeId::DynamicOutputMarker("new1".into(), "new".into());

    // delete p1
    builder
        .add_operation(BuilderOpLike::Builtin(TestOperation::DeleteNode), vec![p1])
        .unwrap();
    // delete c0
    builder
        .add_operation(BuilderOpLike::Builtin(TestOperation::DeleteNode), vec![c0])
        .unwrap();
    // set p0 to Integer (i.e., no change - this must still be visible!)
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Object,
                target_typ: NodeType::Integer,
                value: NodeValue::Integer(0),
            }),
            vec![p0],
        )
        .unwrap();
    // delete edge p2->c1
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::DeleteEdge),
            vec![p2, c1],
        )
        .unwrap();
    // set c1 to String (i.e., subtype of Object - this must still be visible!)
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Object,
                target_typ: NodeType::String,
                value: NodeValue::String("context".to_string()),
            }),
            vec![c1],
        )
        .unwrap();
    // set edge p0->c1 to 'p0->c1' (i.e., subtype of Wildcard)
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetEdgeTo {
                node_typ: NodeType::Object,
                param_typ: EdgeType::Wildcard,
                target_typ: EdgeType::Exact("p0->c1".to_string()),
                value: "p0->c1".to_string(),
            }),
            vec![p0, c1],
        )
        .unwrap();
    // create new node n0
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::String,
                value: NodeValue::String("new".to_string()),
            }),
            vec![],
        )
        .unwrap();
    // overwrite edge value p0->c1
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddEdge {
                node_typ: NodeType::Object,
                param_typ: EdgeType::Wildcard,
                target_typ: EdgeType::Exact("new_edge".to_string()),
                value: "new_edge".to_string(),
            }),
            vec![p0, c1],
        )
        .unwrap();
    // create new non-returned node n1
    builder
        .add_named_operation(
            "new1".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::Integer,
                value: NodeValue::Integer(42),
            }),
            vec![],
        )
        .unwrap();
    // create new non-returned edge p0->n1
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddEdge {
                node_typ: NodeType::Object,
                param_typ: EdgeType::Wildcard,
                target_typ: EdgeType::Exact("new_edge1".to_string()),
                value: "new_edge1".to_string(),
            }),
            vec![p0, n1],
        )
        .unwrap();
    // return n0
    builder
        .return_node(n0, "new".into(), NodeType::String)
        .unwrap();
    // return p0->c1 edge
    builder
        .return_edge(p0, c1, EdgeType::Exact("new_edge".to_string()))
        .unwrap();
    // try to return p0->n1 edge, which should fail because n1 is not returned
    let res = builder.return_edge(p0, n1, EdgeType::Exact("new_edge1".to_string()));
    assert!(
        res.is_err(),
        "Expected returning edge p0->n1 to fail because n1 is not returned"
    );
    let operation = builder.build().unwrap();
    // get signature
    let signature = operation.signature();

    // assert our desired changes
    // number of explicit params
    assert_eq!(
        signature.parameter.explicit_input_nodes.len(),
        3,
        "Expected 3 explicit input nodes, p0, p1, p2"
    );
    // new nodes and edges
    assert_eq!(
        &signature.output.new_nodes,
        &HashMap::from([("new".into(), NodeType::String)]),
        "Expected new node 'new' of type String"
    );
    assert_eq!(
        &signature.output.new_edges,
        &HashMap::from([(
            (
                SubstMarker::from("p0").into(),
                SubstMarker::from("c1").into()
            ),
            EdgeType::Exact("new_edge".to_string()),
        )]),
        "Expected new edge from p0 to c1 of type 'new_edge'"
    );
    macro_rules! assert_deleted_and_changed_nodes_and_edges {
        ($signature:expr, $expected_maybe_changed_nodes:expr) => {
            // deleted nodes and edges
            assert_eq!(
                &$signature.output.maybe_deleted_nodes,
                &HashSet::from([
                    SubstMarker::from("p1").into(),
                    SubstMarker::from("c0").into()
                ]),
                "Expected nodes p1 and c0 to be deleted"
            );
            assert_eq!(
                &$signature.output.maybe_deleted_edges,
                &HashSet::from([
                    (
                        SubstMarker::from("p2").into(),
                        SubstMarker::from("c1").into()
                    ),
                    (
                        SubstMarker::from("p0").into(),
                        SubstMarker::from("c0").into()
                    )
                ]),
                "Expected edges p2->c1 and p0->c0 to be deleted"
            );
            // changed nodes and edges
            assert_eq!(
                &$signature.output.maybe_changed_nodes, &$expected_maybe_changed_nodes,
                "Expected nodes p0 to be changed to Integer and c1 to String"
            );
            assert_eq!(
                &$signature.output.maybe_changed_edges,
                &HashMap::from([(
                    (
                        SubstMarker::from("p0").into(),
                        SubstMarker::from("c1").into()
                    ),
                    EdgeType::Exact("new_edge".to_string())
                )]),
                "Expected edge p0->c1 to be changed to 'new_edge'"
            );
        };
    }
    assert_deleted_and_changed_nodes_and_edges!(
        signature,
        HashMap::from([
            (SubstMarker::from("p0"), NodeType::Integer),
            (SubstMarker::from("c1"), NodeType::String)
        ])
    );

    // Now ensure the same changes (minus the newly added nodes and edges) are propagated to another operation
    // that calls this operation.

    op_ctx.add_custom_operation(0, operation);
    let mut builder = OperationBuilder::new(&op_ctx, 1);
    // same parameter graph so we can call the other operation
    param_instructions(&mut builder);

    // now call the other operation
    builder
        .add_operation(BuilderOpLike::FromOperationId(0), vec![p0, p1, p2])
        .unwrap();
    let operation = builder.build().unwrap();
    let signature = operation.signature();
    // assert changes and deletions
    // note that the expected node changes are different for c1, since
    assert_deleted_and_changed_nodes_and_edges!(
        signature,
        HashMap::from([
            (SubstMarker::from("p0"), NodeType::Integer),
            (SubstMarker::from("c1"), NodeType::String)
        ])
    );
}

// TODO: add tests for:
//  * shape queries not being allowed to match already-matched nodes
//  * recursion abstract changes

macro_rules! recursion_signature_is_sound {
    (before) => {
        // when we change the abstract value of the node _before_ the recursive call
        recursion_signature_is_sound!(true, false, false, NodeType::Integer, NodeType::Integer);
    };
    (after) => {
        // when we change the abstract value of the node _after_ the recursive call
        recursion_signature_is_sound!(false, true, false, NodeType::Integer, NodeType::Integer);
    };
    ($fst:literal, $snd:literal, $set_last_to_string:literal, $p0_typ:expr, $c0_typ:expr) => {
        let op_ctx = OperationContext::<TestSemantics>::new();
        let mut builder = OperationBuilder::new(&op_ctx, 0);
        // the operation we're designing takes p0->c0, the start of a linked list, and sets all nodes (except the last node) to Integer.
        // it does the "except the last node" check by first seeing if there is a child, and only then recursing.
        builder
            .expect_parameter_node("p0", NodeType::Object)
            .unwrap();
        builder.expect_context_node("c0", NodeType::Object).unwrap();
        builder
            .expect_parameter_edge("p0", "c0", EdgeType::Wildcard)
            .unwrap();
        let p0 = AbstractNodeId::ParameterMarker("p0".into());
        let c0 = AbstractNodeId::ParameterMarker("c0".into());
        if $fst {
            builder
                .add_operation(
                    BuilderOpLike::Builtin(TestOperation::SetTo {
                        op_typ: NodeType::Object,
                        target_typ: NodeType::Integer,
                        value: NodeValue::Integer(0),
                    }),
                    vec![p0],
                )
                .unwrap();
        }
        builder.start_shape_query("q").unwrap();
        builder
            .expect_shape_node("child".into(), NodeType::Object)
            .unwrap();
        let child_aid = AbstractNodeId::dynamic_output("q", "child");
        builder
            .expect_shape_edge(c0.clone(), child_aid.clone(), EdgeType::Wildcard)
            .unwrap();
        builder.enter_true_branch().unwrap();
        // if we have a child, recurse
        builder
            .add_named_operation(
                "recurse".into(),
                BuilderOpLike::Recurse,
                vec![c0], // only need to select c0: child_aid should be matched by context.
            )
            .unwrap();
        if $set_last_to_string {
            builder.enter_false_branch().unwrap();
            // if we don't have a child, set the last node to String
            builder
                .add_operation(
                    BuilderOpLike::Builtin(TestOperation::SetTo {
                        op_typ: NodeType::Object,
                        target_typ: NodeType::String,
                        value: NodeValue::String("Last".to_string()),
                    }),
                    vec![c0.clone()],
                )
                .unwrap();
        }
        builder.end_query().unwrap();
        if $snd {
            builder
                .add_operation(
                    BuilderOpLike::Builtin(TestOperation::SetTo {
                        op_typ: NodeType::Object,
                        target_typ: NodeType::Integer,
                        value: NodeValue::Integer(0),
                    }),
                    vec![p0],
                )
                .unwrap();
        }

        let operation = builder.build().unwrap();
        let signature = operation.signature();
        // assert that the signature is correct
        assert_eq!(
            signature.output.maybe_deleted_nodes,
            HashSet::new(),
            "Expected no nodes to be deleted"
        );
        assert_eq!(
            signature.output.maybe_deleted_edges,
            HashSet::new(),
            "Expected no edges to be deleted"
        );
        assert_eq!(
            signature.output.maybe_changed_nodes,
            HashMap::from([
                (SubstMarker::from("p0").into(), $p0_typ),
                (SubstMarker::from("c0").into(), $c0_typ), // Note: c0 also changed due to the recursive call.
            ]),
            "Expected both p0 and c0 to change"
        );
        assert_eq!(
            signature.output.maybe_changed_edges,
            HashMap::new(),
            "Expected no edges to be changed"
        );
        assert_eq!(
            signature.output.new_nodes,
            HashMap::new(),
            "Expected no new nodes to be created"
        );
        assert_eq!(
            signature.output.new_edges,
            HashMap::new(),
            "Expected no new edges to be created"
        );
    };
}

#[test_log::test]
fn recursion_signature_is_sound_when_changed_before() {
    // if we do changes and then recurse, those are correctly communicated to caller operations via the signature.
    recursion_signature_is_sound!(before);
}

#[test_log::test]
fn recursion_signature_is_sound_when_changed_after() {
    // if we recurse and then do changes, those are correctly communicated to caller operations via the signature.
    recursion_signature_is_sound!(after);
    // Note: this test passes because we recalculate the signature at the very end, *and then use it for calculating the recurse call's effects*!
}

#[test_log::test]
fn recursion_signature_is_sound_when_changed_before_and_last_node_set_to_string() {
    // since c0 may or may not be the last node, the system has no choice but to infer a common supertype.
    recursion_signature_is_sound!(true, false, true, NodeType::Integer, NodeType::Object);
}

#[test_log::test]
fn recursion_signature_is_sound_when_changed_after_and_last_node_set_to_string() {
    // since c0 may or may not be the last node, the system has no choice but to infer a common supertype.
    recursion_signature_is_sound!(false, true, true, NodeType::Integer, NodeType::Object);
}

// TODO: add test for recursion that matches differently based on future changes. See the excalidraws.

#[test_log::test]
fn recursion_breaks_when_modification_changes_after_use() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // we're writing a recursive operation
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // recurse on p0
    builder
        .add_operation(BuilderOpLike::Recurse, vec![p0])
        .unwrap();
    // expect it to be an integer
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Integer, // This enforces that the input type is integer
                target_typ: NodeType::Integer, // no-op
                value: NodeValue::Integer(42),
            }),
            vec![p0],
        )
        .unwrap();
    // now, change the abstract value of p0 to String
    let res = builder.add_operation(
        BuilderOpLike::Builtin(TestOperation::SetTo {
            op_typ: NodeType::Object,
            target_typ: NodeType::String,
            value: NodeValue::String("Changed".to_string()),
        }),
        vec![p0],
    );
    assert!(
        res.is_err(),
        "Expected changing the abstract value of p0 to fail, since it is used in a recursive call after which it is used as an integer"
    );
}

#[test_log::test]
fn shape_query_doesnt_match_nodes_for_which_handles_exist() {
    // TODO: make this more lenient. See problems-testcases.md to support eg read-only shape queries.

    // If an outer operation already has a handle to a specific concrete node (checked dynamically),
    // then a shape query cannot match that node.

    fn get_shape_query_modifying_operation(
        op_id: OperationId,
    ) -> UserDefinedOperation<TestSemantics> {
        let op_ctx = OperationContext::<TestSemantics>::new();
        let mut builder = OperationBuilder::new(&op_ctx, op_id);
        builder
            .expect_parameter_node("p0", NodeType::Object)
            .unwrap();
        let p0 = AbstractNodeId::param("p0");
        // start a shape query for a child.
        builder.start_shape_query("q").unwrap();
        builder
            .expect_shape_node("child".into(), NodeType::Object)
            .unwrap();
        let child_aid = AbstractNodeId::dynamic_output("q", "child");
        builder
            .expect_shape_edge(p0, child_aid, EdgeType::Wildcard)
            .unwrap();
        builder.enter_true_branch().unwrap();
        // if we have a child, set it to "I'm a string"
        // TODO: once we support read-only shape queries, add a second test that replaces this SetTo with a CopyTo, and then assert that it is matched.
        builder
            .add_operation(
                BuilderOpLike::Builtin(TestOperation::SetTo {
                    op_typ: NodeType::Object,
                    target_typ: NodeType::String,
                    value: NodeValue::String("I'm a string".to_string()),
                }),
                vec![child_aid],
            )
            .unwrap();
        builder.enter_false_branch().unwrap();
        // if we don't, set p0 to "no child"
        builder
            .add_operation(
                BuilderOpLike::Builtin(TestOperation::SetTo {
                    op_typ: NodeType::Object,
                    target_typ: NodeType::String,
                    value: NodeValue::String("no child".to_string()),
                }),
                vec![p0],
            )
            .unwrap();

        builder.build().unwrap()
    }

    let mut op_ctx = OperationContext::<TestSemantics>::new();
    op_ctx.add_custom_operation(0, get_shape_query_modifying_operation(0));
    let mut builder = OperationBuilder::new(&op_ctx, 1);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    builder
        .expect_context_node("c0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    let c0 = AbstractNodeId::param("c0");
    builder
        .expect_parameter_edge("p0", "c0", EdgeType::Wildcard)
        .unwrap();
    // call op 0
    builder
        .add_operation(BuilderOpLike::FromOperationId(0), vec![p0])
        .unwrap();
    let state = builder.show_state().unwrap();
    // c0 should still be Integer, since the operation does not know about the inner operation's shape query.
    let c0_key = state.node_keys_to_aid.get_right(&c0).unwrap();
    assert_eq!(
        state.graph.get_node_attr(*c0_key).unwrap(),
        &NodeType::Integer,
        "Expected c0 to remain unchanged, since the operation does not know about the inner operation's shape query"
    );

    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(1, op);

    // now run the operation with a concrete graph
    {
        // in the concrete:
        // check that no child leads to the node being set to "no child"
        let mut g_no_child = TestSemantics::new_concrete_graph();
        let p0_key = g_no_child.add_node(NodeValue::Integer(42));
        run_from_concrete(&mut g_no_child, &op_ctx, 0, &[p0_key]).unwrap();
        let p0_value = g_no_child.get_node_attr(p0_key).unwrap();
        assert_eq!(
            p0_value,
            &NodeValue::String("no child".to_string()),
            "Expected p0 to be set to 'no child' when no child exists"
        );
    }
    {
        // in the concrete:
        // check that a node with a child leads to the child being set to "I'm a string"
        let mut g_with_child = TestSemantics::new_concrete_graph();
        let p0_key = g_with_child.add_node(NodeValue::Integer(42));
        let c0_key = g_with_child.add_node(NodeValue::Integer(43));
        g_with_child.add_edge(p0_key, c0_key, "child".to_string());
        run_from_concrete(&mut g_with_child, &op_ctx, 0, &[p0_key]).unwrap();
        let p0_value = g_with_child.get_node_attr(p0_key).unwrap();
        let c0_value = g_with_child.get_node_attr(c0_key).unwrap();
        assert_eq!(
            p0_value,
            &NodeValue::Integer(42),
            "Expected p0 to remain unchanged when a child exists"
        );
        assert_eq!(
            c0_value,
            &NodeValue::String("I'm a string".to_string()),
            "Expected child to be set to 'I'm a string' when it exists"
        );
    }
    {
        // in the abstract, i.e., with the outer operation active and having a handle to the child node:
        let mut g = TestSemantics::new_concrete_graph();
        let p0_key = g.add_node(NodeValue::Integer(42));
        let c0_key = g.add_node(NodeValue::Integer(43));
        g.add_edge(p0_key, c0_key, "child".to_string());
        run_from_concrete(&mut g, &op_ctx, 1, &[p0_key]).unwrap();
        let p0_value = g.get_node_attr(p0_key).unwrap();
        let c0_value = g.get_node_attr(c0_key).unwrap();
        // despite p0 having a child, the shape query should not match it.
        // otherwise, the abstract information from the outer operation is unsound.
        assert_eq!(
            p0_value,
            &NodeValue::String("no child".to_string()),
            "Expected p0 to be set to 'no child' when the shape query does not match the child node, even if one exists"
        );
        assert_eq!(
            c0_value,
            &NodeValue::Integer(43),
            "Expected child to remain unchanged since it is not matched, even though it exists",
        );
    }
}

// TODO: add "forget node" instruction that can be used to make sure that shape queries can match and modify
//  the forgotten nodes.

#[test_log::test]
fn may_writes_remember_previous_abstract_value() {
    // The semantics of our abstract output changes for the "changed_*" cases is that those are writes that *may have happened*.
    // Importantly, they may not have happened too.
    // So, we cannot *overwrite* the current abstract value with the result of a write that only *may* have happened, we need to consider the case that no write happened.
    // The builder itself must also be aware of this and not overapproximate to the point of uselessness.

    let mut op_ctx = OperationContext::<TestSemantics>::new();

    let op = {
        let mut builder = OperationBuilder::new(&op_ctx, 0);
        // an operation that takes a p0: Object and changes it to a String.
        // TODO: we could loosen the constraints and make it so that a known, unconditional change, even in a user defined op, leads to unconditional changes in the caller.
        //  but at the moment, any changes in UDOs are considered "may" changes.
        // in other words, the query below is actually not necessary.
        builder
            .expect_parameter_node("p0", NodeType::Object)
            .unwrap();
        let p0 = AbstractNodeId::param("p0");
        // note: query only necessary to actually break type safety in the concrete.
        builder
            .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(3)), vec![p0])
            .unwrap();
        builder.enter_true_branch().unwrap();
        builder
            .add_operation(
                BuilderOpLike::LibBuiltin(LibBuiltinOperation::SetNode {
                    param: NodeType::Object,
                    value: NodeValue::String("Changed".to_string()),
                }),
                vec![p0],
            )
            .unwrap();
        builder.end_query().unwrap();
        let state = builder.show_state().unwrap();
        // note that we still expect p0 to be object, it's just that we tell our caller that it may have changed to String.
        let typ = state.node_av_of_aid(&p0).unwrap();
        assert_eq!(
            typ,
            &NodeType::Object,
            "Expected p0 to remain Object after conditional change to String"
        );
        builder.build().unwrap()
    };
    op_ctx.add_custom_operation(0, op);

    let mut builder = OperationBuilder::new(&op_ctx, 1);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // a builtin op that changes p0 to Integer should unconditionally change the abstract value of p0 to Integer.
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::SetNode {
                param: NodeType::Object,
                value: NodeValue::Integer(42),
            }),
            vec![p0],
        )
        .unwrap();
    let state = builder.show_state().unwrap();
    let typ = state.node_av_of_aid(&p0).unwrap();
    assert_eq!(
        typ,
        &NodeType::Integer,
        "Expected p0 to be Integer after unconditional, builtin SetNode"
    );

    // a custom op that conditionally (i.e., *may*) changes p0 to String should change the abstract value of p0 to the join of its previous value (Integer) and String, so Object.
    builder
        .add_operation(BuilderOpLike::FromOperationId(0), vec![p0])
        .unwrap();
    let state = builder.show_state().unwrap();
    let typ = state.node_av_of_aid(&p0).unwrap();
    assert_eq!(
        typ,
        &NodeType::Object,
        "Expected p0 to be Object after conditional, user-defined SetNode"
    );

    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(1, op);

    // See here for broken type safety:
    if true {
        // if false {
        let mut g = TestSemantics::new_concrete_graph();
        let p0_key = g.add_node(NodeValue::Integer(0));
        // this won't hit the inner operation's set to string operation, so it would remain integer.
        run_from_concrete(&mut g, &op_ctx, 1, &[p0_key]).unwrap();
        let p0_value = g.get_node_attr(p0_key).unwrap();
        assert_eq!(
            p0_value,
            &NodeValue::Integer(42),
            "Expected p0 to remain Integer after running the operation, since the inner operation's set to String was not hit"
        );
    }

    // furthermore, just because a operation _may_ change a node, it doesn't unnecessarily make the caller's av less precise.
    let mut builder = OperationBuilder::new(&op_ctx, 2);
    builder
        .expect_parameter_node("p0", NodeType::String)
        .unwrap();
    // we expect a String
    builder
        .add_operation(BuilderOpLike::FromOperationId(0), vec![p0])
        .unwrap();
    let state = builder.show_state().unwrap();
    let typ = state.node_av_of_aid(&p0).unwrap();
    assert_eq!(
        typ,
        &NodeType::String,
        "Expected p0 to remain String after running the operation, since the inner operation let us now it may at most write a string."
    );
}

// Test that shape queries have the entire connected component as context - i.e., they cannot match anything that already exists in the abstract graph.
#[test_log::test]
fn shape_query_cannot_match_existing_nodes() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // we expect a p0: Object -child-> p1: Object
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    builder
        .expect_parameter_node("p1", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    let p1 = AbstractNodeId::param("p1");
    builder
        .expect_parameter_edge("p0", "p1", EdgeType::Exact("child".into()))
        .unwrap();
    // start a shape query for a child.
    builder.start_shape_query("q").unwrap();
    builder
        .expect_shape_node("child".into(), NodeType::Object)
        .unwrap();
    let child_aid = AbstractNodeId::dynamic_output("q", "child");
    builder
        .expect_shape_edge(p0, child_aid, EdgeType::Exact("child".into()))
        .unwrap();
    builder.enter_true_branch().unwrap();
    // now delete child
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![child_aid],
        )
        .unwrap();
    builder.end_query().unwrap();
    // we expect that p1 was not matched, hence it must still exist
    let state = builder.show_state().unwrap();
    let p1_av = state.node_av_of_aid(&p1);
    assert_eq!(
        p1_av,
        Some(&NodeType::Object),
        "Expected p1 to remain Object after running the shape query, since it was not matched"
    );
    // finish
    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(0, op);

    // run on concrete graph.
    let mut g = TestSemantics::new_concrete_graph();
    let p0_key = g.add_node(NodeValue::Integer(0));
    let p1_key = g.add_node(NodeValue::Integer(1));
    g.add_edge(p0_key, p1_key, "child".into());

    run_from_concrete(&mut g, &op_ctx, 0, &[p0_key, p1_key]).unwrap();
    // assert that p1 still exists
    assert_eq!(
        g.get_node_attr(p1_key),
        Some(&NodeValue::Integer(1)),
        "Expected p1 to remain a node in the graph",
    );
}

#[test_log::test]
fn rename_nodes_and_merge_test() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // assert that we cannot rename a param
    let res = builder.rename_node(p0, "test");
    assert!(
        res.is_err(),
        "Expected to not be able to rename a parameter marker"
    );

    // first branches: merge two nodes with same output marker, but different operation marker
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![p0])
        .unwrap();
    builder.enter_true_branch().unwrap();
    builder
        .add_named_operation(
            "op1".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::String("hello".into()),
            }),
            vec![],
        )
        .unwrap();
    let true_branch_aid = AbstractNodeId::dynamic_output("op1", "new");
    builder.rename_node(true_branch_aid, "a").unwrap();
    builder.enter_false_branch().unwrap();
    builder
        .add_named_operation(
            "op2".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(0),
            }),
            vec![],
        )
        .unwrap();
    let false_branch_aid = AbstractNodeId::dynamic_output("op2", "new");
    builder.rename_node(false_branch_aid, "a").unwrap();
    builder.end_query().unwrap();
    let a_aid = AbstractNodeId::named("a");
    // assert that the node "a" is now a merge of the two nodes
    let state = builder.show_state().unwrap();
    let a_av = state.node_av_of_aid(&a_aid);
    assert_eq!(
        a_av,
        Some(&NodeType::Object),
        "Expected node 'a' to be a merge of the two nodes with the same name, but different operation markers"
    );

    // second branches: shape queries with same operation marker, but different output markers
    // also assert that we cannot return these
    builder.start_shape_query("query").unwrap();
    builder
        .expect_shape_node("wow".into(), NodeType::Integer)
        .unwrap();
    let wow_aid = AbstractNodeId::dynamic_output("query", "wow");
    builder
        .expect_shape_edge(p0, wow_aid, EdgeType::Wildcard)
        .unwrap();
    builder.enter_true_branch().unwrap();
    builder.rename_node(wow_aid, "b").unwrap();
    builder.enter_false_branch().unwrap();
    // in false branch create a new node ("wow", "new")
    builder
        .add_named_operation(
            "wow".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::String("Something".into()),
            }),
            vec![],
        )
        .unwrap();
    let false_branch_aid = AbstractNodeId::dynamic_output("wow", "new");
    builder.rename_node(false_branch_aid, "b").unwrap();
    builder.end_query().unwrap();
    let b_aid = AbstractNodeId::named("b");
    // assert that the node "b" is now a merge of the two nodes
    let state = builder.show_state().unwrap();
    let b_av = state.node_av_of_aid(&b_aid);
    assert_eq!(
        b_av,
        Some(&NodeType::Object),
        "Expected node 'b' to be a merge of the two nodes with the same operation marker, but different output markers"
    );
    // assert that we cannot return these nodes
    // UPDATE: we're allowed to return shape query nodes now
    let res = builder.return_node(b_aid, "b".into(), NodeType::Object);
    // assert!(
    //     res.is_err(),
    //     "Expected to not be able to return a renamed node that partially comes from a shape query",
    // );
    assert!(res.is_ok());
}

#[test_log::test]
fn shape_query_allows_refinement_of_existing_nodes_and_edges() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // we expect a p0: Object -*-> p1: Object
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    builder
        .expect_parameter_node("p1", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    let p1 = AbstractNodeId::param("p1");
    builder
        .expect_parameter_edge("p0", "p1", EdgeType::Wildcard)
        .unwrap();

    // first: start a shape query to check if p0->p1 has type "child"
    builder.start_shape_query("q").unwrap();
    builder
        .expect_shape_edge(p0, p1, EdgeType::Exact("child".to_string()))
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if it does, set p0 to "has child"
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::SetNode {
                param: NodeType::Object,
                value: NodeValue::String("has child".to_string()),
            }),
            vec![p0],
        )
        .unwrap();
    builder.end_query().unwrap();

    // also, check if p1 has type Integer.
    builder.start_shape_query("q1").unwrap();
    builder
        .expect_shape_node_change(p1, NodeType::Integer)
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if it does, set p1 to "was integer"
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::SetNode {
                param: NodeType::Object,
                value: NodeValue::String("was integer".to_string()),
            }),
            vec![p1],
        )
        .unwrap();

    // finalize and test
    let operation = builder.build().unwrap();
    op_ctx.add_custom_operation(0, operation);

    {
        // in the concrete:
        // check that p0->p1 has type "child" and p0 is set to "has child" and p1 is set to "was integer"
        let mut g = TestSemantics::new_concrete_graph();
        let p0_key = g.add_node(NodeValue::Integer(42));
        let p1_key = g.add_node(NodeValue::Integer(43));
        g.add_edge(p0_key, p1_key, "child".to_string());

        run_from_concrete(&mut g, &op_ctx, 0, &[p0_key, p1_key]).unwrap();

        let p0_value = g.get_node_attr(p0_key);
        assert_eq!(p0_value, Some(&NodeValue::String("has child".to_string())));

        let p1_value = g.get_node_attr(p1_key);
        assert_eq!(
            p1_value,
            Some(&NodeValue::String("was integer".to_string()))
        );
    }
}

#[test_log::test]
fn shape_query_av_refinement_works_in_branch_merge() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // we expect a p0: Object
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // check if p0 is actually an Integer
    builder.start_shape_query("q").unwrap();
    builder
        .expect_shape_node_change(p0, NodeType::Integer)
        .unwrap();
    builder.enter_false_branch().unwrap();
    // if not, turn it into an integer
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::SetNode {
                param: NodeType::Object,
                value: NodeValue::Integer(42),
            }),
            vec![p0],
        )
        .unwrap();
    builder.end_query().unwrap();

    // assert that after the merge we statically know that p0 is an Integer
    let state = builder.show_state().unwrap();
    let p0_av = state.node_av_of_aid(&p0);
    assert_eq!(
        p0_av,
        Some(&NodeType::Integer),
        "Expected p0 to be Integer after the shape query refinement"
    );

    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(0, op);

    // check behavior in concrete
    let mut g = TestSemantics::new_concrete_graph();
    let p0_key = g.add_node(NodeValue::String("not an integer".to_string()));
    run_from_concrete(&mut g, &op_ctx, 0, &[p0_key]).unwrap();
    // assert that p0 is now an integer
    let p0_value = g.get_node_attr(p0_key);
    assert_eq!(
        p0_value,
        Some(&NodeValue::Integer(42)),
        "Expected p0 to be set to Integer after the shape query did not match"
    );

    let p1_key = g.add_node(NodeValue::Integer(100));
    run_from_concrete(&mut g, &op_ctx, 0, &[p1_key]).unwrap();
    // assert that p1 did not change since the true branch does nothing
    let p1_value = g.get_node_attr(p1_key);
    assert_eq!(
        p1_value,
        Some(&NodeValue::Integer(100)),
        "Expected p1 to not change"
    );
}

// Check that deleting a node in an inner op, then calling that on an abstract graph with an edge connected to it results in a signature that
// indicates the edge was also deleted.
#[test_log::test]
fn delete_node_deletes_all_incident_edges_in_signature() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let op_deleting_one_node = {
        let mut builder = OperationBuilder::new(&op_ctx, 0);
        // expect p0: Object
        builder
            .expect_parameter_node("p0", NodeType::Object)
            .unwrap();
        let p0 = AbstractNodeId::param("p0");
        // delete it
        builder
            .add_operation(
                BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                    param: NodeType::Object,
                }),
                vec![p0],
            )
            .unwrap();
        let op = builder.build().unwrap();
        // assert op is deleting a node
        let signature = op.signature();
        assert_eq!(
            signature.output.maybe_deleted_nodes,
            HashSet::from([SubstMarker::from("p0")]),
            "Expected the operation to delete p0"
        );
        op
    };
    op_ctx.add_custom_operation(0, op_deleting_one_node);

    // now call that operation from an operation that expects a p0: Object -child-> p1: Object
    let mut builder = OperationBuilder::new(&op_ctx, 1);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    builder
        .expect_parameter_node("p1", NodeType::Object)
        .unwrap();
    let _p0 = AbstractNodeId::param("p0");
    let p1 = AbstractNodeId::param("p1");
    builder
        .expect_parameter_edge("p0", "p1", EdgeType::Wildcard)
        .unwrap();
    // call the operation that deletes p1
    builder
        .add_operation(BuilderOpLike::FromOperationId(0), vec![p1])
        .unwrap();
    // assert that the signature indicates that p1 was deleted, and hence the edge p0->p1 was also deleted
    let op = builder.build().unwrap();
    let signature = op.signature();
    assert_eq!(
        signature.output.maybe_deleted_nodes,
        HashSet::from([SubstMarker::from("p1")]),
        "Expected the operation to delete p1"
    );
    assert_eq!(
        signature.output.maybe_deleted_edges,
        HashSet::from([("p0".into(), "p1".into())]),
        "Expected the operation to delete the edge p0->p1"
    );
}

#[test_log::test]
fn delete_node_after_writing_to_it() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // expect p0: Object
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // write to it
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::SetNode {
                param: NodeType::Object,
                value: NodeValue::String("Hello".to_string()),
            }),
            vec![p0],
        )
        .unwrap();
    // delete it
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![p0],
        )
        .unwrap();
    let op = builder.build().unwrap();
    // assert op is deleting a node
    let signature = op.signature();
    assert_eq!(
        signature.output.maybe_deleted_nodes,
        HashSet::from([SubstMarker::from("p0")]),
        "Expected the operation to delete p0"
    );
    // TODO: decide if this should be inside the signature or not.
    //  the node has been deleted, so really this is useless information and could confuse consumers.
    //  so add an invariant to opsignature that a node can only be mentioned in either deleted or changed?
    //  could use an enum for that. and make it a hashmap to enforce uniqueness: p0 => Deleted/Changed.
    // TODO: do this!
    assert_eq!(
        signature.output.maybe_changed_nodes,
        HashMap::from([(SubstMarker::from("p0"), NodeType::String)]),
        "Expected the operation to change p0 to Object before deleting it"
    );
}

// Just for testing the new builder. TODO: delete.
#[test_log::test]
fn new_builder_test() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = stack_based_builder::Builder::new(&op_ctx, 0);

    use grabapl::operation::builder::BuilderInstruction as BI;

    builder
        .consume(BI::ExpectParameterNode("p0".into(), NodeType::Object))
        .unwrap();

    builder
        .consume(BI::AddNamedOperation(
            "hello".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::String("Hello".to_string()),
            }),
            vec![],
        ))
        .unwrap();
    let show_data = builder.show();
    println!("{show_data:?}");

    builder
        .consume(BI::RenameNode(
            AbstractNodeId::dynamic_output("hello", "new"),
            "renamed".into(),
        ))
        .unwrap();
    let show_data = builder.show();
    println!("{show_data:?}");

    let res = builder.consume(BI::ExpectParameterNode("p0".into(), NodeType::Object));
    println!("{res:?}");

    // assert!(false);
}

#[test_log::test]
fn recursion_return_node() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // we're writing a recursive operation that returns a node.
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // check if p0 is 200, if so, create a new node with value 100 and return it.
    // otherwise, recurse on p0. (note this is a nonsense operation)
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(200)), vec![p0])
        .unwrap();
    builder.enter_true_branch().unwrap();
    builder
        .add_named_operation(
            "op0".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(100),
            }),
            vec![],
        )
        .unwrap();
    let new_node_aid = AbstractNodeId::dynamic_output("op0", "new");
    // TODO: unfortunate that we need to specify "ret_node" twice as string.
    // maybe it would be nice to have the rename_node, expect_parameter_node etc ops return an AID?
    let ret_node_aid = AbstractNodeId::named("ret_node");
    builder.rename_node(new_node_aid, "ret_node").unwrap();
    builder.enter_false_branch().unwrap();
    // recurse
    builder
        .add_named_operation("recursion".into(), BuilderOpLike::Recurse, vec![p0])
        .unwrap();
    let new_node_aid = AbstractNodeId::dynamic_output("recursion", "ret_node");
    builder
        .expect_self_return_node("ret_node", NodeType::Integer)
        .unwrap();
    // rename the output node to ret_node
    builder.rename_node(new_node_aid, "ret_node").unwrap();
    builder.end_query().unwrap();
    // return the ret_node
    builder
        .return_node(ret_node_aid, "ret_node".into(), NodeType::Integer)
        .unwrap();

    // TODO: make this work.
    //  Maybe it should be by allowing us to specify expected return nodes somewhere? maybe anywhere?
    //  and then we make sure we consider those:
    //  1. when recursing - we need to create the nodes (pretend they exist)
    //  2. when building - we need to make sure that the return nodes are actually created and returned with the appropriate name and type.

    let _ = builder.build().unwrap();
}

#[test_log::test]
fn recursion_expect_self_return_node_corner_cases() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // we're writing a recursive operation that returns a node.
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // recurse on p0
    builder
        .add_named_operation("self".into(), BuilderOpLike::Recurse, vec![p0])
        .unwrap();
    // expect self output to be an integer
    builder
        .expect_self_return_node("ret_node", NodeType::Integer)
        .unwrap();
    let ret_node_aid = AbstractNodeId::dynamic_output("self", "ret_node");
    // Call op that requires integer argument
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::Integer, // This enforces that the input type is integer
                target_typ: NodeType::Integer, // no-op
                value: NodeValue::Integer(42),
            }),
            vec![ret_node_aid],
        )
        .unwrap();

    // now pull a little prank and expect the return node to actually be String
    let res = builder.expect_self_return_node("ret_node", NodeType::String);
    assert!(
        res.is_err(),
        "Expected expecting the return node to be String to fail, since it needs to be Integer in a prior instruction"
    );

    // build without returning
    let res = builder.build();
    assert!(
        res.is_err(),
        "Expected building the operation to fail, since we expect a return node that is not created"
    );
    // now actually return the node
    builder
        .return_node(ret_node_aid, "ret_node".into(), NodeType::Integer)
        .unwrap();
    let _ = builder.build().unwrap();
}
