use grabapl::prelude::*;
use proptest::bool::weighted;
use proptest::prelude::*;
use proptest::proptest;
use proptest::test_runner::Config;
use std::collections::{HashMap, HashSet};

mod util;
use test_log::test;
use grabapl::semantics::example_with_ref::{*, ExampleWithRefSemantics as TestSemantics};
use util::semantics::helpers;
use util::shrink_outer_first_extension::StrategyOutsideFirstExtension;

fn get_ops() -> (
    OperationContext<TestSemantics>,
    HashMap<&'static str, OperationId>,
) {
    syntax::grabapl_parse!(TestSemantics,
        // -------- BFS with Queue --------
        /*
        Idea is:
        1. push all siblings to queue.
        2.
        */

        fn bfs(start_node: Integer) -> (head: Object) {
            let! head = mk_list();
            list_insert_by_copy(head, start_node);
            let! queue = mk_queue();
            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                insert_siblings_to_queue_as_ref(child, queue);
            }
            add_values_to_list(queue, head);
            return (head: head);
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

        fn insert_siblings_to_queue_as_ref(child: Integer, queue: Object) [parent: Integer, parent->child:*] {
            // insert self
            let! self_ref = make_ref(child);
            push_queue_by_copy(queue, self_ref);
            remove_node(self_ref);
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] {
                // insert all siblings
                insert_siblings_to_queue_as_ref(sibling, queue);
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

        fn pop_queue(head: Object) -> (value: Ref<Int>) {
            // remove the first element from the queue
            let! res_src = add_node<int,-9999>();
            // if we don't match any children, we need some form of base-case result. we just create a dangling reference here.
            let! res = make_ref(res_src);
            remove_node(res_src);
            if shape [
                fst: Ref<Int>,
                snd: Ref<Int>,
                head -> fst: *,
                fst -> snd: *,
            ] {
                // remove the edge from head to fst and fst to snd
                remove_edge(head, fst);
                remove_edge(fst, snd);
                add_edge<"queue_next">(head, snd);
                // return fst
                copy_value_from_to(fst, res);
            } else if shape [
                fst: Ref<Int>,
                head -> fst: *
            ] {
                remove_edge(head, fst);
                copy_value_from_to(fst, res);
            } else {

            }
            return (value: res);
        }

        fn push_queue_by_copy(head: Object, value: Ref<Int>) {
            // insert value at the end of the queue
            let! new_node = add_node<int,0>();
            copy_value_from_to(value, new_node);
            // TODO: this block could be avoided if Ref were subtypes of Object.
            if shape [
                next: Ref<Int>,
                head -> next: *,
            ] {
                add_node<int,2000>();

                push_queue_helper_linking(next, new_node);
            } else {
                // we're at the tail of the queue
                add_node<int,1000>();
                add_edge<"queue_next">(head, new_node);
            }
        }

        // links the given node to the end of the queue.
        fn push_queue_helper_linking(curr: Ref<Int>, node_to_insert: Ref<Int>) {

            if shape [
                next: Ref<Int>,
                curr -> next: *,
            ] {
                push_queue_helper_linking(next, node_to_insert);
            } else {
                // we're at the tail of the queue
                add_edge<"queue_next">(curr, node_to_insert);
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

    )
}

type BfsLayers = Vec<Vec<NodeValue>>;

/// Returns a vec where vec[i] contains all nodes that are at distance i from the start_node. vec[0] contains the start_node itself.
fn bfs_layers(g: &ConcreteGraph<TestSemantics>, start_node: NodeKey) -> BfsLayers {
    let mut layers = vec![];
    let mut visited = HashSet::new();
    visited.insert(start_node);
    let start_node_value = g.get_node_attr(start_node).unwrap().clone();
    let current_layer = HashSet::from([(start_node, start_node_value)]);
    layers.push(current_layer);
    loop {
        let mut next_layer = HashSet::new();
        for (node, _) in layers.last().unwrap() {
            for (neighbor, _) in g.out_edges(*node) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let neighbor_value = g.get_node_attr(neighbor).unwrap().clone();
                    next_layer.insert((neighbor, neighbor_value));
                }
            }
        }
        if !next_layer.is_empty() {
            layers.push(next_layer);
        } else {
            break; // no more nodes to visit
        }
    }
    layers
        .into_iter()
        .map(|layer| {
            layer
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn valid_bfs_order(bfs_order: &[NodeValue], mut bfs_layers: BfsLayers) -> bool {
    let total_bfs_nodes: usize = bfs_layers.iter().map(|layer| layer.len()).sum();
    if bfs_order.len() != total_bfs_nodes {
        return false; // the BFS order must contain all nodes in the layers
    }
    let mut bfs_order_iter = bfs_order.iter();
    let mut curr_layer_index = 0;
    while curr_layer_index < bfs_layers.len() {
        let current_layer = &mut bfs_layers[curr_layer_index];
        if current_layer.is_empty() {
            // advance to the next layer
            curr_layer_index += 1;
            continue;
        }

        // if it's not empty, the next element in the bfs_order must be in the current layer
        let node_value = bfs_order_iter.next().unwrap();
        let Some(index) = current_layer.iter().position(|v| v == node_value) else {
            return false; // the node is not in the current layer
        };
        // remove the node from the current layer
        current_layer.remove(index);
    }
    true
}

fn test_bfs(
    op_ctx: &OperationContext<TestSemantics>,
    fn_map: &HashMap<&'static str, OperationId>,
    g: &mut ConcreteGraph<TestSemantics>,
    start_node: NodeKey,
) {
    let bfs_layers = bfs_layers(g, start_node);

    // as a sanity check, check the petgraph BFS
    {
        let mut bfs = petgraph::visit::Bfs::new(g.inner_graph(), start_node);
        let mut bfs_nodes = vec![];
        while let Some(node) = bfs.next(&g.inner_graph()) {
            let val = g.get_node_attr(node).unwrap();
            bfs_nodes.push(val.clone());
        }
        assert!(
            valid_bfs_order(&bfs_nodes, bfs_layers.clone()),
            "petgraph BFS result does not match the BFS layers"
        );
    }

    let res = run_from_concrete(g, &op_ctx, fn_map["bfs"], &[start_node]).unwrap();
    let head_bfs = res.new_nodes[&"head".into()];
    let grabapl_bfs_list = helpers::list_to_value_vec_generic::<TestSemantics>(g, head_bfs);
    let grabapl_bfs_list = &grabapl_bfs_list[1..]; // skip the sentinel node
    assert!(
        valid_bfs_order(&grabapl_bfs_list, bfs_layers.clone()),
        "grabapl BFS result does not match the BFS layers for start_node {:?},
        expected layers: {:?},
        got: {:?}
        final dot:\n{}",
        start_node,
        bfs_layers,
        grabapl_bfs_list,
        g.dot(),
    );
}

// TODO: when adding all siblings to a list, add test with a second parent that breaks in algot

#[test_log::test]
fn diamond_shape_bfs() {
    let (op_ctx, fn_map) = get_ops();
    let mut g = TestSemantics::new_concrete_graph();
    // build a diamond shape graph
    let n0 = g.add_node(NodeValue::Integer(0));
    let n1 = g.add_node(NodeValue::Integer(1));
    let n2 = g.add_node(NodeValue::Integer(2));
    let n3 = g.add_node(NodeValue::Integer(3));

    g.add_edge(n0, n1, "edge".to_string());
    g.add_edge(n0, n2, "edge".to_string());
    g.add_edge(n1, n3, "edge".to_string());
    g.add_edge(n2, n3, "edge".to_string());

    // run BFS from n0
    test_bfs(&op_ctx, &fn_map, &mut g, n0);
}

#[test_log::test]
fn all_siblings_test() {
    let (op_ctx, fn_map) = get_ops();
    let mut g = TestSemantics::new_concrete_graph();
    let op = fn_map["test_insert_all_siblings_of"];
    // build a simple graph with siblings
    // add siblings to list
    let c1 = g.add_node(NodeValue::Integer(1));
    // p1 is the parent that would break the algorithm in Algot's semantics
    let p1 = g.add_node(NodeValue::Integer(-1));
    g.add_edge(p1, c1, "edge".to_string());
    // p2 is the parent of which we want to add all siblings to a list
    let p2 = g.add_node(NodeValue::Integer(-2));
    g.add_edge(p2, c1, "edge".to_string());
    let c2 = g.add_node(NodeValue::Integer(2));
    g.add_edge(p2, c2, "edge".to_string());

    let res = run_from_concrete(&mut g, &op_ctx, op, &[p2]).unwrap();
    let head = res.new_nodes[&"head".into()];
    let siblings_list = helpers::list_to_value_vec_generic::<TestSemantics>(&g, head);
    assert_eq!(
        siblings_list,
        vec![
            NodeValue::Integer(0), /*list head sentinel*/
            NodeValue::Integer(1),
            NodeValue::Integer(2)
        ],
        "Expected siblings list to contain 1 and 2, got: {:?}",
        siblings_list
    );
}

#[test_log::test]
#[test_log(default_log_filter = "warn")]
fn proptest_bfs() {
    // Generate a random graph and test BFS on it
    let (op_ctx, fn_map) = get_ops();

    proptest!(
        Config { cases: 10, max_shrink_iters: 100, ..Config::default() },
        |((node_vals, edge_gen) in proptest::collection::vec(any::<i32>(), 0..=10).proptest_flat_map_outside_first(|nodes| {
            // directed edge count
            let node_count = nodes.len();
            let edges = node_count * node_count - node_count;

            (Just(nodes), proptest::collection::vec(weighted(0.2), edges..=edges))
        }))| {
            let mut g = TestSemantics::new_concrete_graph();
            let mut node_keys = vec![];
            for node_val in node_vals {
                let key = g.add_node(NodeValue::Integer(node_val));
                node_keys.push(key);
            }
            let mut edge_gen_iter = edge_gen.iter();
            for src in &node_keys {
                for dst in &node_keys {
                    if src != dst && *edge_gen_iter.next().unwrap() {
                        g.add_edge(*src, *dst, "irrelevant".to_string());
                    }
                }
            }
            println!("Generated graph:\n{}", g.dot());

            // run bfs on every node and check if the BFS order is valid
            for start in node_keys {
                test_bfs(&op_ctx, &fn_map, &mut g, start);
            }

            // assert!(false);
        }
    )
}
