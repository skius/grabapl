use derive_more::From;
use petgraph::Direction;
use petgraph::graphmap::{DiGraphMap, GraphMap};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::RandomState;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

pub mod dot;

pub use crate::operation::OperationContext;
pub use crate::operation::OperationId;
pub use crate::semantics::Semantics;
pub use dot::DotCollector;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeAttribute<NodeAttr> {
    pub node_attr: NodeAttr,
    // Additional attributes can be added here
}

impl<NodeAttr> NodeAttribute<NodeAttr> {
    pub fn new(node_attr: NodeAttr) -> Self {
        NodeAttribute { node_attr }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeAttribute<EdgeAttr> {
    pub edge_attr: EdgeAttr,
    // Additional attributes can be added here
    /// The order of the edge as an outgoing edge from the source node
    source_out_order: EdgeOrder,
    /// The order of the edge as an incoming edge to the target node
    target_in_order: EdgeOrder,
}

impl<EdgeAttr> EdgeAttribute<EdgeAttr> {
    pub fn new(
        edge_attr: EdgeAttr,
        source_out_order: EdgeOrder,
        target_in_order: EdgeOrder,
    ) -> Self {
        EdgeAttribute {
            edge_attr,
            source_out_order,
            target_in_order,
        }
    }

    pub fn attr(&self) -> &EdgeAttr {
        &self.edge_attr
    }

    pub fn with<NewAttr>(&self, new_attr: NewAttr) -> EdgeAttribute<NewAttr> {
        EdgeAttribute::new(new_attr, self.source_out_order, self.target_in_order)
    }
}

type EdgeOrder = i32;

#[derive(
    Hash,
    Eq,
    PartialEq,
    derive_more::Debug,
    Clone,
    Copy,
    PartialOrd,
    Ord,
    derive_more::Add,
    derive_more::AddAssign,
    From,
)]
#[debug("N({_0})")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeKey(pub u32);
pub type EdgeKey = (NodeKey, NodeKey);

#[derive(Debug, Copy, Clone)]
pub enum EdgeInsertionOrder {
    Append,
    Prepend,
}

/// A graph with ordered edges and arbitrary associated edge and node data.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Graph<NodeAttr, EdgeAttr> {
    #[cfg_attr(feature = "serde", serde(bound = "EdgeAttr: Serialize + Clone + DeserializeOwned"))]
    pub(crate) graph: DiGraphMap<NodeKey, EdgeAttribute<EdgeAttr>, RandomState>,
    pub(crate) max_node_key: NodeKey,
    pub(crate) node_attr_map: HashMap<NodeKey, NodeAttribute<NodeAttr>>,
}

impl<NodeAttr, EdgeAttr> Graph<NodeAttr, EdgeAttr> {
    pub fn new() -> Self {
        Graph {
            graph: GraphMap::new(),
            max_node_key: 0.into(),
            node_attr_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_attr: NodeAttr) -> NodeKey {
        let node_key = self.max_node_key;
        let node_key = self.graph.add_node(node_key);
        self.node_attr_map
            .insert(node_key, NodeAttribute::new(node_attr));
        self.max_node_key += 1.into();
        node_key
    }

    /// Returns the old `EdgeAttr` if it exists, otherwise returns `None`.
    ///
    /// Same as `add_edge_ordered` with `Append` for both source and target.
    pub fn add_edge(
        &mut self,
        source: impl Into<NodeKey>,
        target: impl Into<NodeKey>,
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

    pub fn nodes(&self) -> impl Iterator<Item = (NodeKey, &NodeAttr)> {
        self.graph.nodes().map(|node_key| {
            (
                node_key,
                &self
                    .node_attr_map
                    .get(&node_key)
                    .as_ref()
                    .unwrap()
                    .node_attr,
            )
        })
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
                edge_attr.target_in_order
            };
            if extremum_order.is_none() || order.cmp(&extremum_order.unwrap()) == wanted_order {
                extremum_order = Some(order);
            }
        }
        extremum_order
    }

