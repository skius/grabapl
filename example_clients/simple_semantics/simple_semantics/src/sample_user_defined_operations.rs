use crate::{BuiltinOperation, BuiltinQuery, EdgePattern, SimpleSemantics};
use grabapl::operation::query::GraphShapeQuery;
use grabapl::operation::signature::parameter::OperationParameter;
use grabapl::operation::signature::parameterbuilder::OperationParameterBuilder;
use grabapl::operation::user_defined::{
    AbstractNodeId, AbstractOperationArgument, Instruction, OpLikeInstruction, QueryInstructions,
    UserDefinedOperation,
};
use grabapl::prelude::*;
use grabapl::util::bimap::BiMap;
use std::collections::HashMap;

fn mk_builtin_instruction(
    op: BuiltinOperation,
    args: Vec<AbstractNodeId>,
) -> Instruction<SimpleSemantics> {
    let arg = AbstractOperationArgument::infer_explicit_for_param(
        args,
        &<BuiltinOperation as grabapl::operation::BuiltinOperation>::parameter(&op),
    )
    .unwrap();
    Instruction::OpLike(OpLikeInstruction::Builtin(op), arg)
}

fn mk_builtin_query(
    query: BuiltinQuery,
    args: Vec<AbstractNodeId>,
    instructions: QueryInstructions<SimpleSemantics>,
) -> Instruction<SimpleSemantics> {
    let arg = AbstractOperationArgument::infer_explicit_for_param(
        args,
        &<BuiltinQuery as grabapl::operation::query::BuiltinQuery>::parameter(&query),
    )
    .unwrap();
    Instruction::BuiltinQuery(query, arg, instructions)
}

// Note: assumes the subst markers are 0..n
fn mk_operation_instruction(
    op_id: OperationId,
    param: &OperationParameter<SimpleSemantics>,
    args: Vec<AbstractNodeId>,
) -> Instruction<SimpleSemantics> {
    let arg = AbstractOperationArgument::infer_explicit_for_param(args, param).unwrap();
    Instruction::OpLike(OpLikeInstruction::Operation(op_id), arg)
}

pub fn get_sample_user_defined_operation() -> UserDefinedOperation<SimpleSemantics> {
    let mut g = grabapl::graph::Graph::new();
    let a = g.add_node(());
    let param = OperationParameter {
        explicit_input_nodes: vec!["input".into()],
        parameter_graph: g,
        node_keys_to_subst: BiMap::from([(a, "input".into())]),
    };

    let input_node = AbstractNodeId::param("input");

    let mut instructions = vec![
        (
            Some("first_child".into()),
            mk_builtin_instruction(BuiltinOperation::AppendChild, vec![input_node]),
        ),
        (
            Some("second_child".into()),
            mk_builtin_instruction(BuiltinOperation::AppendChild, vec![input_node]),
        ),
        (
            Some("third_child".into()),
            mk_builtin_instruction(BuiltinOperation::AppendChild, vec![input_node]),
        ),
        (
            Some("fourth_child".into()),
            mk_builtin_instruction(BuiltinOperation::AppendChild, vec![input_node]),
        ),
    ];

    let second_id = AbstractNodeId::DynamicOutputMarker("second_child".into(), "child".into());
    let third_id = AbstractNodeId::DynamicOutputMarker("third_child".into(), "child".into());
    let fourth_id = AbstractNodeId::DynamicOutputMarker("fourth_child".into(), "child".into());

    instructions.push((
        None,
        mk_builtin_instruction(BuiltinOperation::AddEdge, vec![fourth_id, third_id]),
    ));
    instructions.push((
        None,
        mk_builtin_instruction(BuiltinOperation::AddEdge, vec![third_id, second_id]),
    ));
    instructions.push((
        None,
        mk_builtin_instruction(BuiltinOperation::AddEdge, vec![second_id, fourth_id]),
    ));
    instructions.push((
        None,
        mk_builtin_instruction(
            BuiltinOperation::SetEdgeValue("cycle".to_string()),
            vec![second_id, fourth_id],
        ),
    ));

    instructions.push((
        None,
        Instruction::OpLike(
            OpLikeInstruction::Builtin(BuiltinOperation::IndexCycle),
            AbstractOperationArgument {
                selected_input_nodes: vec![fourth_id],
                // TODO: double check this hashmap. I think it's right but ...
                subst_to_aid: HashMap::from([
                    (0.to_string().into(), fourth_id),
                    (
                        1.to_string().into(),
                        AbstractNodeId::DynamicOutputMarker("third_child".into(), "child".into()),
                    ),
                    (
                        2.to_string().into(),
                        AbstractNodeId::DynamicOutputMarker("second_child".into(), "child".into()),
                    ),
                    // (1, AbstractNodeId::DynamicOutputMarker("first_child".into(), "child".into())),
                    // (4, AbstractNodeId::DynamicOutputMarker("fourth_child".into(), "child".into())),
                ]),
            },
        ),
    ));

    UserDefinedOperation::new(param, instructions)
}

