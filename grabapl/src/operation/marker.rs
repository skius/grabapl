use std::collections::{HashMap, HashSet};
use thiserror::Error;
use crate::{interned_string_newtype, NodeKey};
use crate::util::InternString;

#[derive(derive_more::Debug, Clone, PartialEq, Eq, Hash, Copy)]
#[debug("{_0}")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Marker(pub InternString);
interned_string_newtype!(Marker);

#[derive(Error, Debug)]
pub enum MarkerError {
    #[error("Marker `{0:?}` already exists")]
    MarkerAlreadyExists(Marker),
    #[error("Marker `{0:?}` does not exist")]
    MarkerDoesNotExist(Marker),
}

#[derive(Debug, Clone, Default)]
pub struct MarkerSet {
    // which markers currently exist
    pub markers: HashSet<Marker>,
    // which nodes are marked with a specific marker?
    pub marker_to_marked_nodes: HashMap<Marker, HashSet<NodeKey>>,
    // which markers does a specific node have?
    pub marked_nodes_to_markers: HashMap<NodeKey, HashSet<Marker>>,
}

impl MarkerSet {
    pub fn new() -> Self {
        MarkerSet::default()
    }

    pub fn all_marked_nodes(&self) -> impl Iterator<Item = NodeKey> {
        self.marked_nodes_to_markers.keys().cloned()
    }

    pub fn add_marker(&mut self, marker: impl Into<Marker>) -> Result<(), MarkerError> {
        let marker = marker.into();
        if self.markers.contains(&marker) {
            return Err(MarkerError::MarkerAlreadyExists(marker));
        }
        self.markers.insert(marker);
        self.marker_to_marked_nodes.insert(marker, HashSet::new());
        Ok(())
    }

    pub fn mark_node(&mut self, marker: impl Into<Marker>, node_key: NodeKey) -> Result<(), MarkerError> {
        let marker = marker.into();
        if !self.markers.contains(&marker) {
            return Err(MarkerError::MarkerDoesNotExist(marker));
        }
        self.marker_to_marked_nodes.get_mut(&marker).unwrap().insert(node_key);
        self.marked_nodes_to_markers.entry(node_key).or_default().insert(marker);
        Ok(())
    }

    pub fn create_marker_and_mark_node(&mut self, marker: impl Into<Marker>, node_key: NodeKey) {
        let marker = marker.into();
        if !self.markers.contains(&marker) {
            self.add_marker(marker).expect("Marker should not already exist");
        }
        self.mark_node(marker, node_key).expect("Node should be able to be marked");
    }

    pub fn remove_marker(&mut self, marker: impl Into<Marker>) {
        let marker = marker.into();
        if let Some(nodes) = self.marker_to_marked_nodes.remove(&marker) {
            for node in nodes {
                if let Some(markers) = self.marked_nodes_to_markers.get_mut(&node) {
                    markers.remove(&marker);
                    if markers.is_empty() {
                        self.marked_nodes_to_markers.remove(&node);
                    }
                }
            }
        }
        self.markers.remove(&marker);
    }
}