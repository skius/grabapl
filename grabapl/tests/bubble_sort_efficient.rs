mod util;

use grabapl::operation::run_from_concrete;
use grabapl::prelude::*;
use grabapl::semantics::example_with_ref::{ExampleWithRefSemantics, NodeValue};
use proptest::proptest;
use proptest::test_runner::Config;
syntax::grabapl_defs!(get_ops, ExampleWithRefSemantics,
// Better bubble sort implementation
/*
Idea: We go down the list until the end, pulling the max element with us.
As soon as we reach the end of the list, we know for a fact that element will now stay there.
So we mark that node as visited to not enter it again.
Then we go up, pulling the min element with us, when we reach the end, we mark that as visited, for the same reason above.
We repeat until there's no more elements to process.

*/


    fn bubble_sort_wrapper(head: Int) {
        let! direction = add_node<int, 0>();
        bubble_sort(head, direction);
        remove_node(direction);
    }

fn bubble_sort(curr_elt: Int, direction: Int) {
    // direction == 0: down
    // direction == 1: up
    if is_eq<0>(direction) {
        // we go down the list, pulling the max element with us
        if shape [
            next: Int,
            curr_elt -> next: *,
        ] skipping ["fixed"] {
            if cmp_fst_snd%<%(curr_elt, next) {
                // already in order
            } else {
                // need to swap values then continue
                swap_values(curr_elt, next);
            }
            // just continue to next, while making sure to forget our node
            hide_node(curr_elt);
            bubble_sort(next, direction);
        } else {
            // we have reached the end. Since we were going down, that means `curr_elt` is now in the right position.
            mark_node<"fixed">(curr_elt);
            // now we need to check if we can go back up again
            increment(direction);
            if shape [
                prev: Int,
                prev -> curr_elt: *,
            ] skipping ["fixed"] {
                // if so, just recurse on prev
                bubble_sort(prev, direction);
            } else {
                // we're done!
            }
        }
    } else {
        // we go up the list, pulling the min element with us
        if shape [
            prev: Int,
            prev -> curr_elt: *,
        ] skipping ["fixed"] {
            if cmp_fst_snd%<%(prev, curr_elt) {
                // already in order
            } else {
                // need to swap
                swap_values(prev, curr_elt);
            }
            hide_node(curr_elt);
            bubble_sort(prev, direction);
        } else {
            // we have reached the top. hence we must be the min node and can fix ourselves.
            mark_node<"fixed">(curr_elt);
            // now we need to go back down
            decrement(direction);
            if shape [
                next: Int,
                curr_elt -> next: *,
            ] skipping ["fixed"] {
                // we can go back down, so let's recurse
                bubble_sort(next, direction);
            } else {
                // we're done!
            }
        }
    }
}

fn swap_values(a: int, b: int) {
    let! temp = add_node<int, 0>();
    copy_value_from_to(a, temp);
    copy_value_from_to(b, a);
    copy_value_from_to(temp, b);
    remove_node(temp);
}




fn hide_node(node: Object) {
    let! one = add_node<int,1>();
    if is_eq<0>(one) {
        // statically 'maybe' delete the node, but in practice this is never executed.
        remove_node(node);
    }
    remove_node(one);
}

);

fn sort_using_grabapl(
    values: &[i32],
    op_ctx: &OperationContext<ExampleWithRefSemantics>,
    bubble_sort_op_id: OperationId,
) -> Vec<i32> {
    let mut node_keys_ordered = vec![];
    let mut g = ExampleWithRefSemantics::new_concrete_graph();
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
    log_crate::info!("Graph before sorting: {}", g.dot());
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

#[test_log::test]
fn bubble_sort_proptest() {
    let (op_ctx, fn_map) = get_ops();
    let bubble_sort_op_id = *fn_map.get("bubble_sort_wrapper").unwrap();

    proptest!(
        Config { cases: 10, max_shrink_iters: 100, ..Config::default() },
        // TODO: lol, any more than ~10 elements will overflow the stack. which makes sense since the O(n^2) calls are all appended to the stack.
        |(input in proptest::collection::vec(proptest::num::i32::ANY, 1..=10))| {
            // sort using grabapl
            let grabapl_sorted = sort_using_grabapl(&input, &op_ctx, bubble_sort_op_id);
            // sort using std
            let mut std_sorted = input.clone();
            std_sorted.sort_unstable();
            assert_eq!(grabapl_sorted, std_sorted, "grabapl sorting did not match std sorting for input: {input:?}, grabapl_sorted: {grabapl_sorted:?}, std_sorted: {std_sorted:?}");
        }
    );
}
