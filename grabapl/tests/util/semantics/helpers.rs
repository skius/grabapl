use grabapl::NodeKey;
use grabapl::prelude::ConcreteGraph;
use crate::util::semantics::{NodeValue, TestSemantics};

pub fn list_to_value_vec(graph: &ConcreteGraph<TestSemantics>, head: NodeKey) -> Vec<NodeValue> {
    let mut values = vec![];
    let mut current = Some(head);
    while let Some(current_key) = current.take() {
        let val = graph.get_node_attr(current_key).unwrap();
        values.push(val.clone());

        // get next node in the list, if one exists
        let mut out_nodes_current = graph.out_edges(current_key);
        if let Some((next_node, _)) = out_nodes_current.next() {
            current = Some(next_node);
        }
    }
    values
}