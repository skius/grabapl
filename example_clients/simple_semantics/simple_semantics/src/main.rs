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

fn main() {
    let user_defined_op = get_sample_user_defined_operation();
    let mk_list_user_op = get_mk_n_to_0_list_user_defined_operation();

    let count_list_len_user_op = get_count_list_len_user_defined_operation(11);

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


    println!("{}", dot_collector.finalize());
}