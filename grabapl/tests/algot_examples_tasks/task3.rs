//! # Task 3: Tree Serialization
//! Binary Search Tree Serialisation
//! The function f takes as input the root of a binary search tree.
//! It should return an ordered list of the elements of the tree.
//! Reminder: A binary search tree is a tree where for each node, all the values in the left subtree are smaller, and
//! all the values in the right subtree are greater than the node's number value.

use grabapl::prelude::*;
use syntax::{grabapl_defs, grabapl_parse};
use crate::util::semantics::{NodeValue, TestSemantics, helpers};

grabapl_defs!(get_ops, TestSemantics,
    fn tree_serialize(root: Integer) -> (list: Integer) {
        let! list = mk_list();
        tree_serialize_helper(root, list);
        return (list: list);
    }

    fn tree_serialize_helper(node: Integer, list: Integer) {
        // if there's a left child, serialize it first
        if shape [
            left: Integer,
            node -> left: "left",
        ] {
            tree_serialize_helper(left, list);
        }
        // then copy the value of the current node
        list_insert_by_copy(list, node);
        // if there's a right child, serialize it next
        if shape [
            right: Integer,
            node -> right: "right",
        ] {
            tree_serialize_helper(right, list);
        }
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
);

#[test_log::test]
fn task3() {
    let (op_ctx, fn_names) = get_ops();

    let mut g = ConcreteGraph::<TestSemantics>::new();
    // build a tree of this form:
    //         5
    //        / \
    //       3   6
    //      / \   \
    //     2   4   7
    //    /
    //   1

    let n1 = g.add_node(NodeValue::Integer(1));
    let n2 = g.add_node(NodeValue::Integer(2));
    let n3 = g.add_node(NodeValue::Integer(3));
    let n4 = g.add_node(NodeValue::Integer(4));
    let n5 = g.add_node(NodeValue::Integer(5));
    let n6 = g.add_node(NodeValue::Integer(6));
    let n7 = g.add_node(NodeValue::Integer(7));

    g.add_edge(n2, n1, "left".to_string());
    g.add_edge(n3, n2, "left".to_string());
    g.add_edge(n3, n4, "right".to_string());
    g.add_edge(n5, n3, "left".to_string());
    g.add_edge(n5, n6, "right".to_string());
    g.add_edge(n6, n7, "right".to_string());

    let root = n5;
    let res = run_from_concrete(&mut g, &op_ctx, fn_names["tree_serialize"], &[root]).unwrap();
    let list = res.new_nodes[&"list".into()];
    let values_with_sentinel = helpers::list_to_value_vec(&g, list);
    let values = &values_with_sentinel[1..]; // skip the sentinel node
    assert_eq!(values, &[
        NodeValue::Integer(1),
        NodeValue::Integer(2),
        NodeValue::Integer(3),
        NodeValue::Integer(4),
        NodeValue::Integer(5),
        NodeValue::Integer(6),
        NodeValue::Integer(7),
    ], "Serialized tree does not match expected order");


}