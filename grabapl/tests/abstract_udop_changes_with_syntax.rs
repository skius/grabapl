use grabapl::prelude::*;
use grabapl_template_semantics::{EdgeType, NodeType, TheSemantics};
use syntax::grabapl_parse;

#[test_log::test]
fn edge_av_maybe_write_respected() {
    let src = stringify!(
        fn main() {
            let! x = add_node();
            let! y = add_node();
            add_edge<"child">(x, y);
            foo(x, y);
            show_state();
        }

        fn foo(x: any, y: any) [x -> y: string] {
            new_edge<"parent">(x, y);
        }
    );

    let res = syntax::try_parse_to_op_ctx_and_map::<TheSemantics>(src, true);
    // assert successful parse
    assert!(res.op_ctx_and_map.is_ok(), "program should be valid");
    let state1 = res.state_map.values().next().unwrap();
    // assert that edge between x and y is string
    let x = AbstractNodeId::named("x");
    let y = AbstractNodeId::named("y");
    assert_eq!(
        state1.edge_av_of_aid(&x, &y).unwrap(),
        &EdgeType::String,
        "Expected edge type to change to join of 'child' and 'parent': string",
    );
}