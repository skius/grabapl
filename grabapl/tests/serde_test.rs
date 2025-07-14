mod util;

use grabapl::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::{OperationContext, Semantics};
use grabapl::operation::run_from_concrete;
use util::semantics::*;

#[cfg(feature = "serde")]
#[test_log::test]
fn serde_test() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    builder
        .expect_parameter_node("p1", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    let p1 = AbstractNodeId::param("p1");
    // add op that adds an edge
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddEdge {
                node_typ: NodeType::Object,
                param_typ: EdgeType::Wildcard,
                target_typ: EdgeType::Exact("hello".to_string()),
                value: "hello".to_string(),
            }),
            vec![p0, p1],
        )
        .unwrap();
    let op = builder.build().unwrap();

    let serialized = serde_json::to_string_pretty(&op).unwrap();
    eprintln!("Serialized operation: {}", serialized);

    let deserialized: UserDefinedOperation<TestSemantics> =
        serde_json::from_str(&serialized).unwrap();

    op_ctx.add_custom_operation(0, deserialized);
    // check if it does what it should

    let mut g = TestSemantics::new_concrete_graph();
    let n0 = g.add_node(NodeValue::String("blah".to_string()));
    let n1 = g.add_node(NodeValue::Integer(42));

    run_from_concrete(&mut g, &op_ctx, 0, &[n0, n1]).unwrap();

    let edge = g.get_edge_attr((n0, n1));
    assert_eq!(edge, Some(&"hello".to_string()));
}
