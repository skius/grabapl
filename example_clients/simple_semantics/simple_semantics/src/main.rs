use std::collections::HashMap;
use grabapl::{DotCollector, OperationContext, OperationId, Semantics, WithSubstMarker};
use grabapl::graph::EdgeAttribute;
use grabapl::graph::operation::query::{GraphShapeQuery, ShapeNodeIdentifier};
use grabapl::graph::operation::run_operation;
use grabapl::graph::operation::user_defined::{AbstractNodeId, Instruction, QueryInstructions, QueryTaken, UserDefinedOperation};
use grabapl::graph::pattern::{OperationOutput, OperationParameter};
use simple_semantics::{BuiltinOperation, BuiltinQuery, EdgePattern, SimpleSemantics};

fn get_sample_user_defined_operation() -> UserDefinedOperation<SimpleSemantics> {
    // Expects a child
    let mut g = grabapl::graph::Graph::new();
    let a = g.add_node(());
    let param = OperationParameter {
        explicit_input_nodes: vec![0],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, a)]),
        node_keys_to_subst: HashMap::from([(a, 0)]),
    };

    let input_node = AbstractNodeId::ParameterMarker(0);

    let mut instructions = vec![];
    instructions.push(("first_child".into(), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));
    instructions.push(("second_child".into(), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));
    instructions.push(("third_child".into(), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));
    instructions.push(("fourth_child".into(), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));

    let second_id = AbstractNodeId::DynamicOutputMarker("second_child".into(), "child".into());
    let third_id = AbstractNodeId::DynamicOutputMarker("third_child".into(), "child".into());
    let fourth_id = AbstractNodeId::DynamicOutputMarker("fourth_child".into(), "child".into());

    instructions.push(("TODO ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![fourth_id, third_id])));
    instructions.push(("TODO ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![third_id, second_id])));
    instructions.push(("TODO ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![second_id, fourth_id])));
    instructions.push(("TODO ignore".into(), Instruction::Builtin(BuiltinOperation::SetEdgeValue("cycle".to_string()), vec![second_id, fourth_id])));


    instructions.push(("TODO ignore me".into(), Instruction::Builtin(BuiltinOperation::IndexCycle, vec![
        fourth_id
    ])));

    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

fn get_mk_n_to_0_list_user_defined_operation() -> UserDefinedOperation<SimpleSemantics> {
    // Expects one input node
    let mut g = grabapl::graph::Graph::new();
    let a = g.add_node(());
    let param = OperationParameter {
        explicit_input_nodes: vec![0],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, a)]),
        node_keys_to_subst: HashMap::from([(a, 0)]),
    };

    let input_node = AbstractNodeId::ParameterMarker(0);
    let mut instructions = vec![];

    // If the input value is 0, we do nothing, otherwise we recurse on a new child
    instructions.push(("eq_0_query (TODO: ignore)".into(), Instruction::BuiltinQuery(BuiltinQuery::IsValueGt(0), vec![input_node], QueryInstructions {
        not_taken: vec![],
        taken: vec![
            ("add_child".into(), Instruction::Operation(1, vec![input_node])),
            ("TODO: ignore".into(), Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![input_node, AbstractNodeId::DynamicOutputMarker("add_child".into(), "child".into())])),
            ("TODO: ignore".into(), Instruction::Builtin(BuiltinOperation::Decrement, vec![AbstractNodeId::DynamicOutputMarker("add_child".into(), "child".into())])),
            // recursive call
            ("TODO: ignore".into(), Instruction::Operation(10, vec![AbstractNodeId::DynamicOutputMarker("add_child".into(), "child".into())])),
        ],
    })));

    // TODO: think about how to define the "new nodes" thing for user defined ops. In particular, how can we somehow specify
    //  the names for all recursive calls?
    //  we could have an automatically generated name by default which is just some concat of op id and the actual op's result marker,
    //  and then also the option for the user to override a mapping like:
    //  AbstractNodeId::DynamicOutputMarker("add_child", "child") -> OutputMarker("the_child").
    //  .
    //  In such a case, would we want to check that the node always gets created? probably.
    //  What if a caller wants to access a conditionally created node? the query system needs to be used to check that a node exists.

    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

fn get_count_list_len_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
    // Expects the list head as first input node, then the accumulator as second input node
    let mut g = grabapl::graph::Graph::new();
    let input_key = g.add_node(());
    let acc_key = g.add_node(());

    let param = OperationParameter {
        explicit_input_nodes: vec![0, 1],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, input_key), (1, acc_key)]),
        node_keys_to_subst: HashMap::from([(input_key, 0), (acc_key, 1)]),
    };

    let input_node = AbstractNodeId::ParameterMarker(0);
    let acc_node = AbstractNodeId::ParameterMarker(1);

    let mut instructions = vec![];
    // Increment acc
    instructions.push(("TODO ignore".into(), Instruction::Builtin(BuiltinOperation::Increment, vec![acc_node])));

    // shape query to get next child if it exists
    let shape_query = {
        let mut g = grabapl::graph::Graph::new();
        let head = g.add_node(());
        let mut expected_g = g.clone();
        let param = OperationParameter {
            explicit_input_nodes: vec![0],
            parameter_graph: g,
            subst_to_node_keys: HashMap::from([(0, head)]),
            node_keys_to_subst: HashMap::from([(head, 0)]),
        };

        let child = expected_g.add_node(());
        expected_g.add_edge(head, child, EdgePattern::Wildcard);
        GraphShapeQuery {
            parameter: param,
            expected_graph: expected_g,
            node_keys_to_shape_idents: HashMap::from([(child, "child".into())]),
            shape_idents_to_node_keys: HashMap::from([("child".into(), child)]),
        }
    };

    let new_child = AbstractNodeId::DynamicOutputMarker("next_child_query".into(), "child".into());
    instructions.push(("next_child_query".into(), Instruction::ShapeQuery(shape_query, vec![input_node], QueryInstructions {
        not_taken: vec![],
        taken: vec![
            ("TODO: ignore".into(), Instruction::Operation(self_op_id, vec![new_child, acc_node])),
        ],
    })));


    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