    fn max_out_edge_order_key(&self, source: NodeKey) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(source, Ordering::Greater, Direction::Outgoing)
    }

    fn min_out_edge_order_key(&self, source: NodeKey) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(source, Ordering::Less, Direction::Outgoing)
    }

    fn max_in_edge_order_key(&self, target: NodeKey) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(target, Ordering::Greater, Direction::Incoming)
    }

    fn min_in_edge_order_key(&self, target: NodeKey) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(target, Ordering::Less, Direction::Incoming)
    }

    pub fn add_edge_ordered(
        &mut self,
        source: impl Into<NodeKey>,
        target: impl Into<NodeKey>,
        edge_attr: EdgeAttr,
        source_out_order: EdgeInsertionOrder,
        target_in_order: EdgeInsertionOrder,
    ) -> Option<EdgeAttr> {
        let source = source.into();
        let target = target.into();
        let new_out_order = match source_out_order {
            EdgeInsertionOrder::Append => self.max_out_edge_order_key(source).unwrap_or(0) + 1,
            EdgeInsertionOrder::Prepend => self.min_out_edge_order_key(source).unwrap_or(0) - 1,
        };
        let new_in_order = match target_in_order {
            EdgeInsertionOrder::Append => self.max_in_edge_order_key(target).unwrap_or(0) + 1,
            EdgeInsertionOrder::Prepend => self.min_in_edge_order_key(target).unwrap_or(0) - 1,
        };

        let old_attr = self.graph.add_edge(
            source,
            target,
            EdgeAttribute::new(edge_attr, new_out_order, new_in_order),
        );
        old_attr.map(|attr| attr.edge_attr)
    }

    fn neighbors_out_ordered(&self, source: NodeKey) -> Vec<NodeKey> {
        let mut neighbors = self
            .graph
            .edges_directed(source, Direction::Outgoing)
            .collect::<Vec<_>>();
        neighbors.sort_by(|(_, _, e1), (_, _, e2)| e1.source_out_order.cmp(&e2.source_out_order));
        neighbors.into_iter().map(|(_, target, _)| target).collect()
    }

    fn neighbors_in_ordered(&self, target: NodeKey) -> Vec<NodeKey> {
        let mut neighbors = self
            .graph
            .edges_directed(target, Direction::Incoming)
            .collect::<Vec<_>>();
        neighbors.sort_by(|(_, _, e1), (_, _, e2)| e1.target_in_order.cmp(&e2.target_in_order));
        neighbors.into_iter().map(|(source, _, _)| source).collect()
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

    /// Sets the node attribute for the given node key that already exists in the graph.
    pub fn set_node_attr(&mut self, node_key: NodeKey, node_attr: NodeAttr) -> Option<NodeAttr> {
        if let Some(attr) = self.node_attr_map.get_mut(&node_key) {
            let old_attr = std::mem::replace(&mut attr.node_attr, node_attr);
            Some(old_attr)
        } else {
            None
        }
    }

    /// Sets the edge attribute for the given edge key that already exists in the graph.
    pub fn set_edge_attr(
        &mut self,
        (src, target): EdgeKey,
        edge_attr: EdgeAttr,
    ) -> Option<EdgeAttr> {
        if let Some(ea) = self.graph.edge_weight_mut(src, target) {
            let old_attr = std::mem::replace(&mut ea.edge_attr, edge_attr);
            Some(old_attr)
        } else {
            None
        }
    }
}

pub trait GraphTrait {
    type NodeAttr;
    type EdgeAttr;

    fn add_node(&mut self, node_attr: Self::NodeAttr) -> NodeKey;
    fn delete_node(&mut self, node_key: NodeKey) -> Option<Self::NodeAttr>;
    fn add_edge(
        &mut self,
        source: NodeKey,
        target: NodeKey,
        edge_attr: Self::EdgeAttr,
    ) -> Option<Self::EdgeAttr>;
    fn delete_edge(&mut self, source: NodeKey, target: NodeKey) -> Option<Self::EdgeAttr>;
    fn get_node_attr(&self, node_key: NodeKey) -> Option<&Self::NodeAttr>;
    fn get_mut_node_attr(&mut self, node_key: NodeKey) -> Option<&mut Self::NodeAttr>;
    /// Sets the node attribute for the given node key that already exists in the graph.
    fn set_node_attr(
        &mut self,
        node_key: NodeKey,
        node_attr: Self::NodeAttr,
    ) -> Option<Self::NodeAttr>;
    fn get_edge_attr(&self, edge_key: EdgeKey) -> Option<&Self::EdgeAttr>;
    fn get_mut_edge_attr(&mut self, edge_key: EdgeKey) -> Option<&mut Self::EdgeAttr>;
    /// Sets the edge attribute for the given edge key that already exists in the graph.
    fn set_edge_attr(
        &mut self,
        edge_key: EdgeKey,
        edge_attr: Self::EdgeAttr,
    ) -> Option<Self::EdgeAttr>;

