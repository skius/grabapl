use grabapl::prelude::*;
mod util;
use util::semantics::*;
use test_log::test;

fn list_to_value_vec(graph: &ConcreteGraph<TestSemantics>, head: NodeKey) -> Vec<NodeValue> {
    let mut values = vec![];
    let mut current = Some(head);
    while let Some(current_key) = current.take() {
        let val = graph.get_node_attr(current_key).unwrap();
        values.push(val.clone());

        // get next node in the list, if one exists
        let mut out_nodes_current = graph.out_edges(current_key);
        if let Some((next_node, _)) = out_nodes_current.next() {
            current = Some(next_node);
        }
    }
    values
}

fn imagined_syntax() {
    syntax::grabapl_parse!(TestSemantics,
        // -------- DFS ---------
        fn dfs(start_node: Integer) -> (head: Integer) {
            let! head = add_node<int,0>();
            copy_value_from_to(start_node, head);
            mark<"visited">(start_node);
            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                dfs_helper<"visited">(child, head);
            }

            return (head: head);
        }

        fn dfs_helper<color>(child: Integer, head: Integer) [
            parent: Integer,
            parent -> child: *,
        ] {
            // mark self as visited
            mark<color>(child);
            // insert self
            list_insert_by_copy(head, child);
            // then go to our children
            if shape [
                grandchild: Integer,
                child -> grandchild: *,
            ] {
                dfs_helper<color>(grandchild, head);
            }
            // then go to our siblings
            // problem is, here we lost the function stack 'visited' marker from above, so if a sibling has the same descendant as us,
            // we will visit it again.
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] {
                dfs_helper<color>(sibling, head);
            }
        }


        // ------ BFS --------

        fn bfs(start_node: Integer) -> (head: Integer) {
            let! head = add_node<int,0>();
            copy_value_from_to(start_node, head);

            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                bfs_helper(child, head);
            }

            return (head: head);
        }

        fn bfs_helper(child: Integer, head: Integer) [
            parent: Integer,
            parent -> child: *,
        ] {
            // insert self, then go to parent sibling
            list_insert_by_copy(head, child);
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] {
                bfs_helper(sibling, head);
            }
            // done inserting siblings, can go to child
            if shape [
                grandchild: Integer,
                child -> grandchild: *,
            ] {
                bfs_helper(grandchild, head);
            }
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

    );
}

#[test]
fn bfs_and_dfs() {
    let (op_ctx, fn_map) = syntax::grabapl_parse!(TestSemantics,
        // -------- DFS ---------
        fn dfs(start_node: Integer) -> (head: Integer) {
            let! head = add_node<int,0>();
            copy_value_from_to(start_node, head);

            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                dfs_helper(child, head);
            }

            return (head: head);
        }

        fn dfs_helper(child: Integer, head: Integer) [
            parent: Integer,
            parent -> child: *,
        ] {
            // insert self
            list_insert_by_copy(head, child);
            // then go to our children
            if shape [
                grandchild: Integer,
                child -> grandchild: *,
            ] {
                dfs_helper(grandchild, head);
            }
            // then go to our siblings
            // problem is, here we lost the function stack 'visited' marker from above, so if a sibling has the same descendant as us,
            // we will visit it again.
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] {
                dfs_helper(sibling, head);
            }
        }


        // ------ BFS --------

        fn bfs(start_node: Integer) -> (head: Integer) {
            let! head = add_node<int,0>();
            copy_value_from_to(start_node, head);

            if shape [
                child: Integer,
                start_node -> child: *,
            ] {
                bfs_helper(child, head);
            }

            return (head: head);
        }

        fn bfs_helper(child: Integer, head: Integer) [
            parent: Integer,
            parent -> child: *,
        ] {
            // insert self, then go to parent sibling
            list_insert_by_copy(head, child);
            if shape [
                sibling: Integer,
                parent -> sibling: *,
            ] {
                bfs_helper(sibling, head);
            }
            // done inserting siblings, can go to child
            if shape [
                grandchild: Integer,
                child -> grandchild: *,
            ] {
                bfs_helper(grandchild, head);
            }
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

    );

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

    let gen_vec = |g: &ConcreteGraph<TestSemantics>, nodes: &[NodeKey]| {
        nodes.iter().map(|&n| g.get_node_attr(n).unwrap().clone()).collect::<Vec<_>>()
    };

    let acceptable_bfs = vec![
        gen_vec(&g, &[l1, l2_1, l2_2, l3_1, l3_2, l4, l5]),
        gen_vec(&g, &[l1, l2_1, l2_2, l3_2, l3_1, l4, l5]),
        gen_vec(&g, &[l1, l2_2, l2_1, l3_1, l3_2, l4, l5]),
        gen_vec(&g, &[l1, l2_2, l2_1, l3_2, l3_1, l4, l5]),
    ];

    let acceptable_dfs = vec![
        gen_vec(&g, &[l1, l2_1, l3_1, l4, l5, l3_2, l2_2]),
        gen_vec(&g, &[l1, l2_2, l3_2, l2_1, l3_1, l4, l5]),
    ];

    let mut bfs = petgraph::visit::Bfs::new(g.inner_graph(), l1);

    let mut bfs_nodes = vec![];
    while let Some(node) = bfs.next(&g.inner_graph()) {
        let val = g.get_node_attr(node).unwrap();
        // bfs_nodes.push((node, val.clone()));
        bfs_nodes.push(val.clone());
    }
    assert!(acceptable_bfs.contains(&bfs_nodes), "petgraph BFS result does not match any of the acceptable results");
    println!("petgraph BFS: {:?}", bfs_nodes);

    let mut dfs = petgraph::visit::Dfs::new(g.inner_graph(), l1);
    let mut dfs_nodes = vec![];
    while let Some(node) = dfs.next(&g.inner_graph()) {
        let val = g.get_node_attr(node).unwrap();
        // dfs_nodes.push((node, val.clone()));
        dfs_nodes.push(val.clone());
    }
    assert!(acceptable_dfs.contains(&dfs_nodes), "petgraph DFS result does not match any of the acceptable results");
    println!("petgraph DFS: {:?}", dfs_nodes);

    let res = run_from_concrete(&mut g, &op_ctx, fn_map["bfs"], &[l1]).unwrap();
    let head_bfs = res.new_nodes[&"head".into()];
    let grabapl_bfs_list = list_to_value_vec(&g, head_bfs);
    println!("grabapl  BFS: {:?}", grabapl_bfs_list);
    // assert!(acceptable_bfs.contains(&grabapl_bfs_list), "grabapl BFS result does not match any of the acceptable results");

    let res = run_from_concrete(&mut g, &op_ctx, fn_map["dfs"], &[l1]).unwrap();
    let head_dfs = res.new_nodes[&"head".into()];
    let grabapl_dfs_list = list_to_value_vec(&g, head_dfs);
    println!("grabapl  DFS: {:?}", grabapl_dfs_list);
    // assert!(acceptable_dfs.contains(&grabapl_dfs_list), "grabapl DFS result does not match any of the acceptable results");

    println!("{}", g.dot());

    assert!(false);
}