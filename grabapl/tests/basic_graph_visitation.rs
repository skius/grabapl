use grabapl::prelude::*;
use proptest::bool::weighted;
use proptest::prelude::*;
use proptest::proptest;
use proptest::test_runner::Config;
use std::collections::{HashMap, HashSet};

mod util;
use test_log::test;
use util::semantics::helpers;
use util::semantics::*;
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
        2. ah. this only works by copy. we cannot get a node from the queue and then find outgoing edges.
        */

        fn bfs_by_queue(start_node: Integer) -> (head: Integer) {
            let! head = mk_list();
            let! queue = mk_queue();
            copy_value_from_to(start_node, head);
            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                // bfs_by_queue_helper(start_node, head);
            }
            return (head: head);
        }


        // The FIFO queue

        fn mk_queue() -> (head: Object) {
            let! head = add_node<int,0>();
            return (head: head);
        }

        // 0: false, non-zero: true
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

        fn pop_queue(head: Object) -> (value: Integer) {
            // remove the first element from the queue
            let! res = add_node<int,-9999>();
            if shape [
                fst: Integer,
                snd: Integer,
                head -> fst: *,
                fst -> snd: *,
            ] {
                // remove the edge from head to fst and fst to snd
                remove_edge(head, fst);
                remove_edge(fst, snd);
                add_edge<"next">(head, snd);
                // return fst
                copy_value_from_to(fst, res);
            } else if shape [
                fst: Integer,
                head -> fst: *
            ] {
                remove_edge(head, fst);
                copy_value_from_to(fst, res);
            }
            return (value: res);
        }

        fn push_queue_by_copy(head: Object, value: Integer) {
            // insert value at the end of the queue
            let! new_node = add_node<int,0>();
            copy_value_from_to(value, new_node);
            push_queue_helper_linking(head, new_node);
        }

        // links the given node to the end of the queue.
        fn push_queue_helper_linking(curr: Object, node_to_insert: Integer) {
            if shape [
                next: Object,
                curr -> next: *,
            ] {
                push_queue_helper_linking(next, node_to_insert);
            } else {
                // we're at the tail of the queue
                add_edge<"next">(curr, node_to_insert);
            }
        }


        // -------- DFS ---------
        fn dfs(start_node: Integer) -> (head: Integer) {
            let! head = add_node<int,0>();
            copy_value_from_to(start_node, head);
            mark_node<"visited", Object>(start_node);
            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                dfs_helper(child, head);
            }
            remove_marker<"visited">();
            return (head: head);
        }

        fn dfs_helper(child: Integer, head: Integer) [
            parent: Integer,
            parent -> child: *,
        ] {
            // mark self as visited
            mark_node<"visited", Object>(child);
            // insert self
            list_insert_by_copy(head, child);
            // then go to our children
            if shape [
                grandchild: Integer,
                child -> grandchild: *,
            ] skipping ["visited"] {
                dfs_helper(grandchild, head);
            }
            // then go to our siblings
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] skipping ["visited"] {
                dfs_helper(sibling, head);
            }
        }


        // ------ BFS --------
        /*
        General idea:
        1. Start with the input node
        2. For layer 0..n call operation that first descends n times to its children,
           then iterates over all siblings there.
           // TODO: think about marker interactions. what if we could tell a shape query to not skip marked nodes?
           // then we could mark all nodes already added to the result list.
           // this is a problem though since going to a child for the next layer requires a shape query
           // hence we must tell the shape query to not skip marked nodes for everything except the last layer.
           // in the last layer, i.e., when we could accidentally have a back-edge to a earlier layer, we skip marked nodes again.
           //

        */

        fn bfs(start_node: Integer) -> (head: Integer) {
            // initialize list
            // (layer #1)
            let! head = add_node<int,0>();
            copy_value_from_to(start_node, head);

            let! max_height = max_height(start_node);
            if shape [
                initial_child: Integer,
                start_node -> initial_child: *,
            ] {
                // we wish to insert the very next layer.
                let! curr_dist = add_node<int,0>();
                // start the BFS iteration
                // since we already inserted layer #1, and we're starting with `initial_child`,
                // our max_height is actually one higher than necessary. hence we decrement it.
                decrement(max_height);
                // also, a height of N nodes means a distance of N-1 nodes to the last layer.
                // hence we decrement again.
                decrement(max_height);
                bfs_iter(start_node, head, curr_dist, max_height);
                remove_node(curr_dist);
            }
            remove_node(max_height);
            return (head: head);
        }

        // Repeat the inner call with arguments from 1 .. max_height.
        // in particular, we want proceed as follows:
        //  1. first insert the layer that's 1 away
        //  2. then insert the layer that's 2 away
        //  3. and so on...
        fn bfs_iter(start_node: Integer, head: Integer, curr_dist: Integer, max_dist_to_last_layer: Integer)
            [initial_child: Integer, start_node -> initial_child: *] {
            if cmp_fst_snd%>%(curr_dist, max_dist_to_last_layer) {
                // we've handled every distance up to the max distance, so we're done.
            } else {
                // first call the helper with curr_dist as argument
                let! layers_until_insert = add_node<int,0>();
                copy_value_from_to(curr_dist, layers_until_insert);
                bfs_insert_layer(initial_child, head, layers_until_insert);
                remove_node(layers_until_insert);
                // then increment curr_dist and call again
                increment(curr_dist);
                bfs_iter(start_node, head, curr_dist, max_dist_to_last_layer);
            }

        }

        // inserts the layer that is `layer` away from the input node. i.e., if layer is 2, we insert the node x from the chain: input -> a -> x.
        fn bfs_insert_layer(child: Integer, head: Integer, layer: Integer) [
            parent: Integer,
            parent -> child: *,
        ] {
            mark_node<"visited", Object>(child);

            if is_eq<0>(layer) {
                // if the distance is zero, we insert ourselves
                // this is an edge case for the first iteration because no backedge can exist yet.
                // if we ever have a larger distance, we need to skip visited nodes. this happens in the `is_eq<0>(layer)` check.
                bfs_insert_siblings(child, head);
            } else {
                // general case: not this layer.

                // we need to invoke our siblings as well
                if shape [
                    sibling: Integer,
                    parent -> sibling: *,
                ] {
                    bfs_insert_layer(sibling, head, layer);
                }

                // we also go down to our children.
                if is_eq<1>(layer) {
                    // if we're one before the layer-to-insert, that means we need to insert our children
                    if shape [
                        grandchild: Integer,
                        child -> grandchild: *,
                    ] skipping ["visited"] {
                        // NOTE we skip visited nodes here!
                        // if we did not skip visited nodes, we could be taking a back-edge here to a node that was already inserted from a previous layer.
                        bfs_insert_siblings(grandchild, head);
                    }
                } else if shape [grandchild: Integer, child -> grandchild: *] {
                    // if we have more layers to go, we recurse down to our children, while reducing the layer distance by one.
                    let! layer_copy = add_node<int,0>();
                    copy_value_from_to(layer, layer_copy);
                    decrement(layer_copy);
                    bfs_insert_layer(grandchild, head, layer_copy);
                    remove_node(layer_copy);
                }
            }
        }

        fn test_insert_all_siblings_of(parent: Integer) -> (head: Integer) {
            let! head = mk_list();
            if shape [
                child: Integer,
                parent -> child: *,
            ] {
                bfs_insert_siblings(child, head);
            }
            return (head: head);
        }

        fn bfs_insert_siblings(child: Integer, head: Integer) [parent: Integer, parent -> child: *] {
            mark_node<"visited", Object>(child);
            // insert self, then go to parent sibling
            list_insert_by_copy(head, child);
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] skipping ["visited"] { // NOTE: only here do we start skipping visited nodes. Similarly, this is to avoid back-edges.
                bfs_insert_siblings(sibling, head);
            }
        }

        fn max_height(start: Object) -> (max_height: Integer) {
            let! res = add_node<int,1>();
            if shape [
                child: Object,
                start -> child: *,
            ] {
                let! child_max = max_height_helper(child);
                increment(child_max);
                copy_value_from_to(child_max, res);
                remove_node(child_max);
            }
            return (max_height: res);
        }

        fn max_height_helper(child: Object) [parent: Object, parent -> child: *]
            -> (max_height: Integer) {
            let! our_height = add_node<int,1>();
            if shape [
                sibling: Object,
                parent -> sibling: *,
            ] {
                // we have a sibling, so we need to check its height too
                let! sibling_max = max_height_helper(sibling);
                set_fst_to_max(our_height, sibling_max);
                remove_node(sibling_max);
            }

            // if we have a child, recurse
            if shape [
                grandchild: Object,
                child -> grandchild: *,
            ] {
                let! child_max = max_height_helper(grandchild);
                // if our child has height child_max, we have height child_max + 1
                increment(child_max);
                set_fst_to_max(our_height, child_max);
                remove_node(child_max);
            }

            return (max_height: our_height);
        }

        fn set_fst_to_max(a: Integer, b: Integer) {
            let! max = max(a, b);
            copy_value_from_to(max, a);
            remove_node(max);
        }

        fn max(a: Integer, b: Integer) -> (max: Integer) {
            let! res = add_node<int,0>();
            if cmp_fst_snd%>%(a, b) {
                copy_value_from_to(a, res);
            } else {
                copy_value_from_to(b, res);
            }
            return (max: res);
        }

        fn mk_list() -> (head: Integer) {
            let! head = add_node<int,0>();
            return (head: head);
        }

        fn list_insert_by_copy(head: Integer, value: Integer) {
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

#[test]
fn bfs_and_dfs() {
    let (op_ctx, fn_map) = get_ops();

    let mut g = TestSemantics::new_concrete_graph();

    // build some connected graph
    let mut ordered_i = 0;
    let mut next_i = || {
        let i = ordered_i;
        ordered_i += 1;
        i
    };
    let l1 = g.add_node(NodeValue::Integer(next_i()));
    let l2_1 = g.add_node(NodeValue::Integer(next_i()));
    let l2_2 = g.add_node(NodeValue::Integer(next_i()));
    let l3_1 = g.add_node(NodeValue::Integer(next_i()));
    let l3_2 = g.add_node(NodeValue::Integer(next_i()));
    let l4 = g.add_node(NodeValue::Integer(next_i()));
    let l5 = g.add_node(NodeValue::Integer(next_i()));

    // forward links: l1 -> l2_1, l1 -> l2_2, l2_1 -> l3_1, l2_2 -> l3_2, l3_1 -> l4, l4 -> l5, l5 -> l3_2
    // backward links: l3_1 -> l1, l2_1 -> l1,

    let edge_attr = "ignored".to_string();

    g.add_edge(l1, l2_1, edge_attr.clone());
    g.add_edge(l1, l2_2, edge_attr.clone());
    g.add_edge(l2_1, l3_1, edge_attr.clone());
    g.add_edge(l2_2, l3_2, edge_attr.clone());
    g.add_edge(l3_1, l4, edge_attr.clone());
    g.add_edge(l4, l5, edge_attr.clone());
    g.add_edge(l5, l3_2, edge_attr.clone());

    g.add_edge(l3_1, l1, edge_attr.clone());
    g.add_edge(l2_1, l1, edge_attr.clone());
    g.add_edge(l5, l1, edge_attr.clone());

    let gen_vec = |g: &ConcreteGraph<TestSemantics>, nodes: &[NodeKey]| {
        nodes
            .iter()
            .map(|&n| g.get_node_attr(n).unwrap().clone())
            .collect::<Vec<_>>()
    };

    let acceptable_bfs = [
        gen_vec(&g, &[l1, l2_1, l2_2, l3_1, l3_2, l4, l5]),
        gen_vec(&g, &[l1, l2_1, l2_2, l3_2, l3_1, l4, l5]),
        gen_vec(&g, &[l1, l2_2, l2_1, l3_1, l3_2, l4, l5]),
        gen_vec(&g, &[l1, l2_2, l2_1, l3_2, l3_1, l4, l5]),
    ];

    let acceptable_dfs = [
        gen_vec(&g, &[l1, l2_1, l3_1, l4, l5, l3_2, l2_2]),
        gen_vec(&g, &[l1, l2_2, l3_2, l2_1, l3_1, l4, l5]),
    ];

    let bfs_layers = bfs_layers(&g, l1);

    let mut bfs = petgraph::visit::Bfs::new(g.inner_graph(), l1);

    let mut bfs_nodes = vec![];
    while let Some(node) = bfs.next(&g.inner_graph()) {
        let val = g.get_node_attr(node).unwrap();
        // bfs_nodes.push((node, val.clone()));
        bfs_nodes.push(val.clone());
    }
    assert!(
        acceptable_bfs.contains(&bfs_nodes),
        "petgraph BFS result does not match any of the acceptable results"
    );
    assert!(
        valid_bfs_order(&bfs_nodes, bfs_layers.clone()),
        "petgraph BFS result does not match the BFS layers"
    );
    println!("petgraph BFS: {bfs_nodes:?}");

    let mut dfs = petgraph::visit::Dfs::new(g.inner_graph(), l1);
    let mut dfs_nodes = vec![];
    while let Some(node) = dfs.next(&g.inner_graph()) {
        let val = g.get_node_attr(node).unwrap();
        // dfs_nodes.push((node, val.clone()));
        dfs_nodes.push(val.clone());
    }
    assert!(
        acceptable_dfs.contains(&dfs_nodes),
        "petgraph DFS result does not match any of the acceptable results"
    );
    println!("petgraph DFS: {dfs_nodes:?}");

    let res = run_from_concrete(&mut g, &op_ctx, fn_map["bfs"], &[l1]).unwrap();
    let head_bfs = res.key_of_output_marker("head").unwrap();
    let grabapl_bfs_list = helpers::list_to_value_vec(&g, head_bfs);
    let valid = acceptable_bfs.contains(&grabapl_bfs_list);
    println!("grabapl  BFS: {grabapl_bfs_list:?} - valid: {valid}");
    assert!(
        valid,
        "grabapl BFS result does not match any of the acceptable results"
    );
    assert!(
        valid_bfs_order(&grabapl_bfs_list, bfs_layers.clone()),
        "grabapl BFS result does not match the BFS layers"
    );

    let res = run_from_concrete(&mut g, &op_ctx, fn_map["dfs"], &[l1]).unwrap();
    let head_dfs = res.key_of_output_marker("head").unwrap();
    let grabapl_dfs_list = helpers::list_to_value_vec(&g, head_dfs);
    let valid = acceptable_dfs.contains(&grabapl_dfs_list);
    println!("grabapl  DFS: {grabapl_dfs_list:?} - valid: {valid}");
    assert!(
        valid,
        "grabapl DFS result does not match any of the acceptable results"
    );

    println!("{}", g.dot());

    let max_height_res = run_from_concrete(&mut g, &op_ctx, fn_map["max_height"], &[l1]).unwrap();
    let max_height_node = max_height_res.new_nodes()[&"max_height".into()];
    let max_height_value = g.get_node_attr(max_height_node).unwrap();
    println!("max height of the graph starting from node {l1:?}: {max_height_value:?}");

    // queue test
    let queue_head = g.add_node(NodeValue::Integer(next_i()));
    let nums = [5, 9, 10, 22, 5, 2];
    for &num in &nums {
        let new_node = g.add_node(NodeValue::Integer(num));
        run_from_concrete(
            &mut g,
            &op_ctx,
            fn_map["push_queue_by_copy"],
            &[queue_head, new_node],
        )
        .unwrap();
    }
    let mut returned_queue = vec![];
    loop {
        let is_empty_res =
            run_from_concrete(&mut g, &op_ctx, fn_map["queue_empty"], &[queue_head]).unwrap();
        let is_empty_node = is_empty_res.new_nodes()[&"is_empty".into()];
        let is_empty_value = g.get_node_attr(is_empty_node).unwrap();
        if is_empty_value.must_integer() == 1 {
            // queue is empty
            break;
        }
        let pop_res =
            run_from_concrete(&mut g, &op_ctx, fn_map["pop_queue"], &[queue_head]).unwrap();
        let popped_value_node = pop_res.new_nodes()[&"value".into()];
        let popped_value = g.get_node_attr(popped_value_node).unwrap();
        returned_queue.push(popped_value.must_integer());
    }
    assert_eq!(returned_queue, nums);

    // assert!(false);
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

    let res = run_from_concrete(g, op_ctx, fn_map["bfs"], &[start_node]).unwrap();
    let head_bfs = res.key_of_output_marker("head").unwrap();
    let grabapl_bfs_list = helpers::list_to_value_vec(g, head_bfs);
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
    let head = res.key_of_output_marker("head").unwrap();
    let siblings_list = helpers::list_to_value_vec(&g, head);
    assert_eq!(
        siblings_list,
        vec![
            NodeValue::Integer(0), /*list head sentinel*/
            NodeValue::Integer(1),
            NodeValue::Integer(2)
        ],
        "Expected siblings list to contain 1 and 2, got: {siblings_list:?}"
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
