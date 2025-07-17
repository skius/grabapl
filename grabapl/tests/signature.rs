mod util;
use grabapl::prelude::*;
use util::semantics::*;

#[test_log::test]
fn self_return_nodes_are_respected() {
    // if the user asserts that they will return some node under some type, then they must return that node before building.
    
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    builder.expect_self_return_node("ret1", NodeType::Object).unwrap();
    builder.expect_self_return_node("ret2", NodeType::Object).unwrap();

    let res = builder.build();
    assert!(
        res.is_err(),
        "Expected error when building without returning the expected nodes"
    );

    // now create and return the nodes
    builder.add_named_operation(
        "ret1".into(),
        BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
            value: NodeValue::Integer(0),
        }),
        vec![],
    ).unwrap();
    let ret1 = AbstractNodeId::dynamic_output("ret1", "new");
    // returning it as integer does not work, since the self return expected object
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
    // now create and return the second node
    builder.add_named_operation(
        "ret2".into(),
        BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
            value: NodeValue::String("hello".to_string()),
        }),
        vec![],
    ).unwrap();
    let ret2 = AbstractNodeId::dynamic_output("ret2", "new");
    builder.return_node(ret2, "ret2".into(), NodeType::Object).unwrap();
    // now building should work
    let res = builder.build();
    assert!(
        res.is_ok(),
        "Expected successful build after returning all expected nodes"
    );

}