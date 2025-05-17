use std::collections::HashMap;
use grabapl::{DotCollector, OperationContext, Semantics};
use grabapl::graph::operation::run_operation;
use simple_semantics::{BuiltinOperation, SimpleSemantics};

fn main() {
    let operation_ctx = HashMap::from([
        (0, BuiltinOperation::AddNode),
        (1, BuiltinOperation::AppendChild),
        (2, BuiltinOperation::IndexCycle),
    ]);
    let operation_ctx = OperationContext::from_builtins(operation_ctx);

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



    println!("{}", dot_collector.finalize());
}