use crate::Graph;
use petgraph::dot;
use petgraph::dot::Dot;
use std::fmt::Debug;

impl<NA: Debug, EA: Debug> Graph<NA, EA> {
    pub fn dot(&self) -> String
    where
        NA: Debug,
        EA: Debug,
    {
        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|g, (src, target, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    let src_order = attr.source_out_order;
                    let target_order = attr.target_in_order;
                    format!("label = \"{dbg_attr_replaced},src:{src_order},dst:{target_order}\"")
                },
                &|g, (node, _)| {
                    let node_attr = self.node_attr_map.get(&node).unwrap();
                    let dbg_attr_format = format!("{:?}", node_attr.node_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("label = \"{node}|{dbg_attr_replaced}\"")
                }
            )
        )
    }
}

pub struct DotCollector {
    dot: String,
}

impl DotCollector {
    pub fn new() -> Self {
        DotCollector { dot: String::new() }
    }

    pub fn collect<NA: Debug, EA: Debug>(&mut self, graph: &Graph<NA, EA>) {
        if !self.dot.is_empty() {
            self.dot.push_str("\n---\n");
        }
        self.dot.push_str(&graph.dot());
    }

    pub fn finalize(&self) -> String {
        self.dot.clone()
    }
}
