use std::cmp::Ordering;
use petgraph::dot::Dot;
use petgraph::prelude::{DiGraphMap, GraphMap, StableDiGraph};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use petgraph::visit::{IntoEdgesDirected, IntoNeighborsDirected, NodeRef};
use petgraph::{Direction, dot};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct NodeAttribute<NodeAttr> {
    pub node_attr: NodeAttr,
    // Additional attributes can be added here
}

impl<NodeAttr> NodeAttribute<NodeAttr> {
    pub fn new(node_attr: NodeAttr) -> Self {
        NodeAttribute { node_attr }
    }
}

#[derive(Debug)]
pub struct EdgeAttribute<EdgeAttr> {
    pub edge_attr: EdgeAttr,
    // Additional attributes can be added here
    source_out_order: EdgeOrder,
    target_out_order: EdgeOrder,
}

impl<EdgeAttr> EdgeAttribute<EdgeAttr> {
    pub fn new(edge_attr: EdgeAttr, source_out_order: EdgeOrder, target_out_order: EdgeOrder) -> Self {
        EdgeAttribute { edge_attr, source_out_order, target_out_order }
    }
}

type EdgeOrder = i32;

type NodeKey = u32;
type EdgeKey = (NodeKey, NodeKey);

#[derive(Debug, Copy, Clone)]
pub enum EdgeInsertionOrder {
    Append,
    Prepend,
}

pub struct ConcreteGraph<NodeAttr, EdgeAttr> {
    graph: DiGraphMap<NodeKey, EdgeAttribute<EdgeAttr>>,
    max_node_key: NodeKey,
    node_attr_map: HashMap<NodeKey, NodeAttribute<NodeAttr>>,
}

