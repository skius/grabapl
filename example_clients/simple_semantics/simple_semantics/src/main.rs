use grabapl::operation::builder::{BuilderOpLike, OperationBuilder};
use grabapl::operation::run_from_concrete;
use grabapl::operation::user_defined::{AbstractNodeId, UserDefinedOperation};
use grabapl::prelude::*;

use grabapl::graph::dot::DotCollector;
use simple_semantics::sample_user_defined_operations::{
    get_count_list_len_user_defined_operation, get_insert_bst_user_defined_operation,
    get_labeled_edges_insert_bst_user_defined_operation, get_mk_n_to_0_list_user_defined_operation,
    get_node_heights_user_defined_operation, get_sample_user_defined_operation,
};
use simple_semantics::{BuiltinOperation, BuiltinQuery, EdgePattern, SimpleSemantics};
use std::collections::HashMap;

fn insert_bst_builder_test(
    op_ctx: &OperationContext<SimpleSemantics>,
    self_op_id: OperationId,
) -> UserDefinedOperation<SimpleSemantics> {
    // OperationBuilder has an inner state enum. in fact, that is a stack referencing the current query stack it is inside.
    let mut op_builder = OperationBuilder::new(op_ctx, self_op_id);
    let show = |op_builder: &OperationBuilder<_>| {
        println!("{}\n----------", op_builder.format_state());
    };

    let root_node_marker = "root".into();
    let root_node = AbstractNodeId::ParameterMarker(root_node_marker);
    let node_to_insert_marker = "node_to_insert".into();
    let node_to_insert = AbstractNodeId::ParameterMarker(node_to_insert_marker);
    let mk_delete = |op_builder: &mut OperationBuilder<SimpleSemantics>| {
        op_builder
            .add_operation(
                BuilderOpLike::Builtin(BuiltinOperation::DeleteNode),
                vec![node_to_insert],
            )
            .unwrap();
        show(op_builder);
    };

    op_builder
        .expect_parameter_node(root_node_marker, ())
        .unwrap();
    show(&op_builder);
    op_builder
        .expect_parameter_node(node_to_insert_marker, ())
        .unwrap();
    show(&op_builder);

    // Start a query on the root node to figure out if it's -1.
    op_builder
        .start_query(BuiltinQuery::IsValueEq(-1), vec![root_node])
        .unwrap();
    show(&op_builder);
    {
        // If it is, the tree is empty and hence we set the root to be the node to insert.
        op_builder.enter_true_branch().unwrap();
        show(&op_builder);
        {
            op_builder
                .add_operation(
                    BuilderOpLike::Builtin(BuiltinOperation::CopyNodeValueTo),
                    vec![node_to_insert, root_node],
                )
                .unwrap();
            show(&op_builder);
            mk_delete(&mut op_builder);
        }
        op_builder.enter_false_branch().unwrap();
        show(&op_builder);
        {
            // If it is not, we need to insert the node into the existing tree.
            // Check if value > root
            op_builder
                .start_query(BuiltinQuery::FirstGtSnd, vec![node_to_insert, root_node])
                .unwrap();
            show(&op_builder);
            {
                op_builder.enter_true_branch().unwrap();
                show(&op_builder);
                {
                    // If it is, we need to go to the right subtree, if it exists
                    op_builder.start_shape_query("right_query").unwrap();
                    show(&op_builder);
                    // now expect what we want
                    // "child" is the node we will refer to in the "if" case. it doubles as both the identifier for the
                    // shape query, and the actual matched node.
                    let child = "child".into();
                    op_builder.expect_shape_node(child, ()).unwrap();
                    show(&op_builder);
                    let child_id = AbstractNodeId::DynamicOutputMarker("right_query".into(), child);
                    op_builder
                        .expect_shape_edge(
                            root_node,
                            child_id,
                            EdgePattern::Exact("right".to_string()),
                        )
                        .unwrap();
                    show(&op_builder);
                    {
                        op_builder.enter_true_branch().unwrap();
                        show(&op_builder);
                        {
                            // If it exists, we need to recurse into the right subtree.
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Recurse,
                                    vec![child_id, node_to_insert],
                                )
                                .unwrap();
                            show(&op_builder);
                        }
                        // if there is none, we add it as right child
                        op_builder.enter_false_branch();
                        show(&op_builder);
                        {
                            let new_node = AbstractNodeId::DynamicOutputMarker(
                                "add_node".into(),
                                "new".into(),
                            );
                            op_builder
                                .add_named_operation(
                                    "add_node".into(),
                                    BuilderOpLike::Builtin(BuiltinOperation::AddNode),
                                    vec![],
                                )
                                .unwrap();
                            show(&op_builder);
                            // TODO: in above^ show of the intermediate state, we should "see" `add_node:new` as a node with the metadata of that
                            // AbstractNodeId.
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Builtin(BuiltinOperation::CopyNodeValueTo),
                                    vec![node_to_insert, new_node],
                                )
                                .unwrap();
                            show(&op_builder);
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Builtin(BuiltinOperation::AddEdge),
                                    vec![root_node, new_node],
                                )
                                .unwrap();
                            show(&op_builder);
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Builtin(BuiltinOperation::SetEdgeValue(
                                        "right".to_string(),
                                    )),
                                    vec![root_node, new_node],
                                )
                                .unwrap();
                            show(&op_builder);
                            mk_delete(&mut op_builder);
                        }
                        op_builder.end_query().unwrap();
                        show(&op_builder);
                    }
                }

                op_builder.enter_false_branch().unwrap();
                show(&op_builder);
                {
                    // vaLue < root
                    // check if left subtree exists
                    let left_query = "left_query".into();
                    op_builder.start_shape_query(left_query).unwrap();
                    show(&op_builder);
                    let child = "child".into();
                    let child_id = AbstractNodeId::DynamicOutputMarker(left_query, child);
                    op_builder.expect_shape_node(child, ()).unwrap();
                    show(&op_builder);
                    op_builder.expect_shape_edge(
                        root_node,
                        child_id,
                        EdgePattern::Exact("left".to_string()),
                    );
                    show(&op_builder);
                    {
                        op_builder.enter_true_branch().unwrap();
                        show(&op_builder);
                        {
                            // if it exists, recurse into the left subtree
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Recurse,
                                    vec![child_id, node_to_insert],
                                )
                                .unwrap();
                            show(&op_builder);
                        }
                        // if it does not, we add it as left child
                        op_builder.enter_false_branch().unwrap();
                        show(&op_builder);
                        {
                            let new_node = AbstractNodeId::DynamicOutputMarker(
                                "add_node".into(),
                                "new".into(),
                            );
                            op_builder
                                .add_named_operation(
                                    "add_node".into(),
                                    BuilderOpLike::Builtin(BuiltinOperation::AddNode),
                                    vec![],
                                )
                                .unwrap();
                            show(&op_builder);
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Builtin(BuiltinOperation::CopyNodeValueTo),
                                    vec![node_to_insert, new_node],
                                )
                                .unwrap();
                            show(&op_builder);
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Builtin(BuiltinOperation::AddEdge),
                                    vec![root_node, new_node],
                                )
                                .unwrap();
                            show(&op_builder);
                            op_builder
                                .add_operation(
                                    BuilderOpLike::Builtin(BuiltinOperation::SetEdgeValue(
                                        "left".to_string(),
                                    )),
                                    vec![root_node, new_node],
                                )
                                .unwrap();
                            show(&op_builder);
                            mk_delete(&mut op_builder);
                        }
                        // end the query
                        op_builder.end_query().unwrap();
                        show(&op_builder);
                    }
                }
                // value > root query
                op_builder.end_query().unwrap();
                show(&op_builder);
            }
        }
        // -1 query
        op_builder.end_query().unwrap();
        show(&op_builder);
    }

    // TODO: finish operation

    op_builder.build().unwrap()
}

