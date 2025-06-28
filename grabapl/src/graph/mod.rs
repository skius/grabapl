use petgraph::Direction;
use petgraph::algo::{general_subgraph_monomorphisms_iter, subgraph_isomorphisms_iter};
use petgraph::dot::Dot;
use petgraph::graphmap::{DiGraphMap, GraphMap};
use petgraph::visit::NodeIndexable;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::RandomState;
use derive_more::From;

pub mod dot;
pub mod operation;
pub mod pattern;
pub mod semantics;

pub use dot::DotCollector;
pub use operation::OperationContext;
pub use operation::OperationId;
pub use semantics::Semantics;

#[derive(Debug, Clone)]
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
}

type EdgeOrder = i32;

#[derive(Hash, Eq, PartialEq, derive_more::Debug, Clone, Copy, PartialOrd, Ord, derive_more::Add, derive_more::AddAssign, From)]
#[debug("N({_0})")]
pub struct NodeKey(pub u32);
pub type EdgeKey = (NodeKey, NodeKey);

#[derive(Debug, Copy, Clone)]
pub enum EdgeInsertionOrder {
    Append,
    Prepend,
}

/// A graph with ordered edges and arbitrary associated edge and node data.
#[derive(Clone, Debug)]
pub struct Graph<NodeAttr, EdgeAttr> {
    graph: DiGraphMap<NodeKey, EdgeAttribute<EdgeAttr>, RandomState>,
    max_node_key: NodeKey,
    node_attr_map: HashMap<NodeKey, NodeAttribute<NodeAttr>>,
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

    // TODO: delete. outdated.
    // /// Attempts to match the pattern to the graph on the specified inputs.
    // ///
    // /// `inputs` is the ordered list of concrete nodes from `self` that need to match up with `pattern.parameter_nodes`.
    // ///
    // /// The return value is a mapping from the pattern node keys to the graph node keys if a match is found.
    // pub fn try_match_pattern<NAP, EAP>(
    //     &self,
    //     inputs: &[NodeKey],
    //     pattern: &InputPattern<NAP, EAP>,
    //     // nm: &mut NM,
    //     // em: &mut EM,
    // ) -> Option<HashMap<NodeKey, NodeKey>>
    // where
    //     NAP: PatternAttributeMatcher<Attr = NodeAttr>,
    //     EAP: PatternAttributeMatcher<Attr = EdgeAttr>,
    //     // NM: FnMut(&NodeKey, &NodeKey) -> bool,
    //     // EM: FnMut(&EdgeAttribute<EAP::Pattern>, &EdgeAttribute<EdgeAttr>) -> bool,
    // {
    //     let mut expected_input_mapping = HashMap::new();
    //     for (&param_marker, &input_node) in pattern.parameter_nodes.iter().zip(inputs.iter()) {
    //         let param_node = pattern
    //             .subst_to_node_keys
    //             .get(&param_marker)
    //             .expect("Internal error: parameter node not found in pattern");
    //         expected_input_mapping.insert(*param_node, input_node);
    //     }
    //
    //     let mut nm = |pat_node: &_, data_node: &_| {
    //         if let Some(expected_data) = expected_input_mapping.get(pat_node) {
    //             return *expected_data == *data_node;
    //         }
    //
    //         let pat_attr = &pattern
    //             .pattern_graph
    //             .get_node_attr(*pat_node)
    //             .unwrap()
    //             .value;
    //         let data_attr = self.get_node_attr(*data_node).unwrap();
    //         NAP::matches(&data_attr, &pat_attr)
    //     };
    //
    //     let mut em = |pat_edge: &EdgeAttribute<EAP::Pattern>,
    //                   data_edge: &EdgeAttribute<EdgeAttr>| {
    //         EAP::matches(&data_edge.edge_attr, &pat_edge.edge_attr)
    //     };
    //
    //     let self_ref = &self.graph;
    //     let pattern_ref = &pattern.pattern_graph.graph;
    //
    //     let isos = general_subgraph_monomorphisms_iter(&pattern_ref, &self_ref, &mut nm, &mut em)?;
    //
    //     let mapping_from_vec = |index_mapping: &[usize]| {
    //         let mut mapping = HashMap::new();
    //         for (src, target) in index_mapping.iter().copied().enumerate() {
    //             let src_node = pattern.pattern_graph.graph.from_index(src);
    //             let target_node = self.graph.from_index(target);
    //             mapping.insert(src_node, target_node);
    //         }
    //         mapping
    //     };
    //
    //     for iso in isos {
    //         // TODO: handle edge orderedness
    //         let mapped = mapping_from_vec(iso.as_ref());
    //         return Some(mapped);
    //     }
    //
    //     None
    // }
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
    fn set_node_attr(&mut self, node_key: NodeKey, node_attr: Self::NodeAttr) -> Option<Self::NodeAttr>;
    fn get_edge_attr(&self, edge_key: EdgeKey) -> Option<&Self::EdgeAttr>;
    fn get_mut_edge_attr(&mut self, edge_key: EdgeKey) -> Option<&mut Self::EdgeAttr>;
    /// Sets the edge attribute for the given edge key that already exists in the graph.
    fn set_edge_attr(
        &mut self,
        edge_key: EdgeKey,
        edge_attr: Self::EdgeAttr,
    ) -> Option<Self::EdgeAttr>;
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

    fn set_node_attr(&mut self, node_key: NodeKey, node_attr: Self::NodeAttr) -> Option<Self::NodeAttr> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::algo::subgraph_isomorphisms_iter;
    use std::collections::HashMap;

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
