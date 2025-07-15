//! This test module includes tests for the operation builder that test error conditions

mod util;
use util::semantics::*;
use grabapl::prelude::*;
use test_log::test;

#[test]
fn disconnected_context_node_not_allowed() {
    let op_ctx = OperationContext::<TestSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);

    // Expect a parameter node
    builder.expect_parameter_node("p0", NodeType::String).unwrap();
    let p0 = AbstractNodeId::param("p0");
    // Expect the disconnected context node
    builder.expect_context_node("context", NodeType::Object).unwrap();

    // Attempt to add an operation with a disconnected context node
    let res = builder.add_operation(
        BuilderOpLike::Builtin(TestOperation::SetTo {
            op_typ: NodeType::String,
            target_typ: NodeType::String,
            value: NodeValue::String("hello".to_string()),
        }),
        vec![p0],
    );

    // TODO: this currently fails because we unconditionally build the builder to cache
    //  the partial op.
    //  Instead we could only cache a new op when the build does not error, but we
    //  ignore the error and continue?
    //  we could also introspect the error and see if it is recoverable. Or add a method to the error.
    //  So.
    //  At the point where we build the new_builder_stage_1.build(), could we have
    //  an invariant for the builder that says the only error during build() can be
    //  when it is not valid to build yet?
    //  we could even use the type system to help us with that. (return an enum over Ok or NotValid(err))
    //  but this needs us to be certain that build() only fails due to that. and not eg
    //  consequences of a recursion. Otherwise we have false negatives and miss erroneous operations!
    // TODO: add ^ (consideration of not having false negatives) to msc thesis report notes

    // Assert that the operation fails due to the disconnected context node
    assert!(res.is_err(), "Expected error when proceeding with a disconnected context node");
}