fn main() {
    let mut operation_ctx = OperationContext::from_builtins(HashMap::from([
        (0, BuiltinOperation::AddNode),
        (1, BuiltinOperation::AppendChild),
        (2, BuiltinOperation::IndexCycle),
        (4, BuiltinOperation::AddEdge),
    ]));
    let user_defined_op = get_sample_user_defined_operation();
    let mk_list_user_op = get_mk_n_to_0_list_user_defined_operation(&operation_ctx, 10);

    let count_list_len_user_op = get_count_list_len_user_defined_operation(&operation_ctx, 11);
    let insert_bst_user_op = get_insert_bst_user_defined_operation(&operation_ctx, 12);
    let insert_bst_labeled_edges_user_op =
        get_labeled_edges_insert_bst_user_defined_operation(&operation_ctx, 13);
    let node_heights_user_op = get_node_heights_user_defined_operation(&operation_ctx, 14);

    operation_ctx.add_custom_operation(3, user_defined_op);
    operation_ctx.add_custom_operation(10, mk_list_user_op);
    operation_ctx.add_custom_operation(11, count_list_len_user_op);
    operation_ctx.add_custom_operation(12, insert_bst_user_op);
    operation_ctx.add_custom_operation(13, insert_bst_labeled_edges_user_op);
    operation_ctx.add_custom_operation(14, node_heights_user_op);

    // use OperationBuilder to try building a new operation
    let insert_bst_builder_test_user_op = insert_bst_builder_test(&operation_ctx, 15);
    operation_ctx.add_custom_operation(15, insert_bst_builder_test_user_op);

    let mut dot_collector = DotCollector::new();

    let mut g = SimpleSemantics::new_concrete_graph();
    dot_collector.collect(&g);
    let a = g.add_node(1);
    dot_collector.collect(&g);
    let b = g.add_node(2);
    dot_collector.collect(&g);
    g.add_edge(a, b, "edge".to_string());
    dot_collector.collect(&g);

    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 0, &[]).unwrap();
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 1, &[2.into()]).unwrap();
    dot_collector.collect(&g);

    // add 3 new nodes
    // 4
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 0, &[]).unwrap();
    dot_collector.collect(&g);
    // 5
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 0, &[]).unwrap();
    dot_collector.collect(&g);
    // 6
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 0, &[]).unwrap();
    dot_collector.collect(&g);

    // add cycle
    g.add_edge(6, 4, "cycle".to_string());
    dot_collector.collect(&g);
    // add edge
    g.add_edge(4, 5, "anything1".to_string());
    dot_collector.collect(&g);
    // add edge
    g.add_edge(5, 6, "anything2".to_string());
    dot_collector.collect(&g);

    // add other children to 4 that are ignored
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 1, &[4.into()]).unwrap();
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 1, &[4.into()]).unwrap();
    dot_collector.collect(&g);

    // run cycle operation
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 2, &[4.into()]).unwrap();
    dot_collector.collect(&g);

    // run user defined op
    let new_start = g.add_node(99);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 3, &[new_start]).unwrap();
    dot_collector.collect(&g);

    // new node to make list out of
    let list_root = g.add_node(10);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 10, &[list_root]).unwrap();
    dot_collector.collect(&g);

    // new node to count
    let accumulator = g.add_node(0);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 11, &[list_root, accumulator])
        .unwrap();
    dot_collector.collect(&g);

    // new root BST node
    let bst_root = g.add_node(-1);
    dot_collector.collect(&g);
    // insert 5
    let value_to_insert = g.add_node(5);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 12, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    // insert 3
    let value_to_insert = g.add_node(3);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 12, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    // insert 7
    let value_to_insert = g.add_node(7);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 12, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    // insert 1
    let value_to_insert = g.add_node(1);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 12, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    // insert 2
    let value_to_insert = g.add_node(2);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 12, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    // insert 4
    let value_to_insert = g.add_node(4);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 12, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    let bst_labeled_edges_root = g.add_node(-1);
    dot_collector.collect(&g);

    // insert 5
    let value_to_insert = g.add_node(5);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(
        &mut g,
        &operation_ctx,
        13,
        &[bst_labeled_edges_root, value_to_insert],
    )
    .unwrap();
    dot_collector.collect(&g);
    // insert 3
    let value_to_insert = g.add_node(3);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(
        &mut g,
        &operation_ctx,
        13,
        &[bst_labeled_edges_root, value_to_insert],
    )
    .unwrap();
    dot_collector.collect(&g);
    // insert 7
    let value_to_insert = g.add_node(7);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(
        &mut g,
        &operation_ctx,
        13,
        &[bst_labeled_edges_root, value_to_insert],
    )
    .unwrap();
    dot_collector.collect(&g);
    // insert 1
    let value_to_insert = g.add_node(1);
    dot_collector.collect(&g);
    // println!("{}", dot_collector.finalize());
    run_from_concrete::<SimpleSemantics>(
        &mut g,
        &operation_ctx,
        13,
        &[bst_labeled_edges_root, value_to_insert],
    )
    .unwrap();
    dot_collector.collect(&g);
    // insert 2
    let value_to_insert = g.add_node(2);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(
        &mut g,
        &operation_ctx,
        13,
        &[bst_labeled_edges_root, value_to_insert],
    )
    .unwrap();
    dot_collector.collect(&g);
    // insert 4
    let value_to_insert = g.add_node(4);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(
        &mut g,
        &operation_ctx,
        13,
        &[bst_labeled_edges_root, value_to_insert],
    )
    .unwrap();
    dot_collector.collect(&g);

    // run node heights on that binary tree
    // run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 14, &[bst_labeled_edges_root.into()])
    //     .unwrap();
    // dot_collector.collect(&g);

    // repeat the BST experiement but with op 15
    let bst_root = g.add_node(-1);
    dot_collector.collect(&g);
    // insert 5
    let value_to_insert = g.add_node(5);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 15, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);
    // insert 3
    let value_to_insert = g.add_node(3);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 15, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);
    // insert 7
    let value_to_insert = g.add_node(7);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 15, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);
    // insert 1
    let value_to_insert = g.add_node(1);
    dot_collector.collect(&g);
    // println!("{}", dot_collector.finalize());
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 15, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);
    // insert 2
    let value_to_insert = g.add_node(2);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 15, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);
    // insert 4
    let value_to_insert = g.add_node(4);
    dot_collector.collect(&g);
    run_from_concrete::<SimpleSemantics>(&mut g, &operation_ctx, 15, &[bst_root, value_to_insert])
        .unwrap();
    dot_collector.collect(&g);

    println!("{}", dot_collector.finalize());
}
