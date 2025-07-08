mod util;

use grabapl::graph::operation::builder::{BuilderOpLike, OperationBuilder, OperationBuilderError};
use grabapl::graph::operation::run_from_concrete;
use grabapl::graph::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::{OperationContext, OperationId, Semantics};
use proptest::proptest;
use std::cmp::Ordering::Greater;
use util::semantics::*;

fn bubble_sort_op(op_id: OperationId) -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::new();
    let mut builder = OperationBuilder::new(&op_ctx);

    // first node
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // check if child
    builder.start_shape_query("query").unwrap();
    builder
        .expect_shape_node("child".into(), NodeType::Integer)
        .unwrap();
    let child = AbstractNodeId::dynamic_output("query", "child");
    builder
        .expect_shape_edge(p0, child, EdgeType::Wildcard)
        .unwrap();

    builder.enter_true_branch().unwrap();
    // if we have a child, check if p0 > child
    builder
        .start_query(TestQuery::CmpFstSnd(Greater), vec![p0, child])
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if p0 > child, swap values
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SwapValues),
            vec![p0, child],
        )
        .unwrap();
    builder.end_query().unwrap();
    // continue with the child
    builder
        .add_operation(BuilderOpLike::Recurse, vec![child])
        .unwrap();

    builder.build(op_id).unwrap()
}

fn bubble_sort_op_2(op_id: OperationId) -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::new();
    let mut builder = OperationBuilder::new(&op_ctx);

    // first node
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // check if parent exists
    builder.start_shape_query("query").unwrap();
    builder
        .expect_shape_node("parent".into(), NodeType::Integer)
        .unwrap();
    let parent = AbstractNodeId::dynamic_output("query", "parent");
    builder
        .expect_shape_edge(parent, p0, EdgeType::Wildcard)
        .unwrap();

    builder.enter_true_branch().unwrap();
    // if we have a parent, check if parent > p0
    builder
        .start_query(TestQuery::CmpFstSnd(Greater), vec![parent, p0])
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if parent > p0, swap values
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SwapValues),
            vec![parent, p0],
        )
        .unwrap();
    builder.end_query().unwrap();
    // continue with the parent
    builder
        .add_operation(BuilderOpLike::Recurse, vec![parent])
        .unwrap();

    builder.build(op_id).unwrap()
}

fn list_len_op(op_id: OperationId) -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::new();
    let mut builder = OperationBuilder::new(&op_ctx);

    // first node
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // output node
    builder
        .expect_parameter_node("output", NodeType::Integer)
        .unwrap();
    let output = AbstractNodeId::param("output");
    // p0 exists, so increment output
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddInteger(1)),
            vec![output],
        )
        .unwrap();
    // if child exists, recurse
    builder.start_shape_query("query").unwrap();
    builder
        .expect_shape_node("child".into(), NodeType::Integer)
        .unwrap();
    let child = AbstractNodeId::dynamic_output("query", "child");
    builder
        .expect_shape_edge(p0, child, EdgeType::Wildcard)
        .unwrap();
    builder.enter_true_branch().unwrap();
    builder
        .add_operation(BuilderOpLike::Recurse, vec![child, output])
        .unwrap();
    builder.end_query().unwrap();

    builder.build(op_id).unwrap()
}

fn bubble_sort_n_times_op(op_id: OperationId) -> UserDefinedOperation<TestSemantics> {
    let op_ctx = OperationContext::new();
    let mut builder = OperationBuilder::new(&op_ctx);

    // first node
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // number of times to run the bubble sort
    builder
        .expect_parameter_node("n", NodeType::Integer)
        .unwrap();
    let n = AbstractNodeId::param("n");

    // if n == 0, return
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![n])
        .unwrap();
    builder.enter_true_branch().unwrap();
    // do nothing, just return
    builder.enter_false_branch().unwrap();
    // if not, check for child, swap, decrement n, and recurse
    builder.start_shape_query("query").unwrap();
    builder
        .expect_shape_node("child".into(), NodeType::Integer)
        .unwrap();
    let child = AbstractNodeId::dynamic_output("query", "child");
    builder
        .expect_shape_edge(p0, child, EdgeType::Wildcard)
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if we have a child, check if p0 > child
    builder
        .start_query(TestQuery::CmpFstSnd(Greater), vec![p0, child])
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if p0 > child, swap values
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::SwapValues),
            vec![p0, child],
        )
        .unwrap();
    builder.end_query().unwrap();
    // decrement n
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddInteger(-1)),
            vec![n],
        )
        .unwrap();
    // continue with the child
    builder
        .add_operation(BuilderOpLike::Recurse, vec![child, n])
        .unwrap();

    builder.build(op_id).unwrap()
}

