use std::collections::HashMap;
use grabapl::graph::operation::query::GraphShapeQuery;
use grabapl::graph::operation::user_defined::{AbstractNodeId, Instruction, QueryInstructions, UserDefinedOperation};
use grabapl::graph::pattern::OperationParameter;
use grabapl::OperationId;
use crate::{BuiltinOperation, BuiltinQuery, EdgePattern, SimpleSemantics};

pub fn get_sample_user_defined_operation() -> UserDefinedOperation<SimpleSemantics> {
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
    instructions.push((Some("first_child".into()), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));
    instructions.push((Some("second_child".into()), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));
    instructions.push((Some("third_child".into()), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));
    instructions.push((Some("fourth_child".into()), Instruction::Builtin(BuiltinOperation::AppendChild, vec![input_node])));

    let second_id = AbstractNodeId::DynamicOutputMarker("second_child".into(), "child".into());
    let third_id = AbstractNodeId::DynamicOutputMarker("third_child".into(), "child".into());
    let fourth_id = AbstractNodeId::DynamicOutputMarker("fourth_child".into(), "child".into());

    instructions.push((None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![fourth_id, third_id])));
    instructions.push((None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![third_id, second_id])));
    instructions.push((None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![second_id, fourth_id])));
    instructions.push((None, Instruction::Builtin(BuiltinOperation::SetEdgeValue("cycle".to_string()), vec![second_id, fourth_id])));


    instructions.push((None, Instruction::Builtin(BuiltinOperation::IndexCycle, vec![
        fourth_id
    ])));

    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

pub fn get_mk_n_to_0_list_user_defined_operation() -> UserDefinedOperation<SimpleSemantics> {
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
    instructions.push((None, Instruction::BuiltinQuery(BuiltinQuery::IsValueGt(0), vec![input_node], QueryInstructions {
        not_taken: vec![],
        taken: vec![
            (Some("add_child".into()), Instruction::Operation(1, vec![input_node])),
            (None, Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![input_node, AbstractNodeId::DynamicOutputMarker("add_child".into(), "child".into())])),
            (None, Instruction::Builtin(BuiltinOperation::Decrement, vec![AbstractNodeId::DynamicOutputMarker("add_child".into(), "child".into())])),
            // recursive call
            (None, Instruction::Operation(10, vec![AbstractNodeId::DynamicOutputMarker("add_child".into(), "child".into())])),
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

pub fn get_count_list_len_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
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
    instructions.push((None, Instruction::Builtin(BuiltinOperation::Increment, vec![acc_node])));

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
    instructions.push((Some("next_child_query".into()), Instruction::ShapeQuery(shape_query, vec![input_node], QueryInstructions {
        not_taken: vec![],
        taken: vec![
            (None, Instruction::Operation(self_op_id, vec![new_child, acc_node])),
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

// TODO: I'm pretty sure this has a (user fault, not library) bug when there's just one child and we add the second child,
//  because we _append_ the child even if it should be the left child.
pub fn get_insert_bst_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
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
    instructions.push((None, Instruction::BuiltinQuery(BuiltinQuery::IsValueEq(-1), vec![root_node], QueryInstructions {
        taken: vec![
            // if it is nil, we insert the value here
            // TODO: add an OR ValuesEqual to see if the value is already there.
            (None, Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, root_node])),
        ],
        not_taken: vec![
            // otherwise, we check children. For that we first need to get children.
            (Some("two_children_query".into()), Instruction::ShapeQuery(
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
                        (None, Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                            taken: vec![
                                // if it is greater, we go to the right child
                                (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("two_children_query".into(), "right".into()), value_node])),
                            ],
                            not_taken: vec![
                                // if it is smaller or equal, we go to the left child
                                (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("two_children_query".into(), "left".into()), value_node])),
                            ],
                        })),
                    ],
                    not_taken: vec![
                        // we don't have two children. TODO: check if we have one child or zero children.
                        (Some("one_child_query".into()), Instruction::ShapeQuery(
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
                                    (None, Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                                        taken: vec![
                                            // if value > root, we check if one_child_query.child is the right child (i.e., child > root)
                                            (None, Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), root_node], QueryInstructions {
                                                taken: vec![
                                                    // if it is greater, we go to the right child
                                                    (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), value_node])),
                                                ],
                                                not_taken: vec![
                                                    // if the one child that the root has it is smaller, the value node becomes the right child
                                                    // TODO: same considerations as connected components TODO below
                                                    (None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                                ],
                                            })),
                                        ],
                                        not_taken: vec![
                                            // if value < root, we check if one_child_query.child is the left child (i.e., root > child)
                                            (None, Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![root_node, AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into())], QueryInstructions {
                                                taken: vec![
                                                    // if child < root, we go to the left child
                                                    (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), value_node])),
                                                ],
                                                not_taken: vec![
                                                    // if the one child that the root has it is larger, the value node becomes the left child
                                                    // TODO: same considerations as connected components TODO below
                                                    (None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                                ],
                                            }))
                                        ],
                                    })),
                                ],
                                not_taken: vec![
                                    // we don't have any children, we can insert the value as a child
                                    // TODO: we're just adding an edge from root to the value node, how does that interact with the abstract graph view and connected components discussion?
                                    (None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, value_node])),
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