pub fn get_mk_n_to_0_list_user_defined_operation(
    op_ctx: &OperationContext<SimpleSemantics>,
    self_op_id: u32,
) -> UserDefinedOperation<SimpleSemantics> {
    // Expects one input node
    let mut param_builder = OperationParameterBuilder::new();
    param_builder.expect_explicit_input_node("a", ()).unwrap();
    let param = param_builder.build().unwrap();
    let mk_operation_instruction = |op_id: OperationId, args: Vec<AbstractNodeId>| {
        mk_operation_instruction(op_id, &op_ctx.get(op_id).unwrap().parameter(), args)
    };
    let mk_self_operation_instruction = |args: Vec<AbstractNodeId>| {
        crate::sample_user_defined_operations::mk_operation_instruction(self_op_id, &param, args)
    };

    let input_node = AbstractNodeId::param("a");

    // If the input value is 0, we do nothing, otherwise we recurse on a new child
    let instructions = vec![(
        None,
        mk_builtin_query(
            BuiltinQuery::IsValueGt(0),
            vec![input_node],
            QueryInstructions {
                not_taken: vec![],
                taken: vec![
                    (
                        Some("add_child".into()),
                        mk_operation_instruction(1, vec![input_node]),
                    ),
                    (
                        None,
                        mk_builtin_instruction(
                            BuiltinOperation::CopyNodeValueTo,
                            vec![
                                input_node,
                                AbstractNodeId::DynamicOutputMarker(
                                    "add_child".into(),
                                    "child".into(),
                                ),
                            ],
                        ),
                    ),
                    (
                        None,
                        mk_builtin_instruction(
                            BuiltinOperation::Decrement,
                            vec![AbstractNodeId::DynamicOutputMarker(
                                "add_child".into(),
                                "child".into(),
                            )],
                        ),
                    ),
                    // recursive call
                    (
                        None,
                        mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker(
                            "add_child".into(),
                            "child".into(),
                        )]),
                    ),
                ],
            },
        ),
    )];

    // TODO: think about how to define the "new nodes" thing for user defined ops. In particular, how can we somehow specify
    //  the names for all recursive calls?
    //  we could have an automatically generated name by default which is just some concat of op id and the actual op's result marker,
    //  and then also the option for the user to override a mapping like:
    //  AbstractNodeId::DynamicOutputMarker("add_child", "child") -> OutputMarker("the_child").
    //  .
    //  In such a case, would we want to check that the node always gets created? probably.
    //  What if a caller wants to access a conditionally created node? the query system needs to be used to check that a node exists.

    UserDefinedOperation::new(param, instructions)
}

