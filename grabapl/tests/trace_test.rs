mod util;

use grabapl::graph::dot::DotCollector;
use grabapl::prelude::{run_from_concrete, ConcreteGraph};
use syntax::grabapl_defs;
use util::semantics::*;

grabapl_defs!(get_ops, TestSemantics,
fn children_to_list(p: int, l: int) {
    if shape [
        child: int,
        p -> child: "child",
    ] {
        // trace whenever we find a child
        trace();
        insert_child_to_list(child, l);
        children_to_list(p, l);
    }
}

fn mk_list() -> (head: int) {
    let! head = add_node<int,42>();
    return (head: head);
}

fn list_insert_by_copy(head: int, value: int) {
    if shape [
        child: Integer,
        head -> child: *,
    ] {
        list_insert_by_copy(child, value);
    } else {
        // we're at the tail
        let! new_node = add_node<int,0>();
        copy_value_from_to(value, new_node);
        add_edge<"next">(head, new_node);
    }
}

);

#[test_log::test]
fn trace_test() {
    let (op_ctx, fn_names) = get_ops();
    let mut g = ConcreteGraph::<TestSemantics>::new();
    let res = run_from_concrete(&mut g, &op_ctx, fn_names["mk_list"], &[]).unwrap();
    let list_key = res.key_of_output_marker("head").unwrap();

    // make parent with 5 children
    let parent_key = g.add_node(NodeValue::Integer(100));
    for i in 0..5 {
        let child_key = g.add_node(NodeValue::Integer(i));
        g.add_edge(parent_key, child_key, "child".to_string());
    }

    // run the operation
    let res = run_from_concrete(&mut g, &op_ctx, fn_names["children_to_list"], &[parent_key, list_key]).unwrap();

    let trace = res.trace;

    let mut dot_collector = DotCollector::new();
    for frame in trace.frames {
        dot_collector.collect_raw(&frame.dot());
    }

    println!("Trace DOT:\n{}", dot_collector.finalize());

    // just playing around
    assert!(false);
}