fn main_bubble_sort_op(
    op_id: OperationId,
    n_times_op_id: OperationId,
    op_ctx: &OperationContext<TestSemantics>,
) -> UserDefinedOperation<TestSemantics> {
    let mut builder = OperationBuilder::new(op_ctx);
    // first node
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // number of times to run the bubble sort
    builder
        .expect_parameter_node("n", NodeType::Integer)
        .unwrap();
    let n = AbstractNodeId::param("n");
    // if n == 0, return
    builder
        .start_query(TestQuery::ValueEqualTo(NodeValue::Integer(0)), vec![n])
        .unwrap();
    builder.enter_true_branch().unwrap();
    // do nothing, just return
    builder.enter_false_branch().unwrap();
    // if not, run bubble sort for n steps, then decrement n and recurse on self
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::Integer,
                value: NodeValue::Integer(0),
            }),
            vec![],
        )
        .unwrap();
    let new_node = AbstractNodeId::dynamic_output("new", "new");
    // copy from n to new_node
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![n, new_node],
        )
        .unwrap();
    // run bubble sort n times
    builder
        .add_operation(
            BuilderOpLike::FromOperationId(n_times_op_id),
            vec![p0, new_node],
        )
        .unwrap();
    // remove new_node
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::DeleteNode),
            vec![new_node],
        )
        .unwrap();
    // decrement n
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddInteger(-1)),
            vec![n],
        )
        .unwrap();
    // recurse on self
    builder
        .add_operation(BuilderOpLike::Recurse, vec![p0, n])
        .unwrap();

    builder.build(op_id).unwrap()
}

fn wrap_main_bubble_sort_op(
    op_id: OperationId,
    main_bubble_sort_op_id: OperationId,
    list_len_op_id: OperationId,
    op_ctx: &OperationContext<TestSemantics>,
) -> UserDefinedOperation<TestSemantics> {
    let mut builder = OperationBuilder::new(op_ctx);
    // first node
    builder
        .expect_parameter_node("p0", NodeType::Integer)
        .unwrap();
    let p0 = AbstractNodeId::param("p0");
    // output node
    builder
        .add_named_operation(
            "new".into(),
            BuilderOpLike::Builtin(TestOperation::AddNode {
                node_type: NodeType::Integer,
                value: NodeValue::Integer(0),
            }),
            vec![],
        )
        .unwrap();
    let output_node = AbstractNodeId::dynamic_output("new", "new");
    // get list len
    builder
        .add_operation(
            BuilderOpLike::FromOperationId(list_len_op_id),
            vec![p0, output_node],
        )
        .unwrap();

    // only continue if list len is greater than 0
    // actually: this makes no sense - list len is obviously greater than 0 since we have at least one node
    builder
        .start_query(
            TestQuery::ValueEqualTo(NodeValue::Integer(0)),
            vec![output_node],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();

    // we want to run it len-1 times
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::AddInteger(-1)),
            vec![output_node],
        )
        .unwrap();
    // run main bubble sort operation
    builder
        .add_operation(
            BuilderOpLike::FromOperationId(main_bubble_sort_op_id),
            vec![p0, output_node],
        )
        .unwrap();
    // remove output node
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::DeleteNode),
            vec![output_node],
        )
        .unwrap();

    builder.build(op_id).unwrap()
}

#[test]
fn bubble_sort() {
    let mut op_ctx = OperationContext::new();
    let bubble_sort_op_id = 0;
    let bubble_sort = bubble_sort_op(bubble_sort_op_id);
    op_ctx.add_custom_operation(bubble_sort_op_id, bubble_sort);
    let bubble_sort_op_id_2 = 1;
    let bubble_sort_2 = bubble_sort_op_2(bubble_sort_op_id_2);
    op_ctx.add_custom_operation(bubble_sort_op_id_2, bubble_sort_2);
    let list_len_op_id = 2;
    let list_len_op = list_len_op(list_len_op_id);
    op_ctx.add_custom_operation(list_len_op_id, list_len_op);

    let bubble_sort_n_times_op_id = 3;
    let bubble_sort_n_times_op = bubble_sort_n_times_op(bubble_sort_n_times_op_id);
    op_ctx.add_custom_operation(bubble_sort_n_times_op_id, bubble_sort_n_times_op);

    let main_bubble_sort_op_id = 4;
    let main_bubble_sort_op =
        main_bubble_sort_op(main_bubble_sort_op_id, bubble_sort_n_times_op_id, &op_ctx);
    op_ctx.add_custom_operation(main_bubble_sort_op_id, main_bubble_sort_op);

    let wrap_main_bubble_sort_op_id = 5;
    let wrap_main_bubble_sort_op = wrap_main_bubble_sort_op(
        wrap_main_bubble_sort_op_id,
        main_bubble_sort_op_id,
        list_len_op_id,
        &op_ctx,
    );
    op_ctx.add_custom_operation(wrap_main_bubble_sort_op_id, wrap_main_bubble_sort_op);

    let mut g = TestSemantics::new_concrete_graph();
    // construct 5, 3, 4, 1
    let e0 = g.add_node(NodeValue::Integer(5));
    let e1 = g.add_node(NodeValue::Integer(3));
    let e2 = g.add_node(NodeValue::Integer(4));
    let e3 = g.add_node(NodeValue::Integer(1));
    g.add_edge(e0, e1, "".to_string());
    g.add_edge(e1, e2, "".to_string());
    g.add_edge(e2, e3, "".to_string());
    let initial_g_clone = g.clone();
    // add bubble sort operation
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, bubble_sort_op_id, &[e0]).unwrap();
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, bubble_sort_op_id, &[e0]).unwrap();
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, bubble_sort_op_id, &[e0]).unwrap();
    eprintln!("{}", g.dot());

    eprintln!(" --- op 2: ----");
    let mut g = initial_g_clone.clone();
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, bubble_sort_op_id_2, &[e3]).unwrap();
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, bubble_sort_op_id_2, &[e3]).unwrap();
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, bubble_sort_op_id_2, &[e3]).unwrap();
    eprintln!("{}", g.dot());

    eprintln!(" --- list len: ----");
    let mut g = initial_g_clone.clone();
    let output_node = g.add_node(NodeValue::Integer(0));
    run_from_concrete(&mut g, &op_ctx, list_len_op_id, &[e0, output_node]).unwrap();
    eprintln!("{}", g.dot());
    // run main op
    eprintln!("main bubble sort:");
    run_from_concrete(&mut g, &op_ctx, main_bubble_sort_op_id, &[e0, output_node]).unwrap();
    eprintln!("{}", g.dot());

    eprintln!(" --- wrap main bubble sort: --- ");
    let mut g = initial_g_clone.clone();
    eprintln!("{}", g.dot());
    run_from_concrete(&mut g, &op_ctx, wrap_main_bubble_sort_op_id, &[e0]).unwrap();
    eprintln!("{}", g.dot());

    // get values and check that they're sorted
    let v0 = g.get_node_attr(e0).unwrap().must_integer();
    let v1 = g.get_node_attr(e1).unwrap().must_integer();
    let v2 = g.get_node_attr(e2).unwrap().must_integer();
    let v3 = g.get_node_attr(e3).unwrap().must_integer();
    assert!(vec![v0, v1, v2, v3].is_sorted())
}

