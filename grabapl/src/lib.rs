use std::collections::HashMap;
use petgraph::Direction;
use petgraph::prelude::{DiGraphMap, StableDiGraph};
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use petgraph::visit::NodeRef;

pub struct NodeAttribute<NodeAttr> {
    pub node_attr: NodeAttr,
    // Additional attributes can be added here
    node_idx: NodeIndex,
}

impl<NodeAttr> NodeAttribute<NodeAttr> {
    pub fn new(node_attr: NodeAttr, node_idx: NodeIndex) -> Self {
        NodeAttribute { node_attr, node_idx }
    }
}

pub struct EdgeAttribute<EdgeAttr> {
    pub edge_attr: EdgeAttr,
    // Additional attributes can be added here
    edge_idx: EdgeIndex,
}

impl<EdgeAttr> EdgeAttribute<EdgeAttr> {
    pub fn new(edge_attr: EdgeAttr, edge_index: EdgeIndex) -> Self {
        EdgeAttribute { edge_attr, edge_idx: edge_index }
    }
}

type NodeKey = u32;
type EdgeKey = u32;

pub struct ConcreteGraph<NodeAttr, EdgeAttr> {
    graph: StableDiGraph<NodeKey, EdgeKey>,
    max_node_key: NodeKey,
    node_attr_map: HashMap<NodeKey, NodeAttribute<NodeAttr>>,
    max_edge_key: EdgeKey,
    edge_attr_map: HashMap<EdgeKey, EdgeAttribute<EdgeAttr>>,
}

impl<NodeAttr, EdgeAttr> ConcreteGraph<NodeAttr, EdgeAttr> {
    pub fn new() -> Self {
        ConcreteGraph {
            graph: StableDiGraph::new(),
            max_node_key: 0,
            node_attr_map: HashMap::new(),
            max_edge_key: 0,
            edge_attr_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_attr: NodeAttr) -> NodeKey {
        let node_key = self.max_node_key;
        let idx = self.graph.add_node(node_key);
        self.node_attr_map.insert(node_key, NodeAttribute::new(node_attr, idx));
        self.max_node_key += 1;
        node_key
    }
    
    pub fn add_edge(&mut self, source: NodeKey, target: NodeKey, edge_attr: EdgeAttr) -> NodeKey {
        let edge_key = self.max_edge_key;
        let source_idx = self.node_attr_map.get(&source).expect("Source node not found").node_idx;
        let target_idx = self.node_attr_map.get(&target).expect("Target node not found").node_idx;
        let edge_idx = self.graph.add_edge(source_idx, target_idx, edge_key);
        self.edge_attr_map.insert(edge_key, EdgeAttribute::new(edge_attr, edge_idx));
        self.max_edge_key += 1;
        edge_key
    }
    
    fn outgoing_edge_keys_sorted(&self, source: NodeKey) -> Vec<EdgeKey> {
        let source_idx = self.node_attr_map.get(&source).expect("Source node not found").node_idx;
        let mut keys = self.graph.edges_directed(source_idx, Direction::Outgoing)
            .map(|edge| *edge.weight())
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }
    
    pub fn next_outgoing_edge(&self, source: NodeKey, curr_edge: EdgeKey) -> EdgeKey {
        let all_edge_keys = self.outgoing_edge_keys_sorted(source);
        let curr_edge_key_idx = all_edge_keys.iter().position(|&x| x == curr_edge).unwrap();
        let next_edge_key_idx = curr_edge_key_idx + 1 % all_edge_keys.len();
        all_edge_keys[next_edge_key_idx]
    }
    
    pub fn prev_outgoing_edge(&self, source: NodeKey, curr_edge: EdgeKey) -> EdgeKey {
        let all_edge_keys = self.outgoing_edge_keys_sorted(source);
        let curr_edge_key_idx = all_edge_keys.iter().position(|&x| x == curr_edge).unwrap();
        let prev_edge_key_idx = (curr_edge_key_idx + all_edge_keys.len() - 1) % all_edge_keys.len();
        all_edge_keys[prev_edge_key_idx]
    }
    
    pub fn remove_node(&mut self, node_key: NodeKey) -> Option<NodeAttr> {
        if let Some(node_attr) = self.node_attr_map.remove(&node_key) {
            for dir in [Direction::Incoming, Direction::Outgoing] {
                for edge in self.graph.edges_directed(node_attr.node_idx, dir) {
                    let key = edge.weight();
                    self.edge_attr_map.remove(key);
                }
                self.graph.remove_node(node_attr.node_idx);
            }
            Some(node_attr.node_attr)
        } else {
            None
        }
    }
    
    pub fn remove_edge_between(&mut self, source: NodeKey, target: NodeKey) -> Option<EdgeAttr> {
        let source_idx = self.node_attr_map.get(&source).expect("Source node not found").node_idx;
        let target_idx = self.node_attr_map.get(&target).expect("Target node not found").node_idx;
        if let Some(edge_idx) = self.graph.find_edge(source_idx, target_idx) {
            let edge_key = self.graph.edge_weight(edge_idx).expect("Edge not found");
            self.remove_edge(*edge_key)
        } else {
            None
        }
    }
    
    pub fn remove_edge(&mut self, edge_key: EdgeKey) -> Option<EdgeAttr> {
        if let Some(edge_attr) = self.edge_attr_map.remove(&edge_key) {
            self.graph.remove_edge(edge_attr.edge_idx);
            Some(edge_attr.edge_attr)
        } else {
            None
        }
    }
    
    pub fn get_edge_attr(&self, edge_key: EdgeKey) -> Option<&EdgeAttr> {
        self.edge_attr_map.get(&edge_key).map(|attr| &attr.edge_attr)
    }
    
    pub fn get_mut_edge_attr(&mut self, edge_key: EdgeKey) -> Option<&mut EdgeAttr> {
        self.edge_attr_map.get_mut(&edge_key).map(|attr| &mut attr.edge_attr)
    }
    
    pub fn get_node_attr(&self, node_key: NodeKey) -> Option<&NodeAttr> {
        self.node_attr_map.get(&node_key).map(|attr| &attr.node_attr)
    }
    
    pub fn get_mut_node_attr(&mut self, node_key: NodeKey) -> Option<&mut NodeAttr> {
        self.node_attr_map.get_mut(&node_key).map(|attr| &mut attr.node_attr)
    }
    
}