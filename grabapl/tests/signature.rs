mod util;

use grabapl::prelude::*;
use std::collections::{HashMap, HashSet};
use util::semantics::*;

#[test_log::test]
fn self_return_nodes_are_respected() {
    // if the user asserts that they will return some node under some type, then they must return that node before building.

    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    builder
        .expect_self_return_node("ret1", NodeType::Object)
        .unwrap();
    builder
        .expect_self_return_node("ret2", NodeType::Object)
        .unwrap();

    let res = builder.build();
    assert!(
        res.is_err(),
        "Expected error when building without returning the expected nodes"
    );

    // now create and return the nodes
    builder
        .add_named_operation(
            "ret1".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(0),
            }),
            vec![],
        )
        .unwrap();
    builder
        .add_named_operation(
            "ret2".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::String("hello".to_string()),
            }),
            vec![],
        )
        .unwrap();
    let ret1 = AbstractNodeId::dynamic_output("ret1", "new");
    let ret2 = AbstractNodeId::dynamic_output("ret2", "new");

    // returning first node as integer does not work, since the self return expected object
    let res = builder.return_node(ret1, "ret1".into(), NodeType::Integer);
    assert!(
        res.is_err(),
        "Expected error when returning node with different type than expected"
    );
    // returning it as object works
    builder
        .return_node(ret1, "ret1".into(), NodeType::Object)
        .unwrap();

    // however, building now still doesnt work since we did not return the second node
    let res = builder.build();
    assert!(
        res.is_err(),
        "Expected error when building without returning the second expected node"
    );

    // returning the second node as object works
    builder
        .return_node(ret2, "ret2".into(), NodeType::Object)
        .unwrap();
    // now building should work
    let res = builder.build();
    assert!(
        res.is_ok(),
        "Expected successful build after returning all expected nodes"
    );
}

#[test_log::test]
fn invisible_node_not_deleted() {
    // if a parameter node receives un-joinable types in two branches, then the merge
    // cannot display that node.
    // in such a state, it is unclear what should happen.
    // Note for practical purposes: since this is confusing, a semantics should probably not allow
    // writing incompatible node types. i.e., if int and string are unjoinable, then a p: int should not
    // be allowed to be set to a string value.

    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    // expect a parameter node
    builder
        .expect_parameter_node("p0", NodeType::String)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // start a query and in true branch set the node to NodeType::Separate
    builder
        .start_query(
            TestQuery::ValueEqualTo(NodeValue::String("hello".to_string())),
            vec![p0],
        )
        .unwrap();
    builder.enter_true_branch().unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SetTo {
                op_typ: NodeType::String,
                target_typ: NodeType::Separate,
                value: NodeValue::String("hello".to_string()), // Separate has no clear values, let's just pick String.
            }),
            vec![p0],
        )
        .unwrap();
    builder.end_query().unwrap();
    // check the state
    let state = builder.show_state().unwrap();
    let aids: HashSet<AbstractNodeId> = state.node_keys_to_aid.right_values().copied().collect();
    assert_eq!(
        aids,
        HashSet::new(),
        "state should not have any visible AIDs, found aids: {:?}",
        aids
    );

    // check the signature
    let op = builder.build().unwrap();
    let sig = op.signature;
    assert_eq!(
        HashSet::from([SubstMarker::from("p0")]),
        sig.output.maybe_deleted_nodes,
        "signature should have p0 as deleted node, but found: {:?}",
        sig.output.maybe_deleted_nodes
    );
    // Note: this might be surprising. we did not delete p0 after all!
    // but for all intents and purposes, p0 is not visible in the output of the operation.

    // See initial TODO for more information:
    // TODO: write test to make sure we don't accidentally tell a caller we deleted a node or edge
    //  when merging states where two nodes cannot be merged!
    //  For example, type system without Top type, param p: Int, if cond { p = Int } else { p = String } // now we don't see p anymore.
    //  Make sure that this operation does not tell the caller that we deleted p!
    //  Actually !!! What do we do in this case? Does it make semantic sense to just pretend we deleted p?
    //  Since we cannot display the type and thus the node, since there is no join.
    //  However, this necessitates that we don't unconditionally delete nodes in the concrete,
    //  for which the signature says that it is `maybe_deleted`. ==> just add a test to document it a bit.
    // (we did this^)
}
