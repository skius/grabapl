use grabapl::Semantics;
use grabapl::prelude::run_from_concrete;
use grabapl::semantics::example::{ExampleSemantics as TestSemantics, NodeValue};

const SRC: &str = stringify!(
    fn test(
        p1: int
    ) [
        c1: string,
        p1 -> c1: "child"
    ] -> (
        ret_node: int
    ) {
        let! new_node = add_node%int,5%();
        if cmp_fst_snd%<%(p1, new_node) {
            // p1 < new_node
            let new_map = add_node%int,10%();
            copy_value_from_to(p1, new_map.new);
            renamed := new_map.new;

        } else {
            // p1 >= new_node
            renamed := new_node;
        }
        remove_node(p1);
        return (ret_node: renamed);
    }
);

#[test]
fn rename_scope() {
    let (op_ctx, fn_map) = grabapl_syntax::parse_to_op_ctx_and_map::<TestSemantics>(SRC);

    let _ = grabapl_syntax::grabapl_parse!(TestSemantics, fn test() []{});

    let op_id = fn_map["test"];

    let mut g = TestSemantics::new_concrete_graph();
    let p1 = g.add_node(NodeValue::Integer(0));
    let c1 = g.add_node(NodeValue::String("c1_val".to_string()));
    g.add_edge(p1, c1, "child".to_string());

    let res = run_from_concrete(&mut g, &op_ctx, op_id, &[p1]).unwrap();

    println!("Graph after execution: {:#?}", g.dot());
    let ret_node = res.key_of_output_marker("ret_node").unwrap();
    let value = g.get_node_attr(ret_node).unwrap();
    println!("Return node value {ret_node:?}: {value:?}");

    // assert!(false);
}