    fn edges(&self) -> impl Iterator<Item = (NodeKey, NodeKey, &Self::EdgeAttr)>;
}

impl<NodeAttr, EdgeAttr> GraphTrait for Graph<NodeAttr, EdgeAttr> {
    type NodeAttr = NodeAttr;
    type EdgeAttr = EdgeAttr;

    fn add_node(&mut self, node_attr: Self::NodeAttr) -> NodeKey {
        self.add_node(node_attr)
    }

    fn delete_node(&mut self, node_key: NodeKey) -> Option<Self::NodeAttr> {
        self.remove_node(node_key)
    }

    fn add_edge(
        &mut self,
        source: NodeKey,
        target: NodeKey,
        edge_attr: Self::EdgeAttr,
    ) -> Option<Self::EdgeAttr> {
        self.add_edge(source, target, edge_attr)
    }

    fn delete_edge(&mut self, source: NodeKey, target: NodeKey) -> Option<Self::EdgeAttr> {
        self.remove_edge_between(source, target)
    }

    fn get_node_attr(&self, node_key: NodeKey) -> Option<&Self::NodeAttr> {
        self.get_node_attr(node_key)
    }

    fn get_mut_node_attr(&mut self, node_key: NodeKey) -> Option<&mut Self::NodeAttr> {
        self.get_mut_node_attr(node_key)
    }

    fn set_node_attr(
        &mut self,
        node_key: NodeKey,
        node_attr: Self::NodeAttr,
    ) -> Option<Self::NodeAttr> {
        self.set_node_attr(node_key, node_attr)
    }

    fn get_edge_attr(&self, edge_key: EdgeKey) -> Option<&Self::EdgeAttr> {
        self.get_edge_attr(edge_key)
    }

    fn get_mut_edge_attr(&mut self, edge_key: EdgeKey) -> Option<&mut Self::EdgeAttr> {
        self.get_mut_edge_attr(edge_key)
    }

    fn set_edge_attr(
        &mut self,
        edge_key: EdgeKey,
        edge_attr: Self::EdgeAttr,
    ) -> Option<Self::EdgeAttr> {
        self.set_edge_attr(edge_key, edge_attr)
    }