pub fn get_labeled_edges_insert_bst_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
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
        (Some("delete_value_node".into()), Instruction::Builtin(BuiltinOperation::DeleteNode, vec![value_node]))
    };

    let mut instructions = vec![];
    // check if the root is nil
    instructions.push((None, Instruction::BuiltinQuery(BuiltinQuery::IsValueEq(-1), vec![root_node], QueryInstructions {
        taken: vec![
            // if it is nil, we insert the value here
            (None, Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, root_node])),
            mk_delete(),
        ],
        not_taken: vec![
            // otherwise, we need to check if value > root
            (None, Instruction::BuiltinQuery(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                taken: vec![
                    // value > root. See if there is a right child, or, if not, add the value as right child
                    (Some("right_child_query".into()), Instruction::ShapeQuery(
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
                                (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("right_child_query".into(), "right".into()), value_node])),
                            ],
                            not_taken: vec![
                                // we don't have a right child, add the value as right child
                                (Some("add_node".into()), Instruction::Builtin(BuiltinOperation::AddNode, vec![])),
                                (None, Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                (None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                (None, Instruction::Builtin(BuiltinOperation::SetEdgeValue("right".to_string()), vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                mk_delete(),
                            ],
                        }
                    )),
                ],
                not_taken: vec![
                    // value < root. See if there is a left child, or, if not, add the value as left child
                    (Some("left_child_query".into()), Instruction::ShapeQuery(
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
                                (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("left_child_query".into(), "left".into()), value_node])),
                            ],
                            not_taken: vec![
                                // we don't have a left child, add the value as left child
                                (Some("add_node".into()), Instruction::Builtin(BuiltinOperation::AddNode, vec![])),
                                (None, Instruction::Builtin(BuiltinOperation::CopyNodeValueTo, vec![value_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                (None, Instruction::Builtin(BuiltinOperation::AddEdge, vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
                                (None, Instruction::Builtin(BuiltinOperation::SetEdgeValue("left".to_string()), vec![root_node, AbstractNodeId::DynamicOutputMarker("add_node".into(), "new".into())])),
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

pub fn get_node_heights_user_defined_operation(self_op_id: OperationId) -> UserDefinedOperation<SimpleSemantics> {
    // expects the root node of a binary tree (with left/right edges for children) as input node
    let mut g = grabapl::graph::Graph::new();
    let root_key = g.add_node(());
    let param = OperationParameter {
        explicit_input_nodes: vec![0],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, root_key)]),
        node_keys_to_subst: HashMap::from([(root_key, 0)]),
    };
    let root_node = AbstractNodeId::ParameterMarker(0);
    let mut instructions = vec![];
    
    // set root to 0
    instructions.push((Some("set_root_height".into()), Instruction::Builtin(BuiltinOperation::SetNodeValue(0), vec![root_node])));
    // query if 'left' child exists, if so, call self_op_id on that child
    let left_child_query = {
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
    };
    let right_child_query = {
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
    };
    let left_child = AbstractNodeId::DynamicOutputMarker("left_child_query".into(), "left".into());
    let right_child = AbstractNodeId::DynamicOutputMarker("right_child_query".into(), "right".into());
    instructions.push((Some("left_child_query".into()), Instruction::ShapeQuery(left_child_query, vec![root_node], QueryInstructions {
        taken: vec![
            // we have a left child, recurse on it
            (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("left_child_query".into(), "left".into())])),
            // set root to max of it and left child
            (None, Instruction::Builtin(BuiltinOperation::SetSndToMaxOfFstSnd, vec![left_child, root_node])),
        ],
        not_taken: vec![],
    })));
    instructions.push((Some("right_child_query".into()), Instruction::ShapeQuery(right_child_query, vec![root_node], QueryInstructions {
        taken: vec![
            // we have a right child, recurse on it
            (None, Instruction::Operation(self_op_id, vec![AbstractNodeId::DynamicOutputMarker("right_child_query".into(), "right".into())])),
            // set root to max of it and right child
            (None, Instruction::Builtin(BuiltinOperation::SetSndToMaxOfFstSnd, vec![right_child, root_node])),
        ],
        not_taken: vec![],
    })));
    // add 1 to root node, which is now the max of the heights of the left and right children
    instructions.push((Some("set_root_height".into()), Instruction::Builtin(BuiltinOperation::Increment, vec![root_node])));
    
    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}