mod util;

use grabapl::OperationContext;
use grabapl::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::operation::builtin::LibBuiltinOperation;
use grabapl::operation::user_defined::AbstractNodeId;
use util::interval_semantics::*;

#[test_log::test]
fn example() {
    let op_ctx = OperationContext::<IntervalSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx);
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
