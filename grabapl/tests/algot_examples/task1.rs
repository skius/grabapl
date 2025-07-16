use proptest::proptest;
use grabapl::prelude::*;
use super::semantics::*;

/// Returns an operation that solves "Task 1" from the OSF tasks.
fn get_gcd_op(self_op_id: OperationId) -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, self_op_id);

    // expect two integers, a and b, and a out-param return node
    builder
        .expect_parameter_node("a", NodeType::Integer)
        .unwrap();
    builder
        .expect_parameter_node("b", NodeType::Integer)
        .unwrap();
    builder
        .expect_parameter_node("ret", NodeType::Integer)
        .unwrap();
    let a = AbstractNodeId::param("a");
    let b = AbstractNodeId::param("b");
    let ret = AbstractNodeId::param("ret");

    // implement GCD
    builder.start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![b]).unwrap();
    builder.enter_true_branch().unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![a, ret],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();
    // create a temp node for the result of a % b
    builder.add_named_operation(
        "temp".into(),
        BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
            value: NodeValue::Integer(0),
        }),
        vec![],
    ).unwrap();
    let temp = AbstractNodeId::dynamic_output("temp", "new");
    // add the operation to compute a % b
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AModBToC),
            vec![a, b, temp],
        )
        .unwrap();
    // now copy b to a, and temp to b
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![b, a],
        )
        .unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![temp, b],
        )
        .unwrap();
    // and delete temp again
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![temp],
        )
        .unwrap();
    // and recurse on a,b
    builder
        .add_operation(
            BuilderOpLike::Recurse,
            vec![a, b, ret],
        )
        .unwrap();

    builder.build().unwrap()
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[test_log::test]
fn proptest_gcd() {
    let gcd_op_id = 0;
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let gcd_op = get_gcd_op(gcd_op_id);
    op_ctx.add_custom_operation(gcd_op_id, gcd_op);

    proptest!(|(a in 0..1000, b in 0..1000)| {
        let expected_gcd = gcd(a, b);
        assert!(expected_gcd == 0 || a % expected_gcd == 0 && b % expected_gcd == 0, "GCD of {} and {} should divide both", a, b);
        let mut g = TestSemantics::new_concrete_graph();
        let a_key = g.add_node(NodeValue::Integer(a));
        let b_key = g.add_node(NodeValue::Integer(b));
        let ret_key = g.add_node(NodeValue::Integer(0)); // placeholder for the result
        let run_result = run_from_concrete(&mut g, &op_ctx, gcd_op_id, &[a_key, b_key, ret_key]);
        assert!(run_result.is_ok(), "Running GCD operation failed: {:?}", run_result.err());
        let ret_value = g.get_node_attr(ret_key);
        assert!(ret_value.is_some(), "Result node should have a value");
        let ret_value = ret_value.unwrap();
        assert_eq!(ret_value, &NodeValue::Integer(expected_gcd), "Expected GCD of {} and {} to be {}, but got {:?}", a, b, expected_gcd, ret_value);
    });

}