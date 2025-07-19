//! Markers are tags that can be applied to nodes in the concrete graph. They do not exist abstractly.
//! Markers are primarily used to hide nodes from shape queries.

use crate::util::{InternString, log};
use crate::{NodeKey, interned_string_newtype};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Defines a set of markers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SkipMarkers {
    All,
    Set(HashSet<Marker>),
}

impl Default for SkipMarkers {
    fn default() -> Self {
        SkipMarkers::none()
    }
}

impl SkipMarkers {
    pub fn new(markers: impl IntoIterator<Item = impl Into<Marker>>) -> Self {
        SkipMarkers::Set(markers.into_iter().map(Into::into).collect())
    }

    pub fn none() -> Self {
        SkipMarkers::Set(HashSet::new())
    }

    pub fn all() -> Self {
        SkipMarkers::All
    }

    pub fn skip_all(&mut self) {
        *self = SkipMarkers::All;
    }

    pub fn skip(&mut self, marker: Marker) {
        match self {
            SkipMarkers::All => {}
            SkipMarkers::Set(set) => {
                set.insert(marker);
            }
        }
    }
}

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

// TODO: this is not only a marker set, but also marked nodes themselves.
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

    /// Returns an iterator over all nodes that have a marker from the SkipMarkers set.
    pub fn skipped_nodes<'a>(
        &'a self,
        skip_markers: &'a SkipMarkers,
    ) -> impl Iterator<Item = NodeKey> + 'a {
        let filter: Box<dyn for<'b> Fn(&'b NodeKey) -> bool> = match skip_markers {
            SkipMarkers::All => Box::new(|_node: &NodeKey| true),
            SkipMarkers::Set(markers) => Box::new(|node: &NodeKey| {
                if let Some(node_markers) = self.marked_nodes_to_markers.get(node) {
                    !node_markers.is_disjoint(markers)
                } else {
                    log::warn!("Node {:?} has no marker mapping, should not happen", node);
                    true
                }
            }),
        };
        self.marked_nodes_to_markers.keys().copied().filter(filter)
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

    pub fn mark_node(
        &mut self,
        marker: impl Into<Marker>,
        node_key: NodeKey,
    ) -> Result<(), MarkerError> {
        let marker = marker.into();
        if !self.markers.contains(&marker) {
            return Err(MarkerError::MarkerDoesNotExist(marker));
        }
        self.marker_to_marked_nodes
            .get_mut(&marker)
            .unwrap()
            .insert(node_key);
        self.marked_nodes_to_markers
            .entry(node_key)
            .or_default()
            .insert(marker);
        Ok(())
    }

    pub fn create_marker_and_mark_node(&mut self, marker: impl Into<Marker>, node_key: NodeKey) {
        let marker = marker.into();
        if !self.markers.contains(&marker) {
            self.add_marker(marker)
                .expect("Marker should not already exist");
        }
        self.mark_node(marker, node_key)
            .expect("Node should be able to be marked");
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
