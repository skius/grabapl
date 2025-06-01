use std::collections::HashMap;
use grabapl::{DotCollector, OperationContext, OperationId, Semantics, WithSubstMarker};
use grabapl::graph::EdgeAttribute;
use grabapl::graph::operation::query::{GraphShapeQuery, ShapeNodeIdentifier};
use grabapl::graph::operation::run_operation;
use grabapl::graph::operation::user_defined::{AbstractNodeId, Instruction, QueryInstructions, QueryTaken, UserDefinedOperation};
use grabapl::graph::pattern::{OperationOutput, OperationParameter};
use simple_semantics::{BuiltinOperation, BuiltinQuery, EdgePattern, SimpleSemantics};
use simple_semantics::sample_user_defined_operations::{get_count_list_len_user_defined_operation, get_insert_bst_user_defined_operation, get_labeled_edges_insert_bst_user_defined_operation, get_mk_n_to_0_list_user_defined_operation, get_node_heights_user_defined_operation, get_sample_user_defined_operation};

fn main() {
    let user_defined_op = get_sample_user_defined_operation();
    let mk_list_user_op = get_mk_n_to_0_list_user_defined_operation();

    let count_list_len_user_op = get_count_list_len_user_defined_operation(11);
    let insert_bst_user_op = get_insert_bst_user_defined_operation(12);
    let insert_bst_labeled_edges_user_op = get_labeled_edges_insert_bst_user_defined_operation(13);
    let node_heights_user_op = get_node_heights_user_defined_operation(14);

    let operation_ctx = HashMap::from([
        (0, BuiltinOperation::AddNode),
        (1, BuiltinOperation::AppendChild),
        (2, BuiltinOperation::IndexCycle),
        (4, BuiltinOperation::AddEdge),
        (5, BuiltinOperation::SetEdgeValueToCycle),
    ]);
    let mut operation_ctx = OperationContext::from_builtins(operation_ctx);
    operation_ctx.add_custom_operation(3, user_defined_op);
    operation_ctx.add_custom_operation(10, mk_list_user_op);
    operation_ctx.add_custom_operation(11, count_list_len_user_op);
    operation_ctx.add_custom_operation(12, insert_bst_user_op);
    operation_ctx.add_custom_operation(13, insert_bst_labeled_edges_user_op);
    operation_ctx.add_custom_operation(14, node_heights_user_op);

    let mut dot_collector = DotCollector::new();

    let mut g = SimpleSemantics::new_concrete_graph();
    dot_collector.collect(&g);
    let a = g.add_node(1);
    dot_collector.collect(&g);
    let b = g.add_node(2);
    dot_collector.collect(&g);
    g.add_edge(a, b, "edge".to_string());
    dot_collector.collect(&g);

    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 1, vec![2]).unwrap();
    dot_collector.collect(&g);

    // add 3 new nodes
    // 4
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);
    // 5
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);
    // 6
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
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
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 1, vec![4]).unwrap();
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 1, vec![4]).unwrap();
    dot_collector.collect(&g);

    // run cycle operation
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 2, vec![4]).unwrap();
    dot_collector.collect(&g);

    // run user defined op
    let new_start = g.add_node(99);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 3, vec![new_start]).unwrap();
    dot_collector.collect(&g);

    // new node to make list out of
    let list_root = g.add_node(10);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 10, vec![list_root]).unwrap();
    dot_collector.collect(&g);

    // new node to count
    let accumulator = g.add_node(0);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 11, vec![list_root, accumulator]).unwrap();
    dot_collector.collect(&g);


    // new root BST node
    let bst_root = g.add_node(-1);
    dot_collector.collect(&g);
    // insert 5
    let value_to_insert = g.add_node(5);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 12, vec![bst_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    // insert 3
    let value_to_insert = g.add_node(3);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 12, vec![bst_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    // insert 7
    let value_to_insert = g.add_node(7);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 12, vec![bst_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    // insert 1
    let value_to_insert = g.add_node(1);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 12, vec![bst_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    // insert 2
    let value_to_insert = g.add_node(2);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 12, vec![bst_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    // insert 4
    let value_to_insert = g.add_node(4);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 12, vec![bst_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    let bst_labeled_edges_root = g.add_node(-1);
    dot_collector.collect(&g);

    // insert 5
    let value_to_insert = g.add_node(5);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 13, vec![bst_labeled_edges_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);
    // insert 3
    let value_to_insert = g.add_node(3);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 13, vec![bst_labeled_edges_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);
    // insert 7
    let value_to_insert = g.add_node(7);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 13, vec![bst_labeled_edges_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);
    // insert 1
    let value_to_insert = g.add_node(1);
    dot_collector.collect(&g);
    // println!("{}", dot_collector.finalize());
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 13, vec![bst_labeled_edges_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);
    // insert 2
    let value_to_insert = g.add_node(2);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 13, vec![bst_labeled_edges_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);
    // insert 4
    let value_to_insert = g.add_node(4);
    dot_collector.collect(&g);
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 13, vec![bst_labeled_edges_root, value_to_insert]).unwrap();
    dot_collector.collect(&g);

    
    // run node heights on that binary tree
    run_operation::<SimpleSemantics>(&mut g, &operation_ctx, 14, vec![bst_labeled_edges_root]).unwrap();
    dot_collector.collect(&g);
    
    println!("{}", dot_collector.finalize());
}