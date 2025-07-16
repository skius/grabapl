use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use proptest::proptest;
use proptest::test_runner::Config;
use grabapl::graph::GraphTrait;
use grabapl::operation::builder::stack_based_builder::OperationBuilder2;
use grabapl::operation::signature::parameter::AbstractOutputNodeMarker;
use grabapl::prelude::*;
use super::semantics::*;

const MAX_HEAP_REMOVE_ID: OperationId = 0;
const MAX_HEAP_REMOVE_HELPER_ID: OperationId = 1;

/// Returns an operation that solves "Task 2" from the OSF tasks:
///
/// Max Heap Removal
/// The function f should take as input the root note of a max-heap, and
/// it should return the maximum of the heap (root node), and
/// then restore the heap condition.
/// Reminder: A maximum heap is a binary tree in which the number value of each node is greater than the number value of its children, and
/// each node in the tree is a maximum heap itself.
fn populate_max_heap_remove_op(op_ctx: &mut OperationContext<TestSemantics>) {
    // first we need to have the helper
    populate_max_heap_remove_helper_op(op_ctx);

    // Our max heap has a sentinel node, which points to the root of the heap.
    // The main entry point operation takes the sentinel node as input,
    // creates a new node for the returned maximum value,
    // checks if the heap is empty, and if so, returns -1,
    // otherwise it calls the helper operation which takes the root of the current heap and the
    // out-param for the max value.



    let mut builder = OperationBuilder::new(&op_ctx, MAX_HEAP_REMOVE_ID);
    builder
        .expect_parameter_node("sentinel", NodeType::Object)
        .unwrap();
    let sentinel = AbstractNodeId::param("sentinel");
    // create a new node for the max value
    builder
        .add_named_operation(
            "max_value".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let max_value = AbstractNodeId::dynamic_output("max_value", "new");
    // check if the heap is empty
    builder.start_shape_query("q").unwrap();
    builder.expect_shape_node("root".into(), NodeType::Integer).unwrap();
    let root_aid = AbstractNodeId::dynamic_output("q", "root");
    builder.expect_shape_edge(sentinel, root_aid, EdgeType::Wildcard).unwrap();
    builder.enter_false_branch().unwrap();
    // if we don't have a child, return -1.
    // this is the value we already have
    builder.enter_true_branch().unwrap();
    // we have a child.
    builder.add_operation(BuilderOpLike::FromOperationId(MAX_HEAP_REMOVE_HELPER_ID), vec![root_aid, max_value]).unwrap();
    builder.end_query().unwrap();
    builder.return_node(max_value, "max_value".into(), NodeType::Integer).unwrap();



    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(MAX_HEAP_REMOVE_ID, op);
}

fn populate_max_heap_remove_helper_op(op_ctx: &mut OperationContext<TestSemantics>) {
    let mut builder = OperationBuilder::new(&op_ctx, MAX_HEAP_REMOVE_HELPER_ID);
    builder.expect_parameter_node("root", NodeType::Integer).unwrap();
    let root = AbstractNodeId::param("root");
    builder.expect_parameter_node("max_value", NodeType::Integer).unwrap();
    let max_value = AbstractNodeId::param("max_value");
    // we return value of the root node.
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::CopyValueFromTo), vec![root, max_value]).unwrap();
    // now, to remove the node and restore the heap condition,
    // we check the following cases:
    // if root has two children, recurse on the larger child, get the max value from there, copy that to root.
    // if the root has one child, recurse on that child, get the max value from there, copy that to root.
    // if the root has no children, we can delete root.

    builder.start_shape_query("q").unwrap();
    builder.expect_shape_node("left".into(), NodeType::Integer).unwrap();
    let left_aid = AbstractNodeId::dynamic_output("q", "left");
    builder.expect_shape_edge(root, left_aid, EdgeType::Wildcard).unwrap();
    builder.expect_shape_node("right".into(), NodeType::Integer).unwrap();
    let right_aid = AbstractNodeId::dynamic_output("q", "right");
    builder.expect_shape_edge(root, right_aid, EdgeType::Wildcard).unwrap();
    builder.enter_true_branch().unwrap();
    // we have two children. Check which is larger
    builder.start_query(TestQuery::CmpFstSnd(Ordering::Greater.into()), vec![left_aid, right_aid]).unwrap();
    builder.enter_true_branch().unwrap();
    // if left > right, recurse on left
    // get a new result node for the max value
    // TODO: make temp node
    builder
        .add_named_operation(
            "temp_max".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let temp_max = AbstractNodeId::dynamic_output("temp_max", "new");
    builder.add_operation(BuilderOpLike::Recurse, vec![left_aid, temp_max]).unwrap();
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::CopyValueFromTo), vec![temp_max, root]).unwrap();
    // and delete the temp node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object
            }),
            vec![temp_max],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();
    // if left <= right, recurse on right
    // TODO: make temp node
    builder
        .add_named_operation(
            "temp_max".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let temp_max = AbstractNodeId::dynamic_output("temp_max", "new");
    builder.add_operation(BuilderOpLike::Recurse, vec![right_aid, temp_max]).unwrap();
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::CopyValueFromTo), vec![temp_max, root]).unwrap();
    // and delete the temp node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object
            }),
            vec![temp_max],
        )
        .unwrap();
    builder.end_query().unwrap();
    builder.enter_false_branch().unwrap();
    // If we don't have two children, check if we have one child.
    builder.start_shape_query("q").unwrap();
    builder.expect_shape_node("child".into(), NodeType::Integer).unwrap();
    let child_aid = AbstractNodeId::dynamic_output("q", "child");
    builder.expect_shape_edge(root, child_aid, EdgeType::Wildcard).unwrap();
    builder.enter_true_branch().unwrap();
    // we have one child, recurse on it
    // TODO: make temp node
    builder
        .add_named_operation(
            "temp_max".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let temp_max = AbstractNodeId::dynamic_output("temp_max", "new");
    builder.add_operation(BuilderOpLike::Recurse, vec![child_aid, temp_max]).unwrap();
    builder.add_operation(BuilderOpLike::Builtin(TestOperation::CopyValueFromTo), vec![temp_max, root]).unwrap();
    // and delete the temp node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object
            }),
            vec![temp_max],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();
    // if we don't have a child, we can delete the root node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object
            }),
            vec![root],
        )
        .unwrap();
    builder.end_query().unwrap();
    builder.end_query().unwrap();

    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(MAX_HEAP_REMOVE_HELPER_ID, op);
}


