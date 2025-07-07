mod util;

use grabapl::graph::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::graph::operation::user_defined::AbstractNodeId;
use grabapl::OperationContext;
use util::interval_semantics::*;

#[test]
fn example() {
    let mut op_ctx = OperationContext::<IntervalSemantics>::new();
    let mut builder = OperationBuilder::new(&op_ctx);
    builder.expect_parameter_node("p0", NodeType::new(0, 10)).unwrap();
    let p0 = AbstractNodeId::param("p0");
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::AddInteger(5)), vec![p0]).unwrap();

    let state = builder.show_state().unwrap();
    eprintln!("{:#?}", state.graph.dot());
    let resulting_type = state.node_av_of_aid(&p0).unwrap();
    assert_eq!(resulting_type, &NodeType::new(5, 15));
}