// TODO: add a new op that maybe does something with a binary tree?

// binary search tree:
//  nil node is -1
//  otherwise left child is smaller, right child is larger, inner nodes can store values.

fn get_insert_bst_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
    // Expects the root of the binary tree as first input node, then the value to insert as second input node
    let mut g = grabapl::graph::Graph::new();
    let root_key = g.add_node(());
    let value_key = g.add_node(());
    let param = OperationParameter {
        explicit_input_nodes: vec![0, 1],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, root_key), (1, value_key)]),
        node_keys_to_subst: HashMap::from([(root_key, 0), (value_key, 1)]),
    };

    let root_node = AbstractNodeId::ParameterMarker(0);
    let value_node = AbstractNodeId::ParameterMarker(1);
    let mut instructions = vec![];
    // check if the root is nil
    instructions.push(("is_nil_query (TODO: ignore)".into(), Instruction::BuiltinQuery(BuiltinQuery::IsValueEq(-1), vec![root_node], QueryInstructions {
        taken: vec![
            // if it is nil, we insert the value here
            // TODO: add an OR ValuesEqual to see if the value is already there.
            ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, root_node])),
        ],
        not_taken: vec![
            // otherwise, we check children. For that we first need to get children.
            ("two_children_query".into(), Instruction::ShapeQuery(
                {
                    // the graph shape query
                    let mut g = grabapl::graph::Graph::new();
                    let head = g.add_node(());
                    let mut expected_g = g.clone();
                    let left_child = expected_g.add_node(());
                    let right_child = expected_g.add_node(());
                    expected_g.add_edge(head, left_child, EdgePattern::Wildcard);
                    expected_g.add_edge(head, right_child, EdgePattern::Wildcard);
                    GraphShapeQuery {
                        parameter: OperationParameter {
                            explicit_input_nodes: vec![0],
                            parameter_graph: g,
                            subst_to_node_keys: HashMap::from([(0, head)]),
                            node_keys_to_subst: HashMap::from([(head, 0)]),
                        },
                        expected_graph: expected_g,
                        node_keys_to_shape_idents: HashMap::from([
                            (left_child, "left".into()),
                            (right_child, "right".into()),
                        ]),
                        shape_idents_to_node_keys: HashMap::from([
                            ("left".into(), left_child),
                            ("right".into(), right_child),
                        ]),
                    }
                },
                vec![root_node],
                QueryInstructions {
                    taken: vec![
                        // we have two children, now we need to check if our value is gt or smaller than the root
                        ("todo ignore".into(), Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                            taken: vec![
                                // if it is greater, we go to the right child
                                ("todo ignore".into(), Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("two_children_query".into(), "right".into()), value_node])),
                            ],
                            not_taken: vec![
                                // if it is smaller or equal, we go to the left child
                                ("todo ignore".into(), Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("two_children_query".into(), "left".into()), value_node])),
                            ],
                        })),
                    ],
                    not_taken: vec![
                        // we don't have two children. TODO: check if we have one child or zero children.
                        ("one_child_query".into(), Instruction::ShapeQuery(
                            {
                                // the graph shape query
                                let mut g = grabapl::graph::Graph::new();
                                let head = g.add_node(());
                                let mut expected_g = g.clone();
                                let child = expected_g.add_node(());
                                expected_g.add_edge(head, child, EdgePattern::Wildcard);
                                GraphShapeQuery {
                                    parameter: OperationParameter {
                                        explicit_input_nodes: vec![0],
                                        parameter_graph: g,
                                        subst_to_node_keys: HashMap::from([(0, head)]),
                                        node_keys_to_subst: HashMap::from([(head, 0)]),
                                    },
                                    expected_graph: expected_g,
                                    node_keys_to_shape_idents: HashMap::from([(child, "child".into())]),
                                    shape_idents_to_node_keys: HashMap::from([("child".into(), child)]),
                                }
                            },
                            vec![root_node],
                            QueryInstructions {
                                taken: vec![
                                    // we have one child, now we need to check if our value is gt or smaller than the root
                                    // then we need to check if the child we have is left or right
                                    ("todo ignore".into(), Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                                        taken: vec![
                                            // if value > root, we check if one_child_query.child is the right child (i.e., child > root)
                                            ("todo ignore".into(), Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), root_node], QueryInstructions {
                                                taken: vec![
                                                    // if it is greater, we go to the right child
                                                    ("todo ignore".into(), Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), value_node])),
                                                ],
                                                not_taken: vec![
                                                    // if the one child that the root has it is smaller, the value node becomes the right child
                                                    // TODO: same considerations as connected components TODO below
                                                    ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                                ],
                                            })),
                                        ],
                                        not_taken: vec![
                                            // if value < root, we check if one_child_query.child is the left child (i.e., root > child)
                                            ("todo ignore".into(), Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![root_node, AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into())], QueryInstructions {
                                                taken: vec![
                                                    // if child < root, we go to the left child
                                                    ("todo ignore".into(), Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), value_node])),
                                                ],
                                                not_taken: vec![
                                                    // if the one child that the root has it is larger, the value node becomes the left child
                                                    // TODO: same considerations as connected components TODO below
                                                    ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                                ],
                                            }))
                                        ],
                                    })),
                                ],
                                not_taken: vec![
                                    // we don't have any children, we can insert the value as a child
                                    // TODO: we're just adding an edge from root to the value node, how does that interact with the abstract graph view and connected components discussion?
                                    ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                ],
                            }
                        ))
                    ],
                }
            ))
        ],
    })));
    // TODO: add remove value_node instruction

    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