/// Creates a max-heap from a set of integer values and returns the sentinel node key.
fn mk_heap_from_values(values: &[i32]) -> (ConcreteGraph<TestSemantics>, NodeKey) {
    let mut g = TestSemantics::new_concrete_graph();
    let sentinel = g.add_node(NodeValue::String("sentinel".to_string()));

    let heap = BinaryHeap::from(values.to_vec());
    let mut node_vec = Vec::new();
    // note: relies on implementation detail of binaryheap. if any crater folks are reading this, sorry :( ping me or just break this
    for (i, val) in heap.iter().enumerate() {
        let node = g.add_node(NodeValue::Integer(*val));
        node_vec.push(node);
        if i > 0 {
            // add edges to the parent node
            let parent_index = (i - 1) / 2;
            let parent_node = node_vec[parent_index];
            let NodeValue::Integer(parent_val) = g.get_node_attr(parent_node).unwrap() else {
                unreachable!();
            };
            assert!(parent_val >= val, "Max heap property violated: parent value {} is not greater than or equal to child value {}", parent_val, val);
            g.add_edge(parent_node, node, "blah".to_string());
        }
    }

    // connect the sentinel to the root of the heap
    if let Some(&root) = node_vec.first() {
        g.add_edge(sentinel, root, "root".to_string());
    }

    (g, sentinel)
}

#[test_log::test]
fn proptest_max_heap_remove_heap() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    populate_max_heap_remove_op(&mut op_ctx);

    proptest!(
        Config::with_cases(10),
        |(values in proptest::collection::vec(0..5000, 0..=10))| {
        // |(values in proptest::collection::vec(0..5000, 2000..=2000))| {
            let start = std::time::Instant::now();
            let mut expected_return_order: Vec<i32> = values.clone();
            expected_return_order.sort_unstable_by(|a, b| b.cmp(a)); // sort in descending order
            log_crate::info!("Length: {:?}", values.len());
            // create a max-heap from the values
            let (mut g, sentinel) = mk_heap_from_values(&values);

            // log_crate::info!("Heap created:\n{}", g.dot());

            for expected_max_value in expected_return_order {
                // run the max-heap removal operation
                let op_result = run_from_concrete(&mut g, &op_ctx, MAX_HEAP_REMOVE_ID, &[sentinel]).unwrap();
                // check if the max value node is present
                let max_value_node = op_result.new_nodes.get(&AbstractOutputNodeMarker::from("max_value")).unwrap();
                let max_value = g.get_node_attr(*max_value_node).unwrap();
                assert_eq!(
                    max_value,
                    &NodeValue::Integer(expected_max_value),
                    "Expected max value node to have value {}, but got {:?}",
                    expected_max_value,
                    max_value
                );
            }

            // check that the heap is empty
            // TODO: graph API needs improvements to avoid this loop over all edges
            g.edges().for_each(|(src, _, _)| {
                    assert_ne!(src, sentinel, "Expected no edges from the sentinel node after all removals");
            });

            log_crate::info!("Time taken: {:?}", start.elapsed());
        }
    );
}