impl<NodeAttr, EdgeAttr> ConcreteGraph<NodeAttr, EdgeAttr> {
    pub fn new() -> Self {
        ConcreteGraph {
            graph: GraphMap::new(),
            max_node_key: 0,
            node_attr_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_attr: NodeAttr) -> NodeKey {
        let node_key = self.max_node_key;
        let node_key = self.graph.add_node(node_key);
        self.node_attr_map
            .insert(node_key, NodeAttribute::new(node_attr));
        self.max_node_key += 1;
        node_key
    }

    /// Returns the old `EdgeAttr` if it exists, otherwise returns `None`.
    ///
    /// Same as `add_edge_ordered` with `Append` for both source and target.
    pub fn add_edge(
        &mut self,
        source: NodeKey,
        target: NodeKey,
        edge_attr: EdgeAttr,
    ) -> Option<EdgeAttr> {
        self.add_edge_ordered(
            source,
            target,
            edge_attr,
            EdgeInsertionOrder::Append,
            EdgeInsertionOrder::Append,
        )
    }

    fn extremum_out_edge_order_key(
        &self,
        source: NodeKey,
        wanted_order: Ordering,
        direction: Direction,
    ) -> Option<EdgeOrder> {
        let mut extremum_order = None;
        for (_, _, edge_attr) in self.graph.edges_directed(source, direction) {
            let order = if direction == Direction::Outgoing {
                edge_attr.source_out_order
            } else {
                edge_attr.target_out_order
            };
            if extremum_order.is_none() || order.cmp(&extremum_order.unwrap()) == wanted_order {
                extremum_order = Some(order);
            }
        }
        extremum_order
    }

    fn max_out_edge_order_key(
        &self,
        source: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(source, Ordering::Greater, Direction::Outgoing)
    }

    fn min_out_edge_order_key(
        &self,
        source: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(source, Ordering::Less, Direction::Outgoing)
    }

    fn max_in_edge_order_key(
        &self,
        target: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(target, Ordering::Greater, Direction::Incoming)
    }

    fn min_in_edge_order_key(
        &self,
        target: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(target, Ordering::Less, Direction::Incoming)
    }


    pub fn add_edge_ordered(
        &mut self,
        source: NodeKey,
        target: NodeKey,
        edge_attr: EdgeAttr,
        source_out_order: EdgeInsertionOrder,
        target_in_order: EdgeInsertionOrder,
    ) -> Option<EdgeAttr> {

        let new_out_order = match source_out_order {
            EdgeInsertionOrder::Append => {
                self.max_out_edge_order_key(source).unwrap_or(0) + 1
            }
            EdgeInsertionOrder::Prepend => {
                self.min_out_edge_order_key(source).unwrap_or(0) - 1
            }
        };
        let new_in_order = match target_in_order {
            EdgeInsertionOrder::Append => {
                self.max_in_edge_order_key(target).unwrap_or(0) + 1
            }
            EdgeInsertionOrder::Prepend => {
                self.min_in_edge_order_key(target).unwrap_or(0) - 1
            }
        };

        let old_attr = self
            .graph
            .add_edge(source, target, EdgeAttribute::new(edge_attr, new_out_order, new_in_order));
        old_attr.map(|attr| attr.edge_attr)
    }

    fn neighbors_out_ordered(
        &self,
        source: NodeKey,
    ) -> Vec<NodeKey> {
        let mut neighbors = self
            .graph
            .edges_directed(source, Direction::Outgoing)
            .collect::<Vec<_>>();
        neighbors.sort_by(|(_, _, e1), (_, _, e2)| {
            e1.source_out_order.cmp(&e2.source_out_order)
        });
        neighbors.into_iter()
            .map(|(_, target, _)| target)
            .collect()
    }

    pub fn next_outgoing_edge(&self, source: NodeKey, (_, curr_target): EdgeKey) -> EdgeKey {
        let outgoing_neighbors = self.neighbors_out_ordered(source);
        let curr_idx = outgoing_neighbors
            .iter()
            .position(|&target| target == curr_target)
            .unwrap_or(0);
        let next_idx = (curr_idx + 1) % outgoing_neighbors.len();
        let next_target = outgoing_neighbors[next_idx];
        (source, next_target)
    }

    pub fn prev_outgoing_edge(&self, source: NodeKey, (_, curr_target): EdgeKey) -> EdgeKey {
        let outgoing_neighbors = self.neighbors_out_ordered(source);
        let curr_idx = outgoing_neighbors
            .iter()
            .position(|&target| target == curr_target)
            .unwrap_or(0);
        let prev_idx = if curr_idx == 0 {
            outgoing_neighbors.len() - 1
        } else {
            curr_idx - 1
        };
        let prev_target = outgoing_neighbors[prev_idx];
        (source, prev_target)
    }

    pub fn remove_node(&mut self, node_key: NodeKey) -> Option<NodeAttr> {
        if let Some(node_attr) = self.node_attr_map.remove(&node_key) {
            self.graph.remove_node(node_key);
            Some(node_attr.node_attr)
        } else {
            None
        }
    }

    pub fn remove_edge_between(&mut self, source: NodeKey, target: NodeKey) -> Option<EdgeAttr> {
        self.remove_edge((source, target))
    }

    pub fn remove_edge(&mut self, (src, target): EdgeKey) -> Option<EdgeAttr> {
        self.graph
            .remove_edge(src, target)
            .map(|attr| attr.edge_attr)
    }

    pub fn get_edge_attr(&self, (src, target): EdgeKey) -> Option<&EdgeAttr> {
        self.graph
            .edge_weight(src, target)
            .map(|attr| &attr.edge_attr)
    }

    pub fn get_mut_edge_attr(&mut self, (src, target): EdgeKey) -> Option<&mut EdgeAttr> {
        self.graph
            .edge_weight_mut(src, target)
            .map(|attr| &mut attr.edge_attr)
    }

    pub fn get_node_attr(&self, node_key: NodeKey) -> Option<&NodeAttr> {
        self.node_attr_map
            .get(&node_key)
            .map(|attr| &attr.node_attr)
    }

    pub fn get_mut_node_attr(&mut self, node_key: NodeKey) -> Option<&mut NodeAttr> {
        self.node_attr_map
            .get_mut(&node_key)
            .map(|attr| &mut attr.node_attr)
    }

    pub fn dot(&self) -> String
    where
        EdgeAttr: Debug,
        NodeAttr: Debug,
    {
        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|g, (src, target, attr)| {
                    // TODO: also escape here
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    let src_order = attr.source_out_order;
                    let target_order = attr.target_out_order;
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

#[cfg(test)]
mod tests {
    use super::*;

    struct DotCollector {
        dot: String,
    }

    impl DotCollector {
        fn new() -> Self {
            DotCollector {
                dot: String::new(),
            }
        }

        fn collect(&mut self, graph: &ConcreteGraph<&str, ()>) {
            if !self.dot.is_empty() {
                self.dot.push_str("\n---\n");
            }
            self.dot.push_str(&graph.dot());
        }

        fn finalize(&self) -> String {
            self.dot.clone()
        }
    }

    #[test]
    fn child_order() {
        let mut collector = DotCollector::new();
        let mut graph = ConcreteGraph::<&str, ()>::new();


        macro_rules! c {
            () => {
                collector.collect(&graph);
            };
        }

        c!();
        let a = graph.add_node("hello");
        c!();
        let b = graph.add_node("world");
        c!();

        graph.add_edge(a, b, ());
        c!();

        let next_edge = graph.next_outgoing_edge(a, (a,b));
        assert_eq!(next_edge, (a, b));

        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, b));

        let c = graph.add_node("foo");
        c!();
        graph.add_edge(a, c, ());
        c!();
        let next_edge = graph.next_outgoing_edge(a, (a,b));
        assert_eq!(next_edge, (a, c));
        let prev_edge = graph.prev_outgoing_edge(a, (a,c));
        assert_eq!(prev_edge, (a, b));
        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, c));

        let d = graph.add_node("bar");
        c!();
        graph.add_edge(a, d, ());
        c!();
        let next_edge = graph.next_outgoing_edge(a, (a,c));
        assert_eq!(next_edge, (a, d));
        let next_edge = graph.next_outgoing_edge(a, (a,d));
        assert_eq!(next_edge, (a, b));
        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, d));

        let before_b = graph.add_node("before_b");
        c!();
        graph.add_edge_ordered(a, before_b, (), EdgeInsertionOrder::Prepend, EdgeInsertionOrder::Append);
        c!();

        let after_d = graph.add_node("after_d");
        c!();
        graph.add_edge_ordered(a, after_d, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
        c!();

        let next_edge = graph.next_outgoing_edge(a, (a,b));
        assert_eq!(next_edge, (a, c));
        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, before_b));
        let next_edge = graph.next_outgoing_edge(a, (a,d));
        assert_eq!(next_edge, (a, after_d));
        let prev_edge = graph.prev_outgoing_edge(a, (a,before_b));
        assert_eq!(prev_edge, (a, after_d));


        // TODO: Could have a "add_edge_ordered" function that takes an Append/Prepend enum for both the outgoing order of source, and incoming order of target.
        // An edge could contain a "source order" Ord and a "target order" Ord, and nodes would order edges according to those values.

        let dot = collector.finalize();
        println!("{}", dot);
        assert!(false);
    }
}
