use grabapl::graph::GraphTrait;
use grabapl::operation::signature::parameter::AbstractOutputNodeMarker;
use grabapl::prelude::{ConcreteGraph, run_from_concrete};
use grabapl::semantics::example::{ExampleSemantics as TestSemantics, NodeValue};
use grabapl::{NodeKey, Semantics};
use proptest::proptest;
use proptest::test_runner::Config;
use std::collections::BinaryHeap;

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
            assert!(
                parent_val >= val,
                "Max heap property violated: parent value {} is not greater than or equal to child value {}",
                parent_val,
                val
            );
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
fn proptest_max_heap_remove() {
    let file_path = "syntax-examples/max_heap_remove.gbpl";
    let src = std::fs::read_to_string(file_path).unwrap();

    let (op_ctx, fn_map) = syntax::parse_to_op_ctx_and_map::<TestSemantics>(&src);
    let max_heap_remove_id = fn_map.get("max_heap_remove").copied().unwrap();

    // eprintln!(
    //     "serialized_op_ctx:\n{}",
    //     serde_json::to_string_pretty(&op_ctx).unwrap()
    // );

    proptest!(
        Config::with_cases(10),
        |(values in proptest::collection::vec(0..5000, 0..=10))| {
            let mut expected_return_order: Vec<i32> = values.clone();
            expected_return_order.sort_unstable_by(|a, b| b.cmp(a)); // sort in descending order
            // create a max-heap from the values
            let (mut g, sentinel) = mk_heap_from_values(&values);

            // log_crate::info!("Heap created:\n{}", g.dot());

            for expected_max_value in expected_return_order {
                // run the max-heap removal operation
                let op_result = run_from_concrete(&mut g, &op_ctx, max_heap_remove_id, &[sentinel]).unwrap();
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
        }
    );
}
