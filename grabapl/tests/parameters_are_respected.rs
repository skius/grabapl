mod util;

use grabapl::OperationContext;
use grabapl::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::operation::user_defined::AbstractNodeId;
use util::semantics::*;

#[test_log::test]
fn types_must_be_subtypes() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx);
    builder
        .expect_parameter_node("p0", NodeType::Object)
        .unwrap();
    builder
        .expect_parameter_node("p1", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    let p1 = AbstractNodeId::param("p1");
    // p0 cannot be used as String, nor as Integer
    let res = builder.add_operation(
        BuilderOpLike::Builtin(TestOperation::SetTo {
            op_typ: NodeType::String,
            target_typ: NodeType::String,
            value: NodeValue::String("hello".to_string()),
        }),
        vec![p0],
    );
    assert!(
        res.is_err(),
        "Expected error when using Object argument as String"
    );
    let res = builder.add_operation(
        BuilderOpLike::Builtin(TestOperation::SetTo {
            op_typ: NodeType::Integer,
            target_typ: NodeType::Integer,
            value: NodeValue::Integer(42),
        }),
        vec![p0],
    );
    assert!(
        res.is_err(),
        "Expected error when using Object argument as Integer"
    );
}
