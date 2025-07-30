mod util;

use grabapl::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::operation::builtin::LibBuiltinOperation;
use grabapl::operation::user_defined::AbstractNodeId;
use grabapl::prelude::*;
use util::interval_semantics::*;

#[test_log::test]
fn example() {
    let op_ctx = OperationContext::<IntervalSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::new(0, 10))
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddInteger(5)),
            vec![p0],
        )
        .unwrap();

    let state = builder.show_state().unwrap();
    eprintln!("{}", state.graph.dot());
    let resulting_type = state.node_av_of_aid(&p0).unwrap();
    assert_eq!(resulting_type, &NodeType::new(5, 15));

    // test adding a new node
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue(2),
            }),
            vec![],
        )
        .unwrap();
    let new_node = AbstractNodeId::dynamic_output("new", "new");

    let state = builder.show_state().unwrap();
    eprintln!("{}", state.graph.dot());
    let resulting_type = state.node_av_of_aid(&new_node).unwrap();
    assert_eq!(resulting_type, &NodeType::new(2, 2));
}

// See AH! for why this test is disabled. (TL;DR we need match statements)
// #[test_log::test]
#[allow(dead_code)]
fn recursion_fixed_point() {
    let op_ctx = OperationContext::<IntervalSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx, 0);
    builder
        .expect_parameter_node("p0", NodeType::new(0, 100))
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // now we check if p0 is 100, if not, we add 1 to it and repeat
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue(100)), vec![p0])
        .unwrap();
    builder.enter_false_branch().unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddInteger(1)),
            vec![p0],
        )
        .unwrap();
    // recurse
    // AH! this causes a problem. Add1 turns our [0,100] into a [1,101] interval, and the
    // query does not actually give us a new type in true vs false branch. if it gave us [0,99], then it would be fine.
    builder
        .add_operation(BuilderOpLike::Recurse, vec![p0])
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if p0 is 100, we do nothing

    let op = builder.build().unwrap();
    println!("Built operation: {:#?}", op.signature.output);
}