pub fn get_count_list_len_user_defined_operation(
    op_ctx: &OperationContext<SimpleSemantics>,
    self_op_id: OperationId,
) -> UserDefinedOperation<SimpleSemantics> {
    // Expects the list head as first input node, then the accumulator as second input node
    let mut param_builder = OperationParameterBuilder::new();
    param_builder
        .expect_explicit_input_node("input", ())
        .unwrap();
    param_builder.expect_explicit_input_node("acc", ()).unwrap();
    let param = param_builder.build().unwrap();
    let _mk_operation_instruction = |op_id: OperationId, args: Vec<AbstractNodeId>| {
        mk_operation_instruction(op_id, &op_ctx.get(op_id).unwrap().parameter(), args)
    };
    let mk_self_operation_instruction = |args: Vec<AbstractNodeId>| {
        crate::sample_user_defined_operations::mk_operation_instruction(self_op_id, &param, args)
    };

    let input_node = AbstractNodeId::param("input");
    let acc_node = AbstractNodeId::param("acc");

    let mut instructions = vec![];
    // Increment acc
    instructions.push((
        None,
        mk_builtin_instruction(BuiltinOperation::Increment, vec![acc_node]),
    ));

    // shape query to get next child if it exists
    let shape_query = {
        let mut g = grabapl::graph::Graph::new();
        let head = g.add_node(());
        let mut expected_g = g.clone();
        let param = OperationParameter {
            explicit_input_nodes: vec!["input".into()],
            parameter_graph: g,
            node_keys_to_subst: BiMap::from([(head, "input".into())]),
        };

        let child = expected_g.add_node(());
        expected_g.add_edge(head, child, EdgePattern::Wildcard);
        GraphShapeQuery::new(param, expected_g, BiMap::from([(child, "child".into())]))
    };

    let new_child = AbstractNodeId::DynamicOutputMarker("next_child_query".into(), "child".into());
    instructions.push((
        Some("next_child_query".into()),
        Instruction::ShapeQuery(
            shape_query,
            AbstractOperationArgument {
                selected_input_nodes: vec![input_node],
                subst_to_aid: HashMap::from([("input".into(), input_node)]),
            },
            QueryInstructions {
                not_taken: vec![],
                taken: vec![(
                    None,
                    mk_self_operation_instruction(vec![new_child, acc_node]),
                )],
            },
        ),
    ));

    UserDefinedOperation::new(param, instructions)
}

// TODO: add a new op that maybe does something with a binary tree?

// binary search tree:
//  nil node is -1
//  otherwise left child is smaller, right child is larger, inner nodes can store values.

// TODO: I'm pretty sure this has a (user fault, not library) bug when there's just one child and we add the second child,
//  because we _append_ the child even if it should be the left child.
pub fn get_insert_bst_user_defined_operation(
    op_ctx: &OperationContext<SimpleSemantics>,
    self_op_id: OperationId,
) -> UserDefinedOperation<SimpleSemantics> {
    // Expects the root of the binary tree as first input node, then the value to insert as second input node
    let mut param_builder = OperationParameterBuilder::new();
    param_builder
        .expect_explicit_input_node("root", ())
        .unwrap();
    param_builder
        .expect_explicit_input_node("value", ())
        .unwrap();
    let param = param_builder.build().unwrap();
    let _mk_operation_instruction = |op_id: OperationId, args: Vec<AbstractNodeId>| {
        mk_operation_instruction(op_id, &op_ctx.get(op_id).unwrap().parameter(), args)
    };
    let mk_self_operation_instruction = |args: Vec<AbstractNodeId>| {
        crate::sample_user_defined_operations::mk_operation_instruction(self_op_id, &param, args)
    };

    let root_node = AbstractNodeId::param("root");
    let value_node = AbstractNodeId::param("value");
    let mut instructions = vec![];
    // check if the root is nil
    instructions.push((None, mk_builtin_query(BuiltinQuery::IsValueEq(-1), vec![root_node], QueryInstructions {
        taken: vec![
            // if it is nil, we insert the value here
            // TODO: add an OR ValuesEqual to see if the value is already there.
            (None, mk_builtin_instruction(BuiltinOperation::CopyNodeValueTo, vec![value_node, root_node])),
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
                    GraphShapeQuery::new(
                        OperationParameter {
                            explicit_input_nodes: vec!["input".into()],
                            parameter_graph: g,
                            node_keys_to_subst: BiMap::from([(head, "input".into())]),
                        },
                        expected_g,
                        BiMap::from([
                            (left_child, "left".into()),
                            (right_child, "right".into()),
                        ]),
                    )
                },
                AbstractOperationArgument {  selected_input_nodes: vec![root_node],
                    subst_to_aid: HashMap::from([("input".into(), root_node)]),
                },
                QueryInstructions {
                    taken: vec![
                        // we have two children, now we need to check if our value is gt or smaller than the root
                        (None, mk_builtin_query(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                            taken: vec![
                                // if it is greater, we go to the right child
                                (None, mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker("two_children_query".into(), "right".into()), value_node])),
                            ],
                            not_taken: vec![
                                // if it is smaller or equal, we go to the left child
                                (None, mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker("two_children_query".into(), "left".into()), value_node])),
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
                                GraphShapeQuery::new(
                                    OperationParameter {
                                        explicit_input_nodes: vec!["input".into()],
                                        parameter_graph: g,
                                        node_keys_to_subst: BiMap::from([(head, "input".into())]),
                                    },
                                    expected_g,
                                    BiMap::from([(child, "child".into())]),
                                )
                            },
                            AbstractOperationArgument {  selected_input_nodes: vec![root_node],
                                subst_to_aid: HashMap::from([("input".into(), root_node)]),
                            },
                            QueryInstructions {
                                taken: vec![
                                    // we have one child, now we need to check if our value is gt or smaller than the root
                                    // then we need to check if the child we have is left or right
                                    (None, mk_builtin_query(BuiltinQuery::FirstGtSnd, vec![value_node, root_node], QueryInstructions {
                                        taken: vec![
                                            // if value > root, we check if one_child_query.child is the right child (i.e., child > root)
                                            (None, mk_builtin_query(BuiltinQuery::FirstGtSnd, vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), root_node], QueryInstructions {
                                                taken: vec![
                                                    // if it is greater, we go to the right child
                                                    (None, mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), value_node])),
                                                ],
                                                not_taken: vec![
                                                    // if the one child that the root has it is smaller, the value node becomes the right child
                                                    // TODO: same considerations as connected components TODO below
                                                    (None, mk_builtin_instruction(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                                ],
                                            })),
                                        ],
                                        not_taken: vec![
                                            // if value < root, we check if one_child_query.child is the left child (i.e., root > child)
                                            (None, mk_builtin_query(BuiltinQuery::FirstGtSnd, vec![root_node, AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into())], QueryInstructions {
                                                taken: vec![
                                                    // if child < root, we go to the left child
                                                    (None, mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker("one_child_query".into(), "child".into()), value_node])),
                                                ],
                                                not_taken: vec![
                                                    // if the one child that the root has it is larger, the value node becomes the left child
                                                    // TODO: same considerations as connected components TODO below
                                                    (None, mk_builtin_instruction(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                                ],
                                            }))
                                        ],
                                    })),
                                ],
                                not_taken: vec![
                                    // we don't have any children, we can insert the value as a child
                                    // TODO: we're just adding an edge from root to the value node, how does that interact with the abstract graph view and connected components discussion?
                                    (None, mk_builtin_instruction(BuiltinOperation::AddEdge, vec![root_node, value_node])),
                                ],
                            }
                        ))
                    ],
                }
            ))
        ],
    })));
    // TODO: add remove value_node instruction

    UserDefinedOperation::new(param, instructions)
}

pub fn get_labeled_edges_insert_bst_user_defined_operation(
    op_ctx: &OperationContext<SimpleSemantics>,
    self_op_id: OperationId,
) -> UserDefinedOperation<SimpleSemantics> {
    // Same as the above insert bst operation, but edges have a "left" and "right" label that should make things easier

    // Expects the root of the binary tree as first input node, then the value to insert as second input node
    let mut param_builder = OperationParameterBuilder::new();
    param_builder
        .expect_explicit_input_node("root", ())
        .unwrap();
    param_builder
        .expect_explicit_input_node("value", ())
        .unwrap();
    let param = param_builder.build().unwrap();
    let _mk_operation_instruction = |op_id: OperationId, args: Vec<AbstractNodeId>| {
        mk_operation_instruction(op_id, &op_ctx.get(op_id).unwrap().parameter(), args)
    };
    let mk_self_operation_instruction = |args: Vec<AbstractNodeId>| {
        crate::sample_user_defined_operations::mk_operation_instruction(self_op_id, &param, args)
    };

    let root_node = AbstractNodeId::param("root");
    let value_node = AbstractNodeId::param("value");

    let mk_delete = || {
        (
            Some("delete_value_node".into()),
            mk_builtin_instruction(BuiltinOperation::DeleteNode, vec![value_node]),
        )
    };

    let mut instructions = vec![];
    // check if the root is nil
    instructions.push((
        None,
        mk_builtin_query(
            BuiltinQuery::IsValueEq(-1),
            vec![root_node],
            QueryInstructions {
                taken: vec![
                    // if it is nil, we insert the value here
                    (
                        None,
                        mk_builtin_instruction(
                            BuiltinOperation::CopyNodeValueTo,
                            vec![value_node, root_node],
                        ),
                    ),
                    mk_delete(),
                ],
                not_taken: vec![
                    // otherwise, we need to check if value > root
                    (
                        None,
                        mk_builtin_query(
                            BuiltinQuery::FirstGtSnd,
                            vec![value_node, root_node],
                            QueryInstructions {
                                taken: vec![
                                    // value > root. See if there is a right child, or, if not, add the value as right child
                                    (
                                        Some("right_child_query".into()),
                                        Instruction::ShapeQuery(
                                            {
                                                // the graph shape query
                                                let mut g = grabapl::graph::Graph::new();
                                                let head = g.add_node(());
                                                let mut expected_g = g.clone();
                                                let right_child = expected_g.add_node(());
                                                expected_g.add_edge(
                                                    head,
                                                    right_child,
                                                    EdgePattern::Exact("right".to_string()),
                                                );
                                                GraphShapeQuery::new(
                                                    OperationParameter {
                                                        explicit_input_nodes: vec!["input".into()],
                                                        parameter_graph: g,
                                                        node_keys_to_subst: BiMap::from([(
                                                            head,
                                                            "input".into(),
                                                        )]),
                                                    },
                                                    expected_g,
                                                    BiMap::from([(right_child, "right".into())]),
                                                )
                                            },
                                            AbstractOperationArgument {
                                                selected_input_nodes: vec![root_node],
                                                subst_to_aid: HashMap::from([(
                                                    "input".into(),
                                                    root_node,
                                                )]),
                                            },
                                            QueryInstructions {
                                                taken: vec![
                                                    // we have a right child, recurse on it
                                                    (
                                                        None,
                                                        mk_self_operation_instruction(vec![
                                                            AbstractNodeId::DynamicOutputMarker(
                                                                "right_child_query".into(),
                                                                "right".into(),
                                                            ),
                                                            value_node,
                                                        ]),
                                                    ),
                                                ],
                                                not_taken: vec![
                                                    // we don't have a right child, add the value as right child
                                                    (
                                                        Some("add_node".into()),
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::AddNode,
                                                            vec![],
                                                        ),
                                                    ),
                                                    (
                                                        None,
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::CopyNodeValueTo,
                                                            vec![
                                                                value_node,
                                                                AbstractNodeId::DynamicOutputMarker(
                                                                    "add_node".into(),
                                                                    "new".into(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    (
                                                        None,
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::AddEdge,
                                                            vec![
                                                                root_node,
                                                                AbstractNodeId::DynamicOutputMarker(
                                                                    "add_node".into(),
                                                                    "new".into(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    (
                                                        None,
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::SetEdgeValue(
                                                                "right".to_string(),
                                                            ),
                                                            vec![
                                                                root_node,
                                                                AbstractNodeId::DynamicOutputMarker(
                                                                    "add_node".into(),
                                                                    "new".into(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    mk_delete(),
                                                ],
                                            },
                                        ),
                                    ),
                                ],
                                not_taken: vec![
                                    // value < root. See if there is a left child, or, if not, add the value as left child
                                    (
                                        Some("left_child_query".into()),
                                        Instruction::ShapeQuery(
                                            {
                                                // the graph shape query
                                                let mut g = grabapl::graph::Graph::new();
                                                let head = g.add_node(());
                                                let mut expected_g = g.clone();
                                                let left_child = expected_g.add_node(());
                                                expected_g.add_edge(
                                                    head,
                                                    left_child,
                                                    EdgePattern::Exact("left".to_string()),
                                                );
                                                GraphShapeQuery::new(
                                                    OperationParameter {
                                                        explicit_input_nodes: vec!["input".into()],
                                                        parameter_graph: g,
                                                        node_keys_to_subst: BiMap::from([(
                                                            head,
                                                            "input".into(),
                                                        )]),
                                                    },
                                                    expected_g,
                                                    BiMap::from([(left_child, "left".into())]),
                                                )
                                            },
                                            AbstractOperationArgument {
                                                selected_input_nodes: vec![root_node],
                                                subst_to_aid: HashMap::from([(
                                                    "input".into(),
                                                    root_node,
                                                )]),
                                            },
                                            QueryInstructions {
                                                taken: vec![
                                                    // we have a left child, recurse on it
                                                    (
                                                        None,
                                                        mk_self_operation_instruction(vec![
                                                            AbstractNodeId::DynamicOutputMarker(
                                                                "left_child_query".into(),
                                                                "left".into(),
                                                            ),
                                                            value_node,
                                                        ]),
                                                    ),
                                                ],
                                                not_taken: vec![
                                                    // we don't have a left child, add the value as left child
                                                    (
                                                        Some("add_node".into()),
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::AddNode,
                                                            vec![],
                                                        ),
                                                    ),
                                                    (
                                                        None,
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::CopyNodeValueTo,
                                                            vec![
                                                                value_node,
                                                                AbstractNodeId::DynamicOutputMarker(
                                                                    "add_node".into(),
                                                                    "new".into(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    (
                                                        None,
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::AddEdge,
                                                            vec![
                                                                root_node,
                                                                AbstractNodeId::DynamicOutputMarker(
                                                                    "add_node".into(),
                                                                    "new".into(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    (
                                                        None,
                                                        mk_builtin_instruction(
                                                            BuiltinOperation::SetEdgeValue(
                                                                "left".to_string(),
                                                            ),
                                                            vec![
                                                                root_node,
                                                                AbstractNodeId::DynamicOutputMarker(
                                                                    "add_node".into(),
                                                                    "new".into(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    mk_delete(),
                                                ],
                                            },
                                        ),
                                    ),
                                ],
                            },
                        ),
                    ),
                ],
            },
        ),
    ));
    // finally, we delete the value node
    // OH! we can't delete it of course if an inner operation has already deleted it.
    // instructions.push(("delete_value_node".into(), mk_builtin_instruction(BuiltinOperation::DeleteNode, vec![value_node])));
    // => instead we just delete wherever we _did not_ recurse.

    // TODO: this would be a good example for the abstract graph to take the under approximated view. the value node should not still have been visible abstractly, since it may have
    //  been deleted by then (eg in the recursive call).

    UserDefinedOperation::new(param, instructions)
}

pub fn get_node_heights_user_defined_operation(
    op_ctx: &OperationContext<SimpleSemantics>,
    self_op_id: OperationId,
) -> UserDefinedOperation<SimpleSemantics> {
    // expects the root node of a binary tree (with left/right edges for children) as input node
    let mut param_builder = OperationParameterBuilder::new();
    param_builder
        .expect_explicit_input_node("root", ())
        .unwrap();
    let param = param_builder.build().unwrap();
    let _mk_operation_instruction = |op_id: OperationId, args: Vec<AbstractNodeId>| {
        mk_operation_instruction(op_id, &op_ctx.get(op_id).unwrap().parameter(), args)
    };
    let mk_self_operation_instruction = |args: Vec<AbstractNodeId>| {
        crate::sample_user_defined_operations::mk_operation_instruction(self_op_id, &param, args)
    };

    let root_node = AbstractNodeId::param("root");
    let mut instructions = vec![];

    // set root to 0
    instructions.push((
        Some("set_root_height".into()),
        mk_builtin_instruction(BuiltinOperation::SetNodeValue(0), vec![root_node]),
    ));
    // query if 'left' child exists, if so, call self_op_id on that child
    let left_child_query = {
        // the graph shape query
        let mut g = grabapl::graph::Graph::new();
        let head = g.add_node(());
        let mut expected_g = g.clone();
        let left_child = expected_g.add_node(());
        expected_g.add_edge(head, left_child, EdgePattern::Exact("left".to_string()));
        GraphShapeQuery::new(
            OperationParameter {
                explicit_input_nodes: vec!["input".into()],
                parameter_graph: g,
                node_keys_to_subst: BiMap::from([(head, "input".into())]),
            },
            expected_g,
            BiMap::from([(left_child, "left".into())]),
        )
    };
    let right_child_query = {
        // the graph shape query
        let mut g = grabapl::graph::Graph::new();
        let head = g.add_node(());
        let mut expected_g = g.clone();
        let right_child = expected_g.add_node(());
        expected_g.add_edge(head, right_child, EdgePattern::Exact("right".to_string()));
        GraphShapeQuery::new(
            OperationParameter {
                explicit_input_nodes: vec!["input".into()],
                parameter_graph: g,
                node_keys_to_subst: BiMap::from([(head, "input".into())]),
            },
            expected_g,
            BiMap::from([(right_child, "right".into())]),
        )
    };
    let left_child = AbstractNodeId::DynamicOutputMarker("left_child_query".into(), "left".into());
    let right_child =
        AbstractNodeId::DynamicOutputMarker("right_child_query".into(), "right".into());
    instructions.push((
        Some("left_child_query".into()),
        Instruction::ShapeQuery(
            left_child_query,
            AbstractOperationArgument {
                selected_input_nodes: vec![root_node],
                subst_to_aid: HashMap::from([("input".into(), root_node)]),
            },
            QueryInstructions {
                taken: vec![
                    // we have a left child, recurse on it
                    (
                        None,
                        mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker(
                            "left_child_query".into(),
                            "left".into(),
                        )]),
                    ),
                    // set root to max of it and left child
                    (
                        None,
                        mk_builtin_instruction(
                            BuiltinOperation::SetSndToMaxOfFstSnd,
                            vec![left_child, root_node],
                        ),
                    ),
                ],
                not_taken: vec![],
            },
        ),
    ));
    instructions.push((
        Some("right_child_query".into()),
        Instruction::ShapeQuery(
            right_child_query,
            AbstractOperationArgument {
                selected_input_nodes: vec![root_node],
                subst_to_aid: HashMap::from([("input".into(), root_node)]),
            },
            QueryInstructions {
                taken: vec![
                    // we have a right child, recurse on it
                    (
                        None,
                        mk_self_operation_instruction(vec![AbstractNodeId::DynamicOutputMarker(
                            "right_child_query".into(),
                            "right".into(),
                        )]),
                    ),
                    // set root to max of it and right child
                    (
                        None,
                        mk_builtin_instruction(
                            BuiltinOperation::SetSndToMaxOfFstSnd,
                            vec![right_child, root_node],
                        ),
                    ),
                ],
                not_taken: vec![],
            },
        ),
    ));
    // add 1 to root node, which is now the max of the heights of the left and right children
    instructions.push((
        Some("set_root_height".into()),
        mk_builtin_instruction(BuiltinOperation::Increment, vec![root_node]),
    ));

    UserDefinedOperation::new(param, instructions)
}