    fn edges(&self) -> impl Iterator<Item = (NodeKey, NodeKey, &Self::EdgeAttr)> {
        self.graph
            .all_edges()
            .map(|(src, target, attr)| (src, target, &attr.edge_attr))
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn subgraph_isomorphism_test() {
    //     let mut big_graph = Graph::<&str, ()>::new();
    //     let a = big_graph.add_node("A");
    //     let b = big_graph.add_node("B");
    //     let c = big_graph.add_node("C");
    //     big_graph.remove_node(c);
    //     let d = big_graph.add_node("D");
    //     let c = big_graph.add_node("C");
    //     println!("c: {}", c);
    //     big_graph.add_edge(a, b, ());
    //     big_graph.add_edge(b, c, ());
    //     big_graph.add_edge(c, a, ());
    //
    //     let mut query_graph = Graph::<&str, ()>::new();
    //     let x = query_graph.add_node("X");
    //     let y = query_graph.add_node("Y");
    //     query_graph.add_edge(x, y, ());
    //
    //     let query = &query_graph.graph;
    //     let big = &big_graph.graph;
    //     let mut nm = |_: &_, _: &_| true;
    //     let mut em = |_: &_, _: &_| true;
    //     let isomorphisms = subgraph_isomorphisms_iter(&query, &big, &mut nm, &mut em);
    //
    //     let big_nodes = big_graph.graph.nodes().collect::<Vec<_>>();
    //     let query_nodes = query_graph.graph.nodes().collect::<Vec<_>>();
    //
    //     fn mapping_from_vec(
    //         big_nodes: &[NodeKey],
    //         query_nodes: &[NodeKey],
    //         index_mapping: &[usize],
    //     ) -> HashMap<NodeKey, NodeKey> {
    //         let mut mapping = HashMap::new();
    //         for (src, target) in index_mapping.into_iter().copied().enumerate() {
    //             let src_node = query_nodes[src];
    //             let target_node = big_nodes[target];
    //             mapping.insert(src_node, target_node);
    //         }
    //         mapping
    //     }
    //
    //     for isomorphism in isomorphisms.unwrap() {
    //         println!("Isomorphism raw: {:?}", isomorphism);
    //         let mapped = mapping_from_vec(&big_nodes, &query_nodes, isomorphism.as_ref());
    //         println!("Isomorphism mapped: {:?}", mapped);
    //         let attr_map_list = mapped
    //             .into_iter()
    //             .map(|(src, target)| {
    //                 let src_attr = query_graph.get_node_attr(src).unwrap();
    //                 let target_attr = big_graph.get_node_attr(target).unwrap();
    //                 (src_attr, target_attr)
    //             })
    //             .collect::<Vec<_>>();
    //         println!("Isomorphism attr map: {:?}", attr_map_list);
    //     }
    //
    //     let mut big_graph = Graph::<&str, ()>::new();
    //     let a = big_graph.add_node("A");
    //     let b = big_graph.add_node("B");
    //     let c = big_graph.add_node("C");
    //     let d = big_graph.add_node("D");
    //
    //     big_graph.add_edge_ordered(
    //         a,
    //         b,
    //         (),
    //         EdgeInsertionOrder::Append,
    //         EdgeInsertionOrder::Append,
    //     );
    //     big_graph.add_edge_ordered(
    //         a,
    //         c,
    //         (),
    //         EdgeInsertionOrder::Append,
    //         EdgeInsertionOrder::Append,
    //     );
    //     big_graph.add_edge_ordered(
    //         a,
    //         d,
    //         (),
    //         EdgeInsertionOrder::Prepend,
    //         EdgeInsertionOrder::Append,
    //     );
    //
    //     big_graph.add_edge_ordered(
    //         d,
    //         c,
    //         (),
    //         EdgeInsertionOrder::Append,
    //         EdgeInsertionOrder::Append,
    //     );
    //
    //     // a has ordered children d,b,c
    //
    //     let mut query_graph = Graph::<&str, ()>::new();
    //     let x = query_graph.add_node("X");
    //     let y = query_graph.add_node("Y");
    //     let z = query_graph.add_node("Z");
    //     query_graph.add_edge_ordered(
    //         x,
    //         y,
    //         (),
    //         EdgeInsertionOrder::Append,
    //         EdgeInsertionOrder::Append,
    //     );
    //     query_graph.add_edge_ordered(
    //         x,
    //         z,
    //         (),
    //         EdgeInsertionOrder::Append,
    //         EdgeInsertionOrder::Append,
    //     );
    //     query_graph.add_edge_ordered(
    //         y,
    //         z,
    //         (),
    //         EdgeInsertionOrder::Append,
    //         EdgeInsertionOrder::Append,
    //     );
    //
    //     let big = &big_graph.graph;
    //     let query = &query_graph.graph;
    //
    //     let isomorphisms = subgraph_isomorphisms_iter(&query, &big, &mut nm, &mut em);
    //
    //     let big_nodes = big_graph.graph.nodes().collect::<Vec<_>>();
    //     let query_nodes = query_graph.graph.nodes().collect::<Vec<_>>();
    //     println!("----");
    //     for isomorphism in isomorphisms.unwrap() {
    //         println!("Isomorphism raw: {:?}", isomorphism);
    //         let mapped = mapping_from_vec(&big_nodes, &query_nodes, isomorphism.as_ref());
    //         println!("Isomorphism mapped: {:?}", mapped);
    //         let mut attr_map_list = mapped
    //             .into_iter()
    //             .map(|(src, target)| {
    //                 let src_attr = query_graph.get_node_attr(src).unwrap();
    //                 let target_attr = big_graph.get_node_attr(target).unwrap();
    //                 (src_attr, target_attr)
    //             })
    //             .collect::<Vec<_>>();
    //         attr_map_list.sort_by(|(src1, _), (src2, _)| src1.cmp(src2));
    //         println!("Isomorphism attr map: {:?}", attr_map_list);
    //     }
    //
    //     println!("----");
    //
    //     let mappings = big_graph.match_to_pattern(&query_graph, &mut nm, &mut em);
    //     for mapping in mappings.unwrap() {
    //         let mut attr_map_list = mapping
    //             .iter()
    //             .map(|(src, target)| {
    //                 let src_attr = query_graph.get_node_attr(*src).unwrap();
    //                 let target_attr = big_graph.get_node_attr(*target).unwrap();
    //                 (src_attr, target_attr)
    //             })
    //             .collect::<Vec<_>>();
    //         attr_map_list.sort_by(|(src1, _), (src2, _)| src1.cmp(src2));
    //         println!("Isomorphism attr map: {:?}", attr_map_list);
    //     }
    //
    //     assert!(false);
    // }
}
