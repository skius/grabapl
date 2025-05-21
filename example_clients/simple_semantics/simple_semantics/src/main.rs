use std::collections::HashMap;
use grabapl::{DotCollector, OperationContext, Semantics, WithSubstMarker};
use grabapl::graph::operation::run_operation;
use grabapl::graph::operation::user_defined::{AbstractNodeId, Instruction, UserDefinedOperation};
use grabapl::graph::pattern::OperationParameter;
use simple_semantics::{BuiltinOperation, SimpleSemantics};

fn get_sample_user_defined_operation() -> UserDefinedOperation<SimpleSemantics> {
    // Expects a child
    let mut g = grabapl::graph::Graph::new();
    let a = g.add_node(WithSubstMarker::new(0, ()));
    let param = OperationParameter {
        explicit_input_nodes: vec![0],
        parameter_graph: g,
        subst_to_node_keys: HashMap::from([(0, a)]),
    };

    let input_node = AbstractNodeId::ParameterSubstMarker(0);

    let mut instructions = vec![];
    // TODO: How do I access the node key that represents that new child?
    instructions.push(("first_child", Instruction::Operation(1, vec![input_node])));
    instructions.push(("second_child", Instruction::Operation(1, vec![input_node])));
    instructions.push(("third_child", Instruction::Operation(1, vec![input_node])));
    instructions.push(("fourth_child", Instruction::Operation(1, vec![input_node])));

    let second_id = AbstractNodeId::DynamicOutputSubstMarker("second_child", 0);
    let third_id = AbstractNodeId::DynamicOutputSubstMarker("third_child", 0);
    let fourth_id = AbstractNodeId::DynamicOutputSubstMarker("fourth_child", 0);

    instructions.push(("TODO ignore", Instruction::Operation(4, vec![fourth_id, third_id])));
    instructions.push(("TODO ignore", Instruction::Operation(4, vec![third_id, second_id])));
    instructions.push(("TODO ignore", Instruction::Operation(4, vec![second_id, fourth_id])));
    // TODO cycle label?
    instructions.push(("TODO ignore", Instruction::Operation(5, vec![second_id, fourth_id])));


    instructions.push(("TODO ignore me", Instruction::Operation(2, vec![
        fourth_id
    ])));

    UserDefinedOperation {
        parameter: param,
        instructions,
    }
}

fn main() {
    let user_defined_op = get_sample_user_defined_operation();

    let operation_ctx = HashMap::from([
        (0, BuiltinOperation::AddNode),
        (1, BuiltinOperation::AppendChild),
        (2, BuiltinOperation::IndexCycle),
        (4, BuiltinOperation::AddEdge),
        (5, BuiltinOperation::SetEdgeValueToCycle),
    ]);
    let mut operation_ctx = OperationContext::from_builtins(operation_ctx);
    operation_ctx.add_custom_operation(3, user_defined_op);

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



    println!("{}", dot_collector.finalize());
}