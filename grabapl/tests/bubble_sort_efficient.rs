mod util;

use grabapl::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::operation::run_from_concrete;
use grabapl::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::prelude::*;
use proptest::proptest;
use proptest::test_runner::Config;
use std::cmp::Ordering::Greater;
use grabapl::semantics::example_with_ref::{ExampleWithRefSemantics, NodeValue};
syntax::grabapl_defs!(get_ops, ExampleWithRefSemantics,
 // -------- BFS with Queue --------
        /*
        Idea is just like a regular BFS algorithm:
        1. Queue of unprocessed nodes.
        2. Pop an unvisited node (with magic new checked node references) from the queue
            a. Add it to the result list.
            b. Mark it as visited.
            c. Add all its children to the queue.
        3. Repeat until the queue is empty.
        */

        fn bfs(start_node: Integer) -> (head: Object) {
            let! head = mk_list();
            let! queue = mk_queue();

            // BFS queue initialization: we start with `start_node`.
            push_queue_by_ref(queue, start_node);
            // need to hide start_node from the abstract graph, since otherwise we will not be able to pop it from the queue, since it is shape-hidden.
            // (if a node is hidden from the abstract graph, that means it is *not* hidden from shape queries)
            hide_node(start_node);

            bfs_helper(queue, head);

            // the queue is not needed anymore.
            remove_node(queue);

            return (head: head);
        }

        // BFS helper: recurses until the queue is empty.
        fn bfs_helper(queue: Object, list: Object) {
            let! is_empty_res = queue_empty(queue);
            if is_eq <0>(is_empty_res) {
                // the queue is not empty.

                // let's get a reference to the first element in the queue
                // ref_node is a node reference to a node of the graph we're running BFS on. Let's call that node `next`.
                let! ref_node = pop_queue(queue);
                // we need a node to which we can attach `next`.
                let! attach = add_node<int,1>();
                // now we actually extract `next` from ref_node. this adds an edge `attach` -> `next`: "attached".
                extract_ref<int>(ref_node, attach);
                // now we need to shape query for `next`. Shape querying is necessary to ensure
                // we don't get a reference to a node that is already in the abstract graph (i.e., to avoid aliasing).
                if shape [
                    next: Int,
                    attach -> next: "attached"
                ] skipping ["visited"] {
                    // in addition, we can directly check if `next` is already visited, and if so, skip it!

                    // if it's not visited already, we add it to our BFS result list
                    list_insert_by_copy(list, next);
                    // then mark it as visited
                    mark_node<"visited", Int>(next);
                    // and lastly, we need to add all children of this node to the queue
                    //  note: if we want to a void unnecessarily adding already visited children that would just get skipped in the shape query above,
                    //  we can check _at insertion time_ if the child is already visited and skip it if so.
                    insert_children_into_queue_by_ref(next, queue);
                }
                // we do some cleanup
                remove_node(attach);
                remove_node(ref_node);

                // since the queue was not empty we try again
                bfs_helper(queue, list);
            }
            // cleanup
            remove_node(is_empty_res);
        }

        // inserts all children of the parent node into the queue.
        fn insert_children_into_queue_by_ref(parent: Integer, queue: Object) {
            if shape [
                child: Integer,
                parent -> child: *,
            ] /*skipping ["visited"] -- NOTE: uncommenting this is an optional optimization*/ {
                push_queue_by_ref(queue, child);
                // try to find more children
                insert_children_into_queue_by_ref(parent, queue);
            }
        }

        fn add_values_to_list(queue: Object, list: Object) {
            let! is_empty_res = queue_empty(queue);
            if is_eq<0>(is_empty_res) {
                // increment(list);
                // not empty.
                let! elt = pop_queue(queue);
                show_state(test);
                // new attachment point
                let! attach = add_node<int,1>();
                extract_ref<int>(elt, attach);
                if shape [
                    extracted_node: Int,
                    attach -> extracted_node: "attached"
                ] {
                    list_insert_by_copy(list, extracted_node);
                }
                remove_node(attach);
                // repeat
                add_values_to_list(queue, list);
            }
        }


        // The FIFO queue

        fn mk_queue() -> (head: Object) {
            let! head = add_node<int,0>();
            return (head: head);
        }

        // return value = 0: non-empty, >0: empty
        fn queue_empty(head: Object) -> (is_empty: Integer) {
            let! res = add_node<int,1>();
            // check if the queue is empty
            if shape [
                next: Object,
                head -> next: *,
            ] {
                // set res to false by decrementing if we have a next node
                decrement(res);
            }
            return (is_empty: res);
        }

        fn pop_queue_attach(head: Object) -> (attach: Object) {
            let! elt = pop_queue(head);
            let! attach = add_node<int,1>();
            extract_ref<int>(elt, attach);
            remove_node(elt);
            return (attach: attach);
        }

        fn pop_queue(head: Object) -> (value: Ref<Int>) {
            // remove the first element from the queue
            if shape [
                fst: Ref<Int>,
                snd: Ref<Int>,
                head -> fst: *,
                fst -> snd: *,
            ] {
                let! res = add_node<int,0>();
                // remove the edge from head to fst and fst to snd
                remove_edge(head, fst);
                remove_edge(fst, snd);
                add_edge<"queue_next">(head, snd);
                // return fst (TODO: allow returning from shape queries)
                copy_value_from_to(fst, res);
                remove_node(fst);
            } else if shape [
                fst: Ref<Int>,
                head -> fst: *
            ] {
                let! res = add_node<int,0>();
                remove_edge(head, fst);
                copy_value_from_to(fst, res);
                remove_node(fst);
            } else {
                let! res_src = add_node<int,-9999>();
                // if we don't match any children, we need some form of base-case result. we just create a dangling reference here.
                let! res = make_ref(res_src);
                remove_node(res_src);
            }
            return (value: res);
        }

        fn push_queue_by_ref(head: Object, value: Int) {
            let! ref_node = make_ref(value);
            push_queue_helper_linking(head, ref_node);
        }

        fn push_queue_by_copy(head: Object, value: Ref<Int>) {
            // insert value at the end of the queue
            let! new_node = add_node<int,0>();
            copy_value_from_to(value, new_node);
            push_queue_helper_linking(head, new_node);
        }

        // links the given node to the end of the queue.
        fn push_queue_helper_linking(curr: Object, node_to_insert: Ref<Int>) {
            if shape [
                next: Object,
                curr -> next: *,
            ] {
                push_queue_helper_linking(next, node_to_insert);
            } else {
                // we're at the tail of the queue
                add_edge<"queue_next">(curr, node_to_insert);
            }
        }


        fn all_nodes_list() -> (head: Object) {
            let! head = mk_list();
            all_nodes_list_helper(head);
            return (head: head);
        }

        fn all_nodes_list_helper(list: Object) {
            if shape [p: Int] {
                list_insert_by_copy(list, p);
                all_nodes_list_helper(list);
            }
        }


        fn mk_list() -> (head: Object) {
            let! head = add_node<int,42>();
            return (head: head);
        }

        fn list_insert_by_copy(head: Object, value: Integer) {
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


fn invalid(s: String, node: String) [x: int, s -> x: *] {
    let! int_node = add_node<int,0>();
    invalid(s, node);
    copy_value_from_to(int_node, node);
    // this makes it invalid:
    // invalid(s, node);
}

fn remove(s: Object) {
    remove_node(s);
}

fn add_edge(src: Object, dst: Object) {
    add_edge<"hello">(src, dst);
}

// Welcome! Type your Grabapl code here.
fn foo(x: Int) -> (result: Int) {
    show_state(foo_state);
    // try returning a node!
    let! new_node = add_node<int, 1>();
    return (result: new_node);
}

fn test1(p: int) [c: int, p -> c: "child"] {
    let! c2 = add_child_if_not_handle_to_exists_and_return(p);
    show_state(c2);

    let! one = add_node<int, 1>();
    copy_value_from_to(one, c);
    remove_node(one);

    let! two = add_node<int, 2>();
    copy_value_from_to(two, c2);
    remove_node(two);
}


fn add_child_if_not_handle_to_exists_and_return(p: int) -> (child: int) {
    if shape [child: int, p -> child: *] {

    } else {
        let! child = add_node<int, 0>();
        add_edge<"child">(p, child);
    }
    return (child: child);
}

    fn force_child(
        p1: object
    ) -> (child: int) {
        if shape [child: int, p1 -> child: *] {
            // child exists in this scope
        } else {
            diverge<"no child found">();
            // child does not exist in this scope, but ît diverged
        }
        // hence the child node should be visible here
        remove_node(p1);
        return (child: child);
    }

fn a_shape_test(p: int) {
        if shape [
        child: int,
        p -> child: *
    ] {
        mark_node<"visited">(child);
    }
    if shape [
        child: int,
        p -> child: *
    ] {
        // entered
        add_node<int, 0>();

        if shape [
            child: int,
            p -> child: *,
        ] skipping ["visited"] {
            // entered
            add_node<string, "node">();
            show_state(aaaa);


        }
    }
}

fn aaaaaaaaaa(blah: string) {
    add_node<string, "no2323232323de">();
}

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
    let bubble_sort_op_id = fn_map.get("bubble_sort_wrapper").unwrap().clone();

    proptest!(
        Config { cases: 10, max_shrink_iters: 100, ..Config::default() },
        // TODO: lol, any more than ~10 elements will overflow the stack. which makes sense since the O(n^2) calls are all appended to the stack.
        |(input in proptest::collection::vec(proptest::num::i32::ANY, 1..=10))| {
            // sort using grabapl
            let grabapl_sorted = sort_using_grabapl(&input, &op_ctx, bubble_sort_op_id);
            // sort using std
            let mut std_sorted = input.clone();
            std_sorted.sort_unstable();
            assert_eq!(grabapl_sorted, std_sorted, "grabapl sorting did not match std sorting for input: {:?}, grabapl_sorted: {:?}, std_sorted: {:?}", input, grabapl_sorted, std_sorted);
        }
    );
}