fn get_op_ctx_with_bubble_sort_for_proptest() -> (OperationContext<TestSemantics>, OperationId) {
    let mut op_ctx = OperationContext::new();
    let bubble_sort_n_times_op_id = 0;
    let bubble_sort_n_times_op = bubble_sort_n_times_op(bubble_sort_n_times_op_id);
    op_ctx.add_custom_operation(bubble_sort_n_times_op_id, bubble_sort_n_times_op);

    let list_len_op_id = 1;
    let list_len_op = list_len_op(list_len_op_id);
    op_ctx.add_custom_operation(list_len_op_id, list_len_op);

    let main_bubble_sort_op_id = 2;
    let main_bubble_sort_op =
        main_bubble_sort_op(main_bubble_sort_op_id, bubble_sort_n_times_op_id, &op_ctx);
    op_ctx.add_custom_operation(main_bubble_sort_op_id, main_bubble_sort_op);

    let wrap_main_bubble_sort_op_id = 3;
    let wrap_main_bubble_sort_op = wrap_main_bubble_sort_op(
        wrap_main_bubble_sort_op_id,
        main_bubble_sort_op_id,
        list_len_op_id,
        &op_ctx,
    );
    op_ctx.add_custom_operation(wrap_main_bubble_sort_op_id, wrap_main_bubble_sort_op);

    (op_ctx, wrap_main_bubble_sort_op_id)
}

fn sort_using_grabapl(
    values: &[i32],
    op_ctx: &OperationContext<TestSemantics>,
    bubble_sort_op_id: OperationId,
) -> Vec<i32> {
    let mut node_keys_ordered = vec![];
    let mut g = TestSemantics::new_concrete_graph();
    // add nodes
    for &v in values {
        let node_key = g.add_node(NodeValue::Integer(v));
        node_keys_ordered.push(node_key);
    }
    // add edges
    for i in 0..node_keys_ordered.len() - 1 {
        g.add_edge(
            node_keys_ordered[i],
            node_keys_ordered[i + 1],
            "".to_string(),
        );
    }
    // run the operation
    run_from_concrete(&mut g, op_ctx, bubble_sort_op_id, &[node_keys_ordered[0]]).unwrap();
    // get values and check that they're sorted
    let mut sorted_values = vec![];
    for node_key in node_keys_ordered {
        let value = g.get_node_attr(node_key).unwrap().must_integer();
        sorted_values.push(value);
    }

    sorted_values
}

// do a proptest with this

proptest! {
    // sample from random i32 vecs
    #[test]
    fn bubble_sort_proptest(input in proptest::collection::vec(proptest::num::i32::ANY, 1..=5)) {
        let (op_ctx, bubble_sort_op_id) = get_op_ctx_with_bubble_sort_for_proptest();
        // sort using grabapl
        let grabapl_sorted = sort_using_grabapl(&input, &op_ctx, bubble_sort_op_id);
        // sort using std
        let mut std_sorted = input.clone();
        std_sorted.sort_unstable();
        assert_eq!(grabapl_sorted, std_sorted, "grabapl sorting did not match std sorting for input: {:?}, grabapl_sorted: {:?}, std_sorted: {:?}", input, grabapl_sorted, std_sorted);
    }
}
