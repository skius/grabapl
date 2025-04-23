use std::collections::HashMap;
use petgraph::Direction;
use petgraph::prelude::{DiGraphMap, GraphMap, StableDiGraph};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use petgraph::visit::{IntoEdgesDirected, IntoNeighborsDirected, NodeRef};

pub struct NodeAttribute<NodeAttr> {
    pub node_attr: NodeAttr,
    // Additional attributes can be added here
}

impl<NodeAttr> NodeAttribute<NodeAttr> {
    pub fn new(node_attr: NodeAttr) -> Self {
        NodeAttribute { node_attr }
    }
}

pub struct EdgeAttribute<EdgeAttr> {
    pub edge_attr: EdgeAttr,
    // Additional attributes can be added here
}

impl<EdgeAttr> EdgeAttribute<EdgeAttr> {
    pub fn new(edge_attr: EdgeAttr) -> Self {
        EdgeAttribute { edge_attr }
    }
}

type NodeKey = u32;
type EdgeKey = (NodeKey, NodeKey);

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
        self.node_attr_map.insert(node_key, NodeAttribute::new(node_attr));
        self.max_node_key += 1;
        node_key
    }

    /// Returns the old `EdgeAttr` if it exists, otherwise returns `None`.
    pub fn add_edge(&mut self, source: NodeKey, target: NodeKey, edge_attr: EdgeAttr) -> Option<EdgeAttr> {
        let old_attr = self.graph.add_edge(source, target, EdgeAttribute::new(edge_attr));
        old_attr.map(|attr| attr.edge_attr)
    }
    

    pub fn next_outgoing_edge(&self, source: NodeKey, (_, curr_target): EdgeKey) -> EdgeKey {
        let mut take_next = false;
        for target in self.graph.neighbors_directed(source, Direction::Outgoing).cycle() {
            if take_next {
                return (source, target);
            }
            if target == curr_target {
                take_next = true;
            }
        }
        unreachable!("No edges found")
    }

    pub fn prev_outgoing_edge(&self, source: NodeKey, (_, curr_target): EdgeKey) -> EdgeKey {
        let mut take_idx = None;
        let neighbor_count = self.graph.neighbors_directed(source, Direction::Outgoing).count();
        for (idx, target) in self.graph.neighbors_directed(source, Direction::Outgoing).enumerate().cycle() {
            if take_idx == Some(idx) {
                return (source, target);
            }
            if target == curr_target {
                take_idx = Some((idx - 1 + neighbor_count) % neighbor_count);
            }
        }
        unreachable!("No edges found")
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
        self.graph.remove_edge(src, target)
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
        self.node_attr_map.get(&node_key).map(|attr| &attr.node_attr)
    }

    pub fn get_mut_node_attr(&mut self, node_key: NodeKey) -> Option<&mut NodeAttr> {
        self.node_attr_map.get_mut(&node_key).map(|attr| &mut attr.node_attr)
    }

}