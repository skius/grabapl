use crate::Graph;
use petgraph::dot;
use petgraph::dot::Dot;
use petgraph::prelude::DiGraphMap;
use std::fmt::Debug;
use std::hash::RandomState;

impl<NA: Debug, EA: Debug> Graph<NA, EA> {
    pub fn dot(&self) -> String
    where
        NA: Debug,
        EA: Debug,
    {
        // TODO: accept a mapping from NodeKey to String that we could accept here
        //  and then use in the operation builder to print AIDs instead of NodeKeys?
        //  also, it would be really nice if petgraph didn't require a Debug bound on Dot...

        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|_, (_src, _dst, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    let src_order = attr.source_out_order;
                    let target_order = attr.target_in_order;
                    format!("label = \"{dbg_attr_replaced},src:{src_order},dst:{target_order}\"")
                },
                &|_, (node, _)| {
                    let node_attr = self.node_attr_map.get(&node).unwrap();
                    let dbg_attr_format = format!("{:?}", node_attr.node_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("label = \"{node:?}|{dbg_attr_replaced}\"")
                }
            )
        )
    }
}

impl<NA, EA> Graph<NA, EA> {
    pub fn shape_dot(&self) -> String {
        // TODO: add petgraph changes that make this more efficient (expose non-debug-restricted DOT generation)
        let mut graph_without_edge_attrs: DiGraphMap<_, _, RandomState> = DiGraphMap::new();
        for key in self.graph.nodes() {
            graph_without_edge_attrs.add_node(key);
        }
        for (src, dst, _) in self.graph.all_edges() {
            graph_without_edge_attrs.add_edge(src, dst, ());
        }

        format!(
            "{:?}",
            Dot::new(
                &graph_without_edge_attrs,
                // &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                // &|g, (src, target, attr)| {
                //     // let src_order = attr.source_out_order;
                //     // let target_order = attr.target_in_order;
                //     // format!("src:{src_order},dst:{target_order}\"")
                //     format!("")
                // },
                // &|g, (node, _)| {
                //     format!("label = \"{node}\"")
                // }
            )
        )
    }
}

pub struct DotCollector {
    dot: String,
}

impl Default for DotCollector {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn collect_shape<NA, EA>(&mut self, graph: &Graph<NA, EA>) {
        if !self.dot.is_empty() {
            self.dot.push_str("\n---\n");
        }
        self.dot.push_str(&graph.shape_dot());
    }

    pub fn collect_raw(&mut self, raw_dot: &str) {
        if !self.dot.is_empty() {
            self.dot.push_str("\n---\n");
        }
        self.dot.push_str(raw_dot);
    }

    pub fn finalize(&self) -> String {
        self.dot.clone()
    }
}