fn get_labeled_edges_insert_bst_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
    // Same as the above insert bst operation, but edges have a "left" and "right" label that should make things easier

    // Expects the root of the binary tree as first input node, then the value to insert as second input node
    let mut g = grabapl::graph::Graph::new();
    let root_key = g.add_node(());
    let value_key = g.add_node(());
    let param = OperationParameter {
        explicit_input_nodes: vec![0, 1],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, root_key), (1, value_key)]),
        node_keys_to_subst: HashMap::from([(root_key, 0), (value_key, 1)]),
    };



    let root_node = AbstractNodeId::ParameterMarker(0);
    let value_node = AbstractNodeId::ParameterMarker(1);

    let mk_delete = || {
        ("delete_value_node".into(), Instruction::Builtin(BuiltinOperation::DeleteNode, vec![value_node]))
    };

    let mut instructions = vec![];
    // check if the root is nil
    instructions.push(("is_nil_query (TODO: ignore)".into(), Instruction::BuiltinQuery(BuiltinQuery::IsValueEq(-1), vec![root_node], QueryInstructions {
        taken: vec![
            // if it is nil, we insert the value here
            ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, root_node])),
            mk_delete(),
        ],
        not_taken: vec![
            // otherwise, we need to check if value > root
            ("todo ignore".into(), Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                taken: vec![
                    // value > root. See if there is a right child, or, if not, add the value as right child
                    ("right_child_query".into(), Instruction::ShapeQuery(
                        {
                            // the graph shape query
                            let mut g = grabapl::graph::Graph::new();
                            let head = g.add_node(());
                            let mut expected_g = g.clone();
                            let right_child = expected_g.add_node(());
                            expected_g.add_edge(head, right_child, EdgePattern::Exact("right".to_string()));
                            GraphShapeQuery {
                                parameter: OperationParameter {
                                    explicit_input_nodes: vec![0],
                                    parameter_graph: g,
                                    subst_to_node_keys: HashMap::from([(0, head)]),
                                    node_keys_to_subst: HashMap::from([(head, 0)]),
                                },
                                expected_graph: expected_g,
                                node_keys_to_shape_idents: HashMap::from([(right_child, "right".into())]),
                                shape_idents_to_node_keys: HashMap::from([("right".into(), right_child)]),
                            }
                        },
                        vec![root_node],
                        QueryInstructions {
                            taken: vec![
                                // we have a right child, recurse on it
                                ("todo ignore".into(), Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("right_child_query".into(), "right".into()), value_node])),
                            ],
                            not_taken: vec![
                                // we don't have a right child, add the value as right child
                                ("add_node".into(), Instruction::Builtin(BuiltinOperation::AddNode, vec![])),
                                ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::SetEdgeValue("right".to_string()), vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                mk_delete(),
                            ],
                        }
                    )),
                ],
                not_taken: vec![
                    // value < root. See if there is a left child, or, if not, add the value as left child
                    ("left_child_query".into(), Instruction::ShapeQuery(
                        {
                            // the graph shape query
                            let mut g = grabapl::graph::Graph::new();
                            let head = g.add_node(());
                            let mut expected_g = g.clone();
                            let left_child = expected_g.add_node(());
                            expected_g.add_edge(head, left_child, EdgePattern::Exact("left".to_string()));
                            GraphShapeQuery {
                                parameter: OperationParameter {
                                    explicit_input_nodes: vec![0],
                                    parameter_graph: g,
                                    subst_to_node_keys: HashMap::from([(0, head)]),
                                    node_keys_to_subst: HashMap::from([(head, 0)]),
                                },
                                expected_graph: expected_g,
                                node_keys_to_shape_idents: HashMap::from([(left_child, "left".into())]),
                                shape_idents_to_node_keys: HashMap::from([("left".into(), left_child)]),
                            }
                        },
                        vec![root_node],
                        QueryInstructions {
                            taken: vec![
                                // we have a left child, recurse on it
                                ("todo ignore".into(), Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("left_child_query".into(), "left".into()), value_node])),
                            ],
                            not_taken: vec![
                                // we don't have a left child, add the value as left child
                                ("add_node".into(), Instruction::Builtin(BuiltinOperation::AddNode, vec![])),
                                ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                ("todo ignore".into(), Instruction::Builtin(BuiltinOperation::SetEdgeValue("left".to_string()), vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                mk_delete(),
                            ],
                        }
                    )),
                ],
            })
            )
        ],
    })));
    // finally, we delete the value node
    // OH! we can't delete it of course if an inner operation has already deleted it.
    // instructions.push(("delete_value_node".into(), Instruction::Builtin(BuiltinOperation::DeleteNode, vec![value_node])));
    // => instead we just delete wherever we _did not_ recurse.

    // TODO: this would be a good example for the abstract graph to take the under approximated view. the value node should not still have been visible abstractly, since it may have
    //  been deleted by then (eg in the recursive call).


   UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

fn main() {
    let user_defined_op = get_sample_user_defined_operation();
    let mk_list_user_op = get_mk_n_to_0_list_user_defined_operation();

    let count_list_len_user_op = get_count_list_len_user_defined_operation(11);
    let insert_bst_user_op = get_insert_bst_user_defined_operation(12);
    let insert_bst_labeled_edges_user_op = get_labeled_edges_insert_bst_user_defined_operation(13);

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

    println!("{}", dot_collector.finalize());
}