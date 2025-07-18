use super::semantics::*;
use grabapl::graph::GraphTrait;
use grabapl::operation::builder::stack_based_builder::OperationBuilder2;
use grabapl::operation::signature::parameter::AbstractOutputNodeMarker;
use grabapl::prelude::*;
use proptest::proptest;
use proptest::test_runner::Config;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

const MAX_HEAP_REMOVE_ID: OperationId = 0;
const MAX_HEAP_REMOVE_HELPER_ID: OperationId = 1;

/// Returns an operation that solves "Task 2" from the OSF tasks:
///
/// Max Heap Removal
/// The function f should take as input the root note of a max-heap, and
/// it should return the maximum of the heap (root node), and
/// then restore the heap condition.
/// Reminder: A maximum heap is a binary tree in which the number value of each node is greater than the number value of its children, and
/// each node in the tree is a maximum heap itself.
fn populate_max_heap_remove_op(op_ctx: &mut OperationContext<TestSemantics>) {
    // first we need to have the helper
    populate_max_heap_remove_helper_op(op_ctx);

    // Our max heap has a sentinel node, which points to the root of the heap.
    // The main entry point operation takes the sentinel node as input,
    // creates a new node for the returned maximum value,
    // checks if the heap is empty, and if so, returns -1,
    // otherwise it calls the helper operation which takes the root of the current heap and the
    // out-param for the max value.

    let mut builder = OperationBuilder::new(&op_ctx, MAX_HEAP_REMOVE_ID);
    builder
        .expect_parameter_node("sentinel", NodeType::Object)
        .unwrap();
    let sentinel = AbstractNodeId::param("sentinel");
    // create a new node for the max value
    builder
        .add_named_operation(
            "max_value".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let max_value = AbstractNodeId::dynamic_output("max_value", "new");
    // check if the heap is empty
    builder.start_shape_query("q").unwrap();
    builder
        .expect_shape_node("root".into(), NodeType::Integer)
        .unwrap();
    let root_aid = AbstractNodeId::dynamic_output("q", "root");
    builder
        .expect_shape_edge(sentinel, root_aid, EdgeType::Wildcard)
        .unwrap();
    builder.enter_false_branch().unwrap();
    // if we don't have a child, return -1.
    // this is the value we already have
    builder.enter_true_branch().unwrap();
    // we have a child.
    builder
        .add_operation(
            BuilderOpLike::FromOperationId(MAX_HEAP_REMOVE_HELPER_ID),
            vec![root_aid, max_value],
        )
        .unwrap();
    builder.end_query().unwrap();
    builder
        .return_node(max_value, "max_value".into(), NodeType::Integer)
        .unwrap();

    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(MAX_HEAP_REMOVE_ID, op);
}

fn populate_max_heap_remove_helper_op(op_ctx: &mut OperationContext<TestSemantics>) {
    let mut builder = OperationBuilder::new(&op_ctx, MAX_HEAP_REMOVE_HELPER_ID);
    builder
        .expect_parameter_node("root", NodeType::Integer)
        .unwrap();
    let root = AbstractNodeId::param("root");
    builder
        .expect_parameter_node("max_value", NodeType::Integer)
        .unwrap();
    let max_value = AbstractNodeId::param("max_value");
    // we return value of the root node.
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![root, max_value],
        )
        .unwrap();
    // now, to remove the node and restore the heap condition,
    // we check the following cases:
    // if root has two children, recurse on the larger child, get the max value from there, copy that to root.
    // if the root has one child, recurse on that child, get the max value from there, copy that to root.
    // if the root has no children, we can delete root.

    builder.start_shape_query("q").unwrap();
    builder
        .expect_shape_node("left".into(), NodeType::Integer)
        .unwrap();
    let left_aid = AbstractNodeId::dynamic_output("q", "left");
    builder
        .expect_shape_edge(root, left_aid, EdgeType::Wildcard)
        .unwrap();
    builder
        .expect_shape_node("right".into(), NodeType::Integer)
        .unwrap();
    let right_aid = AbstractNodeId::dynamic_output("q", "right");
    builder
        .expect_shape_edge(root, right_aid, EdgeType::Wildcard)
        .unwrap();
    builder.enter_true_branch().unwrap();
    // we have two children. Check which is larger
    builder
        .start_query(
            TestQuery::CmpFstSnd(Ordering::Greater.into()),
            vec![left_aid, right_aid],
        )
        .unwrap();
    builder.enter_true_branch().unwrap();
    // if left > right, recurse on left
    // get a new result node for the max value
    // TODO: make temp node
    builder
        .add_named_operation(
            "temp_max".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let temp_max = AbstractNodeId::dynamic_output("temp_max", "new");
    builder
        .add_operation(BuilderOpLike::Recurse, vec![left_aid, temp_max])
        .unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![temp_max, root],
        )
        .unwrap();
    // and delete the temp node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![temp_max],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();
    // if left <= right, recurse on right
    // TODO: make temp node
    builder
        .add_named_operation(
            "temp_max".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let temp_max = AbstractNodeId::dynamic_output("temp_max", "new");
    builder
        .add_operation(BuilderOpLike::Recurse, vec![right_aid, temp_max])
        .unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![temp_max, root],
        )
        .unwrap();
    // and delete the temp node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![temp_max],
        )
        .unwrap();
    builder.end_query().unwrap();
    builder.enter_false_branch().unwrap();
    // If we don't have two children, check if we have one child.
    builder.start_shape_query("q").unwrap();
    builder
        .expect_shape_node("child".into(), NodeType::Integer)
        .unwrap();
    let child_aid = AbstractNodeId::dynamic_output("q", "child");
    builder
        .expect_shape_edge(root, child_aid, EdgeType::Wildcard)
        .unwrap();
    builder.enter_true_branch().unwrap();
    // we have one child, recurse on it
    // TODO: make temp node
    builder
        .add_named_operation(
            "temp_max".into(),
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {
                value: NodeValue::Integer(-1), // placeholder value
            }),
            vec![],
        )
        .unwrap();
    let temp_max = AbstractNodeId::dynamic_output("temp_max", "new");
    builder
        .add_operation(BuilderOpLike::Recurse, vec![child_aid, temp_max])
        .unwrap();
    builder
        .add_operation(
            BuilderOpLike::Builtin(TestOperation::CopyValueFromTo),
            vec![temp_max, root],
        )
        .unwrap();
    // and delete the temp node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![temp_max],
        )
        .unwrap();
    builder.enter_false_branch().unwrap();
    // if we don't have a child, we can delete the root node
    builder
        .add_operation(
            BuilderOpLike::LibBuiltin(LibBuiltinOperation::RemoveNode {
                param: NodeType::Object,
            }),
            vec![root],
        )
        .unwrap();
    builder.end_query().unwrap();
    builder.end_query().unwrap();

    let op = builder.build().unwrap();
    op_ctx.add_custom_operation(MAX_HEAP_REMOVE_HELPER_ID, op);
}

/// Creates a max-heap from a set of integer values and returns the sentinel node key.
fn mk_heap_from_values(values: &[i32]) -> (ConcreteGraph<TestSemantics>, NodeKey) {
    let mut g = TestSemantics::new_concrete_graph();
    let sentinel = g.add_node(NodeValue::String("sentinel".to_string()));

    let heap = BinaryHeap::from(values.to_vec());
    let mut node_vec = Vec::new();
    // note: relies on implementation detail of binaryheap. if any crater folks are reading this, sorry :( ping me or just break this
    for (i, val) in heap.iter().enumerate() {
        let node = g.add_node(NodeValue::Integer(*val));
        node_vec.push(node);
        if i > 0 {
            // add edges to the parent node
            let parent_index = (i - 1) / 2;
            let parent_node = node_vec[parent_index];
            let NodeValue::Integer(parent_val) = g.get_node_attr(parent_node).unwrap() else {
                unreachable!();
            };
            assert!(
                parent_val >= val,
                "Max heap property violated: parent value {} is not greater than or equal to child value {}",
                parent_val,
                val
            );
            g.add_edge(parent_node, node, "blah".to_string());
        }
    }

    // connect the sentinel to the root of the heap
    if let Some(&root) = node_vec.first() {
        g.add_edge(sentinel, root, "root".to_string());
    }

    (g, sentinel)
}

#[test_log::test]
fn proptest_max_heap_remove_heap() {
    let mut op_ctx = OperationContext::<TestSemantics>::new();
    populate_max_heap_remove_op(&mut op_ctx);

    eprintln!(
        "serialized_op_ctx:\n{}",
        serde_json::to_string_pretty(&op_ctx).unwrap()
    );

    proptest!(
        Config::with_cases(10),
        |(values in proptest::collection::vec(0..5000, 0..=10))| {
        // |(values in proptest::collection::vec(0..5000, 2000..=2000))| {
            let start = std::time::Instant::now();
            let mut expected_return_order: Vec<i32> = values.clone();
            expected_return_order.sort_unstable_by(|a, b| b.cmp(a)); // sort in descending order
            log_crate::info!("Length: {:?}", values.len());
            // create a max-heap from the values
            let (mut g, sentinel) = mk_heap_from_values(&values);

            // log_crate::info!("Heap created:\n{}", g.dot());

            for expected_max_value in expected_return_order {
                // run the max-heap removal operation
                let op_result = run_from_concrete(&mut g, &op_ctx, MAX_HEAP_REMOVE_ID, &[sentinel]).unwrap();
                // check if the max value node is present
                let max_value_node = op_result.new_nodes.get(&AbstractOutputNodeMarker::from("max_value")).unwrap();
                let max_value = g.get_node_attr(*max_value_node).unwrap();
                assert_eq!(
                    max_value,
                    &NodeValue::Integer(expected_max_value),
                    "Expected max value node to have value {}, but got {:?}",
                    expected_max_value,
                    max_value
                );
            }

            // check that the heap is empty
            // TODO: graph API needs improvements to avoid this loop over all edges
            g.edges().for_each(|(src, _, _)| {
                    assert_ne!(src, sentinel, "Expected no edges from the sentinel node after all removals");
            });

            log_crate::info!("Time taken: {:?}", start.elapsed());
        }
    );
}

#[test_log::test]
fn proptest_max_heap_remove_heap_from_parsed() {
    let op_ctx: OperationContext<TestSemantics> = serde_json::from_str(PARSED_OP_CTX_SRC).unwrap();

    const MAX_HEAP_REMOVE_ID: OperationId = 2;

    proptest!(
        Config::with_cases(10),
        |(values in proptest::collection::vec(0..5000, 0..=10))| {
        // |(values in proptest::collection::vec(0..5000, 2000..=2000))| {
            let start = std::time::Instant::now();
            let mut expected_return_order: Vec<i32> = values.clone();
            expected_return_order.sort_unstable_by(|a, b| b.cmp(a)); // sort in descending order
            log_crate::info!("Length: {:?}", values.len());
            // create a max-heap from the values
            let (mut g, sentinel) = mk_heap_from_values(&values);

            // log_crate::info!("Heap created:\n{}", g.dot());

            for expected_max_value in expected_return_order {
                // run the max-heap removal operation
                let op_result = run_from_concrete(&mut g, &op_ctx, MAX_HEAP_REMOVE_ID, &[sentinel]).unwrap();
                // check if the max value node is present
                let max_value_node = op_result.new_nodes.get(&AbstractOutputNodeMarker::from("max_value")).unwrap();
                let max_value = g.get_node_attr(*max_value_node).unwrap();
                assert_eq!(
                    max_value,
                    &NodeValue::Integer(expected_max_value),
                    "Expected max value node to have value {}, but got {:?}",
                    expected_max_value,
                    max_value
                );
            }

            // check that the heap is empty
            // TODO: graph API needs improvements to avoid this loop over all edges
            g.edges().for_each(|(src, _, _)| {
                    assert_ne!(src, sentinel, "Expected no edges from the sentinel node after all removals");
            });

            log_crate::info!("Time taken: {:?}", start.elapsed());
        }
    );
}

const PARSED_OP_CTX_SRC: &'static str = r##"
{
  "builtins": {},
  "libbuiltins": {},
  "custom": {
    "1": {
      "signature": {
        "name": "some_name",
        "parameter": {
          "explicit_input_nodes": [
            "root",
            "max_value"
          ],
          "parameter_graph": {
            "graph": {
              "nodes": [
                0,
                1
              ],
              "node_holes": [],
              "edge_property": "directed",
              "edges": []
            },
            "max_node_key": 2,
            "node_attr_map": {
              "0": {
                "node_attr": "Integer"
              },
              "1": {
                "node_attr": "Integer"
              }
            }
          },
          "node_keys_to_subst": {
            "left_to_right": {
              "1": "max_value",
              "0": "root"
            },
            "right_to_left": {
              "root": 0,
              "max_value": 1
            }
          }
        },
        "output": {
          "new_nodes": {},
          "new_edges": {},
          "maybe_changed_nodes": {
            "max_value": "Integer"
          },
          "maybe_changed_edges": {},
          "maybe_deleted_nodes": [
            "root"
          ],
          "maybe_deleted_edges": []
        }
      },
      "instructions": [
        [
          null,
          {
            "OpLike": [
              {
                "Builtin": "CopyValueFromTo"
              },
              {
                "selected_input_nodes": [
                  {
                    "ParameterMarker": "root"
                  },
                  {
                    "ParameterMarker": "max_value"
                  }
                ],
                "subst_to_aid": {
                  "source": {
                    "ParameterMarker": "root"
                  },
                  "destination": {
                    "ParameterMarker": "max_value"
                  }
                }
              }
            ]
          }
        ],
        [
          {
            "Custom": "shape_query_0"
          },
          {
            "ShapeQuery": [
              {
                "parameter": {
                  "explicit_input_nodes": [
                    "N(0)",
                    "N(1)"
                  ],
                  "parameter_graph": {
                    "graph": {
                      "nodes": [
                        0,
                        1
                      ],
                      "node_holes": [],
                      "edge_property": "directed",
                      "edges": []
                    },
                    "max_node_key": 2,
                    "node_attr_map": {
                      "0": {
                        "node_attr": "Integer"
                      },
                      "1": {
                        "node_attr": "Integer"
                      }
                    }
                  },
                  "node_keys_to_subst": {
                    "left_to_right": {
                      "1": "N(1)",
                      "0": "N(0)"
                    },
                    "right_to_left": {
                      "N(1)": 1,
                      "N(0)": 0
                    }
                  }
                },
                "expected_graph": {
                  "graph": {
                    "nodes": [
                      0,
                      1,
                      2,
                      3
                    ],
                    "node_holes": [],
                    "edge_property": "directed",
                    "edges": [
                      [
                        0,
                        2,
                        {
                          "edge_attr": "Wildcard",
                          "source_out_order": 1,
                          "target_in_order": 1
                        }
                      ],
                      [
                        0,
                        3,
                        {
                          "edge_attr": "Wildcard",
                          "source_out_order": 2,
                          "target_in_order": 1
                        }
                      ]
                    ]
                  },
                  "max_node_key": 4,
                  "node_attr_map": {
                    "2": {
                      "node_attr": "Integer"
                    },
                    "3": {
                      "node_attr": "Integer"
                    },
                    "0": {
                      "node_attr": "Integer"
                    },
                    "1": {
                      "node_attr": "Integer"
                    }
                  }
                },
                "node_keys_to_shape_idents": {
                  "left_to_right": {
                    "3": "right",
                    "2": "left"
                  },
                  "right_to_left": {
                    "left": 2,
                    "right": 3
                  }
                }
              },
              {
                "selected_input_nodes": [
                  {
                    "ParameterMarker": "root"
                  },
                  {
                    "ParameterMarker": "max_value"
                  }
                ],
                "subst_to_aid": {
                  "N(0)": {
                    "ParameterMarker": "root"
                  },
                  "N(1)": {
                    "ParameterMarker": "max_value"
                  }
                }
              },
              {
                "taken": [
                  [
                    null,
                    {
                      "BuiltinQuery": [
                        {
                          "CmpFstSnd": 1
                        },
                        {
                          "selected_input_nodes": [
                            {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "shape_query_0"
                                },
                                "left"
                              ]
                            },
                            {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "shape_query_0"
                                },
                                "right"
                              ]
                            }
                          ],
                          "subst_to_aid": {
                            "b": {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "shape_query_0"
                                },
                                "right"
                              ]
                            },
                            "a": {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "shape_query_0"
                                },
                                "left"
                              ]
                            }
                          }
                        },
                        {
                          "taken": [
                            [
                              {
                                "Implicit": 50000
                              },
                              {
                                "OpLike": [
                                  {
                                    "Builtin": {
                                      "AddNode": {
                                        "node_type": "Integer",
                                        "value": {
                                          "Integer": -1
                                        }
                                      }
                                    }
                                  },
                                  {
                                    "selected_input_nodes": [],
                                    "subst_to_aid": {}
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "RenameNode": {
                                  "old": {
                                    "DynamicOutputMarker": [
                                      {
                                        "Implicit": 50000
                                      },
                                      "new"
                                    ]
                                  },
                                  "new": {
                                    "Named": "temp_max"
                                  }
                                }
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Operation": 1
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "DynamicOutputMarker": [
                                          {
                                            "Custom": "shape_query_0"
                                          },
                                          "left"
                                        ]
                                      },
                                      {
                                        "Named": "temp_max"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "root": {
                                        "DynamicOutputMarker": [
                                          {
                                            "Custom": "shape_query_0"
                                          },
                                          "left"
                                        ]
                                      },
                                      "max_value": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "CopyValueFromTo"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "Named": "temp_max"
                                      },
                                      {
                                        "ParameterMarker": "root"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "source": {
                                        "Named": "temp_max"
                                      },
                                      "destination": {
                                        "ParameterMarker": "root"
                                      }
                                    }
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "DeleteNode"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "Named": "temp_max"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "target": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ]
                          ],
                          "not_taken": [
                            [
                              {
                                "Implicit": 50000
                              },
                              {
                                "OpLike": [
                                  {
                                    "Builtin": {
                                      "AddNode": {
                                        "node_type": "Integer",
                                        "value": {
                                          "Integer": -1
                                        }
                                      }
                                    }
                                  },
                                  {
                                    "selected_input_nodes": [],
                                    "subst_to_aid": {}
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "RenameNode": {
                                  "old": {
                                    "DynamicOutputMarker": [
                                      {
                                        "Implicit": 50000
                                      },
                                      "new"
                                    ]
                                  },
                                  "new": {
                                    "Named": "temp_max"
                                  }
                                }
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Operation": 1
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "DynamicOutputMarker": [
                                          {
                                            "Custom": "shape_query_0"
                                          },
                                          "right"
                                        ]
                                      },
                                      {
                                        "Named": "temp_max"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "root": {
                                        "DynamicOutputMarker": [
                                          {
                                            "Custom": "shape_query_0"
                                          },
                                          "right"
                                        ]
                                      },
                                      "max_value": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "CopyValueFromTo"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "Named": "temp_max"
                                      },
                                      {
                                        "ParameterMarker": "root"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "destination": {
                                        "ParameterMarker": "root"
                                      },
                                      "source": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "DeleteNode"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "Named": "temp_max"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "target": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ]
                          ]
                        }
                      ]
                    }
                  ]
                ],
                "not_taken": [
                  [
                    {
                      "Custom": "shape_query_1"
                    },
                    {
                      "ShapeQuery": [
                        {
                          "parameter": {
                            "explicit_input_nodes": [
                              "N(0)",
                              "N(1)"
                            ],
                            "parameter_graph": {
                              "graph": {
                                "nodes": [
                                  0,
                                  1
                                ],
                                "node_holes": [],
                                "edge_property": "directed",
                                "edges": []
                              },
                              "max_node_key": 2,
                              "node_attr_map": {
                                "0": {
                                  "node_attr": "Integer"
                                },
                                "1": {
                                  "node_attr": "Integer"
                                }
                              }
                            },
                            "node_keys_to_subst": {
                              "left_to_right": {
                                "1": "N(1)",
                                "0": "N(0)"
                              },
                              "right_to_left": {
                                "N(1)": 1,
                                "N(0)": 0
                              }
                            }
                          },
                          "expected_graph": {
                            "graph": {
                              "nodes": [
                                0,
                                1,
                                2
                              ],
                              "node_holes": [],
                              "edge_property": "directed",
                              "edges": [
                                [
                                  0,
                                  2,
                                  {
                                    "edge_attr": "Wildcard",
                                    "source_out_order": 1,
                                    "target_in_order": 1
                                  }
                                ]
                              ]
                            },
                            "max_node_key": 3,
                            "node_attr_map": {
                              "0": {
                                "node_attr": "Integer"
                              },
                              "1": {
                                "node_attr": "Integer"
                              },
                              "2": {
                                "node_attr": "Integer"
                              }
                            }
                          },
                          "node_keys_to_shape_idents": {
                            "left_to_right": {
                              "2": "child"
                            },
                            "right_to_left": {
                              "child": 2
                            }
                          }
                        },
                        {
                          "selected_input_nodes": [
                            {
                              "ParameterMarker": "root"
                            },
                            {
                              "ParameterMarker": "max_value"
                            }
                          ],
                          "subst_to_aid": {
                            "N(1)": {
                              "ParameterMarker": "max_value"
                            },
                            "N(0)": {
                              "ParameterMarker": "root"
                            }
                          }
                        },
                        {
                          "taken": [
                            [
                              {
                                "Implicit": 50000
                              },
                              {
                                "OpLike": [
                                  {
                                    "Builtin": {
                                      "AddNode": {
                                        "node_type": "Integer",
                                        "value": {
                                          "Integer": -1
                                        }
                                      }
                                    }
                                  },
                                  {
                                    "selected_input_nodes": [],
                                    "subst_to_aid": {}
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "RenameNode": {
                                  "old": {
                                    "DynamicOutputMarker": [
                                      {
                                        "Implicit": 50000
                                      },
                                      "new"
                                    ]
                                  },
                                  "new": {
                                    "Named": "temp_max"
                                  }
                                }
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Operation": 1
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "DynamicOutputMarker": [
                                          {
                                            "Custom": "shape_query_1"
                                          },
                                          "child"
                                        ]
                                      },
                                      {
                                        "Named": "temp_max"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "max_value": {
                                        "Named": "temp_max"
                                      },
                                      "root": {
                                        "DynamicOutputMarker": [
                                          {
                                            "Custom": "shape_query_1"
                                          },
                                          "child"
                                        ]
                                      }
                                    }
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "CopyValueFromTo"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "Named": "temp_max"
                                      },
                                      {
                                        "ParameterMarker": "root"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "destination": {
                                        "ParameterMarker": "root"
                                      },
                                      "source": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ],
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "DeleteNode"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "Named": "temp_max"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "target": {
                                        "Named": "temp_max"
                                      }
                                    }
                                  }
                                ]
                              }
                            ]
                          ],
                          "not_taken": [
                            [
                              null,
                              {
                                "OpLike": [
                                  {
                                    "Builtin": "DeleteNode"
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "ParameterMarker": "root"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "target": {
                                        "ParameterMarker": "root"
                                      }
                                    }
                                  }
                                ]
                              }
                            ]
                          ]
                        }
                      ]
                    }
                  ]
                ]
              }
            ]
          }
        ]
      ],
      "output_changes": {
        "new_nodes": {}
      }
    },
    "2": {
      "signature": {
        "name": "some_name",
        "parameter": {
          "explicit_input_nodes": [
            "sentinel"
          ],
          "parameter_graph": {
            "graph": {
              "nodes": [
                0
              ],
              "node_holes": [],
              "edge_property": "directed",
              "edges": []
            },
            "max_node_key": 1,
            "node_attr_map": {
              "0": {
                "node_attr": "Object"
              }
            }
          },
          "node_keys_to_subst": {
            "left_to_right": {
              "0": "sentinel"
            },
            "right_to_left": {
              "sentinel": 0
            }
          }
        },
        "output": {
          "new_nodes": {
            "max_value": "Integer"
          },
          "new_edges": {},
          "maybe_changed_nodes": {},
          "maybe_changed_edges": {},
          "maybe_deleted_nodes": [],
          "maybe_deleted_edges": []
        }
      },
      "instructions": [
        [
          {
            "Implicit": 50000
          },
          {
            "OpLike": [
              {
                "Builtin": {
                  "AddNode": {
                    "node_type": "Integer",
                    "value": {
                      "Integer": -1
                    }
                  }
                }
              },
              {
                "selected_input_nodes": [],
                "subst_to_aid": {}
              }
            ]
          }
        ],
        [
          null,
          {
            "RenameNode": {
              "old": {
                "DynamicOutputMarker": [
                  {
                    "Implicit": 50000
                  },
                  "new"
                ]
              },
              "new": {
                "Named": "max_value"
              }
            }
          }
        ],
        [
          {
            "Custom": "shape_query_0"
          },
          {
            "ShapeQuery": [
              {
                "parameter": {
                  "explicit_input_nodes": [
                    "N(0)",
                    "N(1)"
                  ],
                  "parameter_graph": {
                    "graph": {
                      "nodes": [
                        0,
                        1
                      ],
                      "node_holes": [],
                      "edge_property": "directed",
                      "edges": []
                    },
                    "max_node_key": 2,
                    "node_attr_map": {
                      "0": {
                        "node_attr": "Object"
                      },
                      "1": {
                        "node_attr": "Integer"
                      }
                    }
                  },
                  "node_keys_to_subst": {
                    "left_to_right": {
                      "0": "N(0)",
                      "1": "N(1)"
                    },
                    "right_to_left": {
                      "N(1)": 1,
                      "N(0)": 0
                    }
                  }
                },
                "expected_graph": {
                  "graph": {
                    "nodes": [
                      0,
                      1,
                      2
                    ],
                    "node_holes": [],
                    "edge_property": "directed",
                    "edges": [
                      [
                        0,
                        2,
                        {
                          "edge_attr": "Wildcard",
                          "source_out_order": 1,
                          "target_in_order": 1
                        }
                      ]
                    ]
                  },
                  "max_node_key": 3,
                  "node_attr_map": {
                    "0": {
                      "node_attr": "Object"
                    },
                    "2": {
                      "node_attr": "Integer"
                    },
                    "1": {
                      "node_attr": "Integer"
                    }
                  }
                },
                "node_keys_to_shape_idents": {
                  "left_to_right": {
                    "2": "root"
                  },
                  "right_to_left": {
                    "root": 2
                  }
                }
              },
              {
                "selected_input_nodes": [
                  {
                    "ParameterMarker": "sentinel"
                  },
                  {
                    "Named": "max_value"
                  }
                ],
                "subst_to_aid": {
                  "N(0)": {
                    "ParameterMarker": "sentinel"
                  },
                  "N(1)": {
                    "Named": "max_value"
                  }
                }
              },
              {
                "taken": [
                  [
                    null,
                    {
                      "OpLike": [
                        {
                          "Operation": 1
                        },
                        {
                          "selected_input_nodes": [
                            {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "shape_query_0"
                                },
                                "root"
                              ]
                            },
                            {
                              "Named": "max_value"
                            }
                          ],
                          "subst_to_aid": {
                            "root": {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "shape_query_0"
                                },
                                "root"
                              ]
                            },
                            "max_value": {
                              "Named": "max_value"
                            }
                          }
                        }
                      ]
                    }
                  ]
                ],
                "not_taken": []
              }
            ]
          }
        ]
      ],
      "output_changes": {
        "new_nodes": {
          "{\"Named\":\"max_value\"}": "max_value"
        }
      }
    },
    "0": {
      "signature": {
        "name": "some_name",
        "parameter": {
          "explicit_input_nodes": [
            "child"
          ],
          "parameter_graph": {
            "graph": {
              "nodes": [
                0,
                1
              ],
              "node_holes": [],
              "edge_property": "directed",
              "edges": [
                [
                  0,
                  1,
                  {
                    "edge_attr": {
                      "Exact": "child"
                    },
                    "source_out_order": 1,
                    "target_in_order": 1
                  }
                ]
              ]
            },
            "max_node_key": 2,
            "node_attr_map": {
              "1": {
                "node_attr": "Object"
              },
              "0": {
                "node_attr": "Integer"
              }
            }
          },
          "node_keys_to_subst": {
            "left_to_right": {
              "1": "parent",
              "0": "child"
            },
            "right_to_left": {
              "parent": 1,
              "child": 0
            }
          }
        },
        "output": {
          "new_nodes": {
            "new_node": "Integer"
          },
          "new_edges": {},
          "maybe_changed_nodes": {},
          "maybe_changed_edges": {},
          "maybe_deleted_nodes": [],
          "maybe_deleted_edges": []
        }
      },
      "instructions": [
        [
          {
            "Custom": "map"
          },
          {
            "OpLike": [
              {
                "Builtin": {
                  "AddNode": {
                    "node_type": "Object",
                    "value": {
                      "Integer": 1
                    }
                  }
                }
              },
              {
                "selected_input_nodes": [],
                "subst_to_aid": {}
              }
            ]
          }
        ],
        [
          {
            "Custom": "some_int_node"
          },
          {
            "OpLike": [
              {
                "Builtin": {
                  "AddNode": {
                    "node_type": "Integer",
                    "value": {
                      "Integer": 2
                    }
                  }
                }
              },
              {
                "selected_input_nodes": [],
                "subst_to_aid": {}
              }
            ]
          }
        ],
        [
          {
            "Implicit": 50000
          },
          {
            "OpLike": [
              {
                "Builtin": {
                  "AddNode": {
                    "node_type": "Integer",
                    "value": {
                      "Integer": 3
                    }
                  }
                }
              },
              {
                "selected_input_nodes": [],
                "subst_to_aid": {}
              }
            ]
          }
        ],
        [
          null,
          {
            "RenameNode": {
              "old": {
                "DynamicOutputMarker": [
                  {
                    "Implicit": 50000
                  },
                  "new"
                ]
              },
              "new": {
                "Named": "some_other_int_node"
              }
            }
          }
        ],
        [
          {
            "Custom": "shape_query_0"
          },
          {
            "ShapeQuery": [
              {
                "parameter": {
                  "explicit_input_nodes": [
                    "N(0)",
                    "N(1)",
                    "N(2)",
                    "N(3)",
                    "N(4)"
                  ],
                  "parameter_graph": {
                    "graph": {
                      "nodes": [
                        0,
                        1,
                        2,
                        3,
                        4
                      ],
                      "node_holes": [],
                      "edge_property": "directed",
                      "edges": [
                        [
                          0,
                          1,
                          {
                            "edge_attr": {
                              "Exact": "child"
                            },
                            "source_out_order": 1,
                            "target_in_order": 1
                          }
                        ]
                      ]
                    },
                    "max_node_key": 5,
                    "node_attr_map": {
                      "4": {
                        "node_attr": "Integer"
                      },
                      "1": {
                        "node_attr": "Object"
                      },
                      "2": {
                        "node_attr": "Object"
                      },
                      "3": {
                        "node_attr": "Integer"
                      },
                      "0": {
                        "node_attr": "Integer"
                      }
                    }
                  },
                  "node_keys_to_subst": {
                    "left_to_right": {
                      "2": "N(2)",
                      "1": "N(1)",
                      "3": "N(3)",
                      "4": "N(4)",
                      "0": "N(0)"
                    },
                    "right_to_left": {
                      "N(1)": 1,
                      "N(2)": 2,
                      "N(3)": 3,
                      "N(4)": 4,
                      "N(0)": 0
                    }
                  }
                },
                "expected_graph": {
                  "graph": {
                    "nodes": [
                      0,
                      1,
                      2,
                      3,
                      4,
                      5
                    ],
                    "node_holes": [],
                    "edge_property": "directed",
                    "edges": [
                      [
                        0,
                        1,
                        {
                          "edge_attr": {
                            "Exact": "child"
                          },
                          "source_out_order": 1,
                          "target_in_order": 1
                        }
                      ],
                      [
                        0,
                        5,
                        {
                          "edge_attr": {
                            "Exact": "child2"
                          },
                          "source_out_order": 2,
                          "target_in_order": 1
                        }
                      ]
                    ]
                  },
                  "max_node_key": 6,
                  "node_attr_map": {
                    "4": {
                      "node_attr": "Integer"
                    },
                    "1": {
                      "node_attr": "Object"
                    },
                    "5": {
                      "node_attr": "Integer"
                    },
                    "2": {
                      "node_attr": "Integer"
                    },
                    "3": {
                      "node_attr": "Integer"
                    },
                    "0": {
                      "node_attr": "Integer"
                    }
                  }
                },
                "node_keys_to_shape_idents": {
                  "left_to_right": {
                    "5": "some_node"
                  },
                  "right_to_left": {
                    "some_node": 5
                  }
                }
              },
              {
                "selected_input_nodes": [
                  {
                    "ParameterMarker": "child"
                  },
                  {
                    "ParameterMarker": "parent"
                  },
                  {
                    "DynamicOutputMarker": [
                      {
                        "Custom": "map"
                      },
                      "new"
                    ]
                  },
                  {
                    "DynamicOutputMarker": [
                      {
                        "Custom": "some_int_node"
                      },
                      "new"
                    ]
                  },
                  {
                    "Named": "some_other_int_node"
                  }
                ],
                "subst_to_aid": {
                  "N(4)": {
                    "Named": "some_other_int_node"
                  },
                  "N(1)": {
                    "ParameterMarker": "parent"
                  },
                  "N(3)": {
                    "DynamicOutputMarker": [
                      {
                        "Custom": "some_int_node"
                      },
                      "new"
                    ]
                  },
                  "N(2)": {
                    "DynamicOutputMarker": [
                      {
                        "Custom": "map"
                      },
                      "new"
                    ]
                  },
                  "N(0)": {
                    "ParameterMarker": "child"
                  }
                }
              },
              {
                "taken": [
                  [
                    null,
                    {
                      "RenameNode": {
                        "old": {
                          "DynamicOutputMarker": [
                            {
                              "Custom": "map"
                            },
                            "new"
                          ]
                        },
                        "new": {
                          "Named": "node_to_ret"
                        }
                      }
                    }
                  ]
                ],
                "not_taken": [
                  [
                    {
                      "Custom": "shape_query_1"
                    },
                    {
                      "ShapeQuery": [
                        {
                          "parameter": {
                            "explicit_input_nodes": [
                              "N(0)",
                              "N(1)",
                              "N(2)",
                              "N(3)",
                              "N(4)"
                            ],
                            "parameter_graph": {
                              "graph": {
                                "nodes": [
                                  0,
                                  1,
                                  2,
                                  3,
                                  4
                                ],
                                "node_holes": [],
                                "edge_property": "directed",
                                "edges": [
                                  [
                                    0,
                                    1,
                                    {
                                      "edge_attr": {
                                        "Exact": "child"
                                      },
                                      "source_out_order": 1,
                                      "target_in_order": 1
                                    }
                                  ]
                                ]
                              },
                              "max_node_key": 5,
                              "node_attr_map": {
                                "4": {
                                  "node_attr": "Integer"
                                },
                                "1": {
                                  "node_attr": "Object"
                                },
                                "2": {
                                  "node_attr": "Object"
                                },
                                "3": {
                                  "node_attr": "Integer"
                                },
                                "0": {
                                  "node_attr": "Integer"
                                }
                              }
                            },
                            "node_keys_to_subst": {
                              "left_to_right": {
                                "4": "N(4)",
                                "2": "N(2)",
                                "3": "N(3)",
                                "1": "N(1)",
                                "0": "N(0)"
                              },
                              "right_to_left": {
                                "N(4)": 4,
                                "N(1)": 1,
                                "N(2)": 2,
                                "N(3)": 3,
                                "N(0)": 0
                              }
                            }
                          },
                          "expected_graph": {
                            "graph": {
                              "nodes": [
                                0,
                                1,
                                2,
                                3,
                                4
                              ],
                              "node_holes": [],
                              "edge_property": "directed",
                              "edges": [
                                [
                                  0,
                                  1,
                                  {
                                    "edge_attr": {
                                      "Exact": "child"
                                    },
                                    "source_out_order": 1,
                                    "target_in_order": 1
                                  }
                                ]
                              ]
                            },
                            "max_node_key": 5,
                            "node_attr_map": {
                              "4": {
                                "node_attr": "Integer"
                              },
                              "1": {
                                "node_attr": "Object"
                              },
                              "2": {
                                "node_attr": "Object"
                              },
                              "3": {
                                "node_attr": "Integer"
                              },
                              "0": {
                                "node_attr": "Integer"
                              }
                            }
                          },
                          "node_keys_to_shape_idents": {
                            "left_to_right": {},
                            "right_to_left": {}
                          }
                        },
                        {
                          "selected_input_nodes": [
                            {
                              "ParameterMarker": "child"
                            },
                            {
                              "ParameterMarker": "parent"
                            },
                            {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "map"
                                },
                                "new"
                              ]
                            },
                            {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "some_int_node"
                                },
                                "new"
                              ]
                            },
                            {
                              "Named": "some_other_int_node"
                            }
                          ],
                          "subst_to_aid": {
                            "N(4)": {
                              "Named": "some_other_int_node"
                            },
                            "N(1)": {
                              "ParameterMarker": "parent"
                            },
                            "N(3)": {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "some_int_node"
                                },
                                "new"
                              ]
                            },
                            "N(0)": {
                              "ParameterMarker": "child"
                            },
                            "N(2)": {
                              "DynamicOutputMarker": [
                                {
                                  "Custom": "map"
                                },
                                "new"
                              ]
                            }
                          }
                        },
                        {
                          "taken": [
                            [
                              null,
                              {
                                "RenameNode": {
                                  "old": {
                                    "DynamicOutputMarker": [
                                      {
                                        "Custom": "some_int_node"
                                      },
                                      "new"
                                    ]
                                  },
                                  "new": {
                                    "Named": "node_to_ret"
                                  }
                                }
                              }
                            ]
                          ],
                          "not_taken": [
                            [
                              null,
                              {
                                "BuiltinQuery": [
                                  {
                                    "CmpFstSnd": -1
                                  },
                                  {
                                    "selected_input_nodes": [
                                      {
                                        "ParameterMarker": "parent"
                                      },
                                      {
                                        "ParameterMarker": "child"
                                      }
                                    ],
                                    "subst_to_aid": {
                                      "a": {
                                        "ParameterMarker": "parent"
                                      },
                                      "b": {
                                        "ParameterMarker": "child"
                                      }
                                    }
                                  },
                                  {
                                    "taken": [
                                      [
                                        null,
                                        {
                                          "RenameNode": {
                                            "old": {
                                              "DynamicOutputMarker": [
                                                {
                                                  "Custom": "some_int_node"
                                                },
                                                "new"
                                              ]
                                            },
                                            "new": {
                                              "Named": "node_to_ret"
                                            }
                                          }
                                        }
                                      ]
                                    ],
                                    "not_taken": [
                                      [
                                        null,
                                        {
                                          "RenameNode": {
                                            "old": {
                                              "Named": "some_other_int_node"
                                            },
                                            "new": {
                                              "Named": "node_to_ret"
                                            }
                                          }
                                        }
                                      ]
                                    ]
                                  }
                                ]
                              }
                            ]
                          ]
                        }
                      ]
                    }
                  ]
                ]
              }
            ]
          }
        ]
      ],
      "output_changes": {
        "new_nodes": {
          "{\"Named\":\"node_to_ret\"}": "new_node"
        }
      }
    }
  }
}
"##;

/*
Playing around with some invented syntax for the two operations:


def max_heap_remove(sentinel: Object) -> (max_value: Integer) {
    // ! syntax: take the single return value of the operation and bind to it.
    // alternative is let map = add_node(...); and then map.new is the returned node.
    let! max_value = add_node(-1);
    if shape [
        root: Integer,
        sentinel -> root: Wildcard
    ] {
        // if we have a root, we can proceed
        max_heap_remove_helper(root, max_value);
    } else {
        // do nothing
    }
    return (max_value: max_value);
}


def max_heap_remove_helper(root: Integer, max_value: Integer) {
    // return the value of the root node
    copy_value_from_to(root, max_value);
    if shape [
        left: Integer,
        root -> left: Wildcard,
        right: Integer,
        root -> right: Wildcard
    ] {
        // we have two children, check which is larger
        // method[] syntax: pass arguments to builtin operations
        if cmp_fst_snd[>](left, right) {
            // left is larger, recurse on left
            let! temp_max = add_node(-1);
            max_heap_remove_helper(left, temp_max);
            copy_value_from_to(temp_max, root);
            remove_node(temp_max);
        } else {
            // right is larger or equal, recurse on right
            let! temp_max = add_node(-1);
            max_heap_remove_helper(right, temp_max);
            copy_value_from_to(temp_max, root);
            remove_node(temp_max);
        }
    } else if shape [
        child: Integer,
        root -> child: Wildcard
    ] {
        // we have one child, recurse on it
        let! temp_max = add_node(int(-1));
        max_heap_remove_helper(child, temp_max);
        copy_value_from_to(temp_max, root);
        remove_node(temp_max);
    } else {
        // no children, we can delete the root node
        remove_node(root);
    }
}
*/
