use crate::graph::GraphTrait;
use crate::operation::marker::MarkerSet;
use crate::operation::trace::Trace;
use crate::operation::{OperationError, OperationResult};
use crate::semantics::AbstractGraph;
use crate::util::bimap::BiMap;
use crate::util::{InternString, log};
use crate::{NodeKey, Semantics, SubstMarker, interned_string_newtype};
use derive_more::From;
use error_stack::bail;
use petgraph::visit::UndirectedAdaptor;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
// TODO: rename/move these structs and file. 'pattern.rs' is an outdated term.
// renamed.

#[derive(Debug, Error)]
pub enum OperationParameterError {
    #[error(
        "Context node {0:?} is not connected to any explicit input nodes in the parameter graph"
    )]
    ContextNodeNotConnected(SubstMarker),
}

#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound = "S: crate::serde::SemanticsSerde")
)]
pub struct OperationParameter<S: Semantics> {
    /// The ordered input nodes that must be explicitly selected.
    pub explicit_input_nodes: Vec<SubstMarker>,
    /// The initial abstract state that the operation expects.
    pub parameter_graph: AbstractGraph<S>,
    // TODO: Actually, because an operation may accept the same node multiple times, we may want to to have the inverse actually be a multimap? so NodeKey -> Vec<SubstMarker>
    //  (^ is if we support node aliasing.)
    /// Associates node keys of the parameter graph with the substitution markers.
    pub node_keys_to_subst: BiMap<NodeKey, SubstMarker>,
}

impl<S: Semantics> PartialEq for OperationParameter<S> {
    fn eq(&self, other: &Self) -> bool {
        // TODO: we could lift the requirement of "with_same_keys" if we remapped based on SubstMarker.
        self.explicit_input_nodes == other.explicit_input_nodes
            && self
                .parameter_graph
                .semantically_matches_with_same_keys(&other.parameter_graph)
            && self.node_keys_to_subst == other.node_keys_to_subst
    }
}

impl<S: Semantics> Clone for OperationParameter<S> {
    fn clone(&self) -> Self {
        OperationParameter {
            explicit_input_nodes: self.explicit_input_nodes.clone(),
            parameter_graph: self.parameter_graph.clone(),
            node_keys_to_subst: self.node_keys_to_subst.clone(),
        }
    }
}

impl<S: Semantics> OperationParameter<S> {
    pub fn new_empty() -> Self {
        OperationParameter {
            explicit_input_nodes: Vec::new(),
            parameter_graph: AbstractGraph::<S>::new(),
            node_keys_to_subst: BiMap::new(),
        }
    }

    pub fn check_validity(&self) -> Result<(), OperationParameterError> {
        // we want weak connected components, hence we use UndirectedAdaptor
        let undi = UndirectedAdaptor(&self.parameter_graph.graph);
        let components = petgraph::algo::tarjan_scc(&undi);

        for component in components {
            let mut contains_explicit_input = false;
            for key in &component {
                let subst_marker = self
                    .node_keys_to_subst
                    .get_left(key)
                    .expect("internal error: should find subst marker for node key");
                if self.explicit_input_nodes.contains(subst_marker) {
                    contains_explicit_input = true;
                    break;
                }
            }
            if !contains_explicit_input {
                let example_context_node = component[0];
                let subst_marker = self
                    .node_keys_to_subst
                    .get_left(&example_context_node)
                    .expect("internal error: should find subst marker for node key");
                return Err(OperationParameterError::ContextNodeNotConnected(
                    *subst_marker,
                ));
            }
        }

        Ok(())
    }
}

/// The result of trying to bind an abstract graph to a parameter graph.
// Note: this is a mapping from OperationParameter substmarkers to the dynamic/argument graph node keys.
#[derive(Debug)]
pub struct ParameterSubstitution {
    pub mapping: HashMap<SubstMarker, NodeKey>,
}

impl ParameterSubstitution {
    pub fn new(mapping: HashMap<SubstMarker, NodeKey>) -> Self {
        ParameterSubstitution { mapping }
    }

    pub fn infer_explicit_for_param(
        selected_nodes: &[NodeKey],
        param: &OperationParameter<impl Semantics>,
    ) -> OperationResult<Self> {
        if param.explicit_input_nodes.len() != selected_nodes.len() {
            bail!(OperationError::InvalidOperationArgumentCount {
                expected: param.explicit_input_nodes.len(),
                actual: selected_nodes.len(),
            });
        }

        let mapping = param
            .explicit_input_nodes
            .iter()
            .zip(selected_nodes.iter())
            .map(|(subst_marker, node_key)| (subst_marker.clone(), *node_key))
            .collect();
        Ok(ParameterSubstitution { mapping })
    }
}

#[derive(Debug, Clone, Copy, From, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NewNodeMarker {
    Named(InternString),
    // TODO: hide this
    #[from(ignore)]
    Implicit(u32),
}
interned_string_newtype!(NewNodeMarker, NewNodeMarker::Named);

/// Used inside GraphWithSubstitution to refer to nodes.
#[derive(Debug, Clone, Copy, From, Hash, Eq, PartialEq)]
pub enum NodeMarker {
    Subst(SubstMarker),
    New(NewNodeMarker),
}

pub struct GraphWithSubstitution<'a, G: GraphTrait> {
    pub graph: &'a mut G,
    /// Maps operation-defined SubstMarkers to the dynamic graph node keys.
    pub subst: &'a ParameterSubstitution,
    /// Maps newly created nodes to their SubstMarker.
    new_nodes_map: HashMap<NewNodeMarker, NodeKey>,
    max_new_node_marker: u32,
    /// Keeps track of changes done to the graph.
    new_nodes: Vec<NodeKey>,
    new_edges: Vec<(NodeKey, NodeKey)>,
    removed_nodes: Vec<NodeKey>,
    removed_edges: Vec<(NodeKey, NodeKey)>,
    changed_node_av: HashMap<NodeKey, G::NodeAttr>,
    changed_edge_av: HashMap<(NodeKey, NodeKey), G::EdgeAttr>,
}

impl<'a, G: GraphTrait<NodeAttr: Clone, EdgeAttr: Clone>> GraphWithSubstitution<'a, G> {
    pub fn new(graph: &'a mut G, subst: &'a ParameterSubstitution) -> Self {
        GraphWithSubstitution {
            graph,
            subst,
            new_nodes_map: HashMap::new(),
            max_new_node_marker: 0,
            new_nodes: Vec::new(),
            new_edges: Vec::new(),
            removed_nodes: Vec::new(),
            removed_edges: Vec::new(),
            changed_node_av: HashMap::new(),
            changed_edge_av: HashMap::new(),
        }
    }

    pub fn get_node_key(&self, marker: &NodeMarker) -> Option<NodeKey> {
        let found_key = match marker {
            NodeMarker::Subst(sm) => {
                // If the marker is a SubstMarker, we look it up in the substitution mapping.
                self.subst.mapping.get(&sm).copied()
            }
            NodeMarker::New(nnm) => self.new_nodes_map.get(&nnm).copied(),
        };
        if let Some(key) = found_key {
            if self.removed_nodes.contains(&key) {
                // don't return a key that was removed
                return None;
            }
        }
        found_key
    }

    pub fn new_node_marker(&mut self) -> NewNodeMarker {
        let marker = NewNodeMarker::Implicit(self.max_new_node_marker);
        self.max_new_node_marker += 1;
        marker
    }

    pub fn add_node(&mut self, marker: impl Into<NewNodeMarker>, value: G::NodeAttr) {
        let marker = marker.into();
        // TODO: make this error
        if self.get_node_key(&NodeMarker::New(marker)).is_some() {
            // TODO: should we disallow re-adding a node that was deleted? if so,
            //  the above should be changed since it skips previously removed nodes
            panic!(
                "Marker {:?} already exists in the substitution mapping",
                marker
            );
        }
        let node_key = self.graph.add_node(value);
        self.new_nodes.push(node_key);
        self.new_nodes_map.insert(marker, node_key);
    }
    pub fn delete_node(&mut self, marker: impl Into<NodeMarker>) -> Option<G::NodeAttr> {
        let marker = marker.into();
        let Some(node_key) = self.get_node_key(&marker) else {
            return None; // Node not found in the substitution or new nodes.
        };
        let removed_value = self.graph.delete_node(node_key);
        if removed_value.is_some() {
            self.removed_nodes.push(node_key);
        }
        removed_value
    }

    pub fn add_edge(
        &mut self,
        src_marker: impl Into<NodeMarker>,
        dst_marker: impl Into<NodeMarker>,
        value: G::EdgeAttr,
    ) -> Option<G::EdgeAttr> {
        let src_marker = src_marker.into();
        let dst_marker = dst_marker.into();
        let src_key = self.get_node_key(&src_marker)?;
        let dst_key = self.get_node_key(&dst_marker)?;
        self.new_edges.push((src_key, dst_key));
        self.graph.add_edge(src_key, dst_key, value)
    }

    pub fn delete_edge(
        &mut self,
        src_marker: impl Into<NodeMarker>,
        dst_marker: impl Into<NodeMarker>,
    ) -> Option<G::EdgeAttr> {
        let src_marker = src_marker.into();
        let dst_marker = dst_marker.into();
        let src_key = self.get_node_key(&src_marker)?;
        let dst_key = self.get_node_key(&dst_marker)?;
        let removed_value = self.graph.delete_edge(src_key, dst_key);
        if removed_value.is_some() {
            self.removed_edges.push((src_key, dst_key));
        }
        removed_value
    }

    pub fn get_node_value(&self, marker: impl Into<NodeMarker>) -> Option<&G::NodeAttr> {
        let marker = marker.into();
        self.get_node_key(&marker)
            .and_then(|node_key| self.graph.get_node_attr(node_key))
    }

    // TODO: in general these functions are a bit awkward (set_edge_value too), since they return
    //  an optional. If the optional is None, it means the node/edge does not exist, and no value was set.
    //  Can we communicate this better?
    pub fn set_node_value(
        &mut self,
        marker: impl Into<NodeMarker>,
        value: G::NodeAttr,
    ) -> Option<G::NodeAttr> {
        let marker = marker.into();
        let node_key = self.get_node_key(&marker)?;
        self.changed_node_av.insert(node_key, value.clone());
        let old_value = self.graph.set_node_attr(node_key, value.clone());
        if old_value.is_some() {
            // we only changed it if it exists, by semantics of set_node_attr
            self.changed_node_av.insert(node_key, value);
        }
        old_value
    }

    /// Use this method to apply another operation's maybe-changes to the graph.
    /// This will update the current view of the node's AV to be sound, i.e., the join of the current AV and the maybe-written AV,
    /// but will remember that it was only the maybe-written AV that was _actually_ maybe written.
    pub fn maybe_set_node_value(
        &mut self,
        marker: impl Into<NodeMarker>,
        maybe_written_av: G::NodeAttr,
        join: impl Fn(&G::NodeAttr, &G::NodeAttr) -> Option<G::NodeAttr>,
    ) -> Option<G::NodeAttr> {
        let marker = marker.into();
        let node_key = self.get_node_key(&marker)?;
        if let Some(old_av) = self.graph.get_node_attr(node_key) {
            // only remember that we maybe wrote "maybe_written_av".
            self.changed_node_av
                .insert(node_key, maybe_written_av.clone());
            // Merge the current AV with the new value.
            let merged_av = join(old_av, &maybe_written_av)
                // TODO: expect is bad here and for edges.
                //  We need to hide the node (just like when merging IntermediateStates) if the join does not exist.
                //  This unwrap means semantics like the following will panic:
                //  `int` type, `str` type, no join. Function takes `str`, maybe-writes `int` to it.
                //  at the call-site of the outer operation, we must now join (i.e., this function) the `int` and `str` types.
                //  This is something that makes sense, and should be supported.
                .expect("must be able to join. TODO: think about if this requirement makes sense");
            // merged_av is the new value we want to set.
            self.graph.set_node_attr(node_key, merged_av)
        } else {
            None
        }
    }

    pub fn get_edge_value(
        &self,
        src_marker: impl Into<NodeMarker>,
        dst_marker: impl Into<NodeMarker>,
    ) -> Option<&G::EdgeAttr> {
        let src_marker = src_marker.into();
        let dst_marker = dst_marker.into();
        let src_key = self.get_node_key(&src_marker)?;
        let dst_key = self.get_node_key(&dst_marker)?;
        self.graph.get_edge_attr((src_key, dst_key))
    }

    pub fn set_edge_value(
        &mut self,
        src_marker: impl Into<NodeMarker>,
        dst_marker: impl Into<NodeMarker>,
        value: G::EdgeAttr,
    ) -> Option<G::EdgeAttr> {
        let src_marker = src_marker.into();
        let dst_marker = dst_marker.into();
        let src_key = self.get_node_key(&src_marker)?;
        let dst_key = self.get_node_key(&dst_marker)?;
        self.changed_edge_av
            .insert((src_key, dst_key), value.clone());
        let old_value = self.graph.set_edge_attr((src_key, dst_key), value.clone());
        if old_value.is_some() {
            // we only changed it if it exists, by semantics of set_edge_attr
            self.changed_edge_av.insert((src_key, dst_key), value);
        } else {
            log::warn!(
                "Attempted to set edge value for non-existing edge from {:?} to {:?}.",
                src_key,
                dst_key
            );
        }
        old_value
    }

    pub fn maybe_set_edge_value(
        &mut self,
        src_marker: impl Into<NodeMarker>,
        dst_marker: impl Into<NodeMarker>,
        maybe_written_av: G::EdgeAttr,
        join: impl Fn(&G::EdgeAttr, &G::EdgeAttr) -> Option<G::EdgeAttr>,
    ) -> Option<G::EdgeAttr> {
        let src_marker = src_marker.into();
        let dst_marker = dst_marker.into();
        let src_key = self.get_node_key(&src_marker)?;
        let dst_key = self.get_node_key(&dst_marker)?;
        if let Some(old_av) = self.graph.get_edge_attr((src_key, dst_key)) {
            // only remember that we maybe wrote "maybe_writte_av".
            self.changed_edge_av
                .insert((src_key, dst_key), maybe_written_av.clone());
            // Merge the current AV with the new value.
            let merged_av = join(old_av, &maybe_written_av)
                .expect("must be able to join. TODO: think about if this requirement makes sense");
            // merged_av is the new value we want to set.
            self.graph.set_edge_attr((src_key, dst_key), merged_av)
        } else {
            log::warn!(
                "Attempted to set edge value for non-existing edge from {:?} to {:?}.",
                src_key,
                dst_key
            );
            None
        }
    }

    fn get_new_nodes_and_edges_from_desired_names(
        &self,
        desired_node_output_names: &HashMap<NewNodeMarker, AbstractOutputNodeMarker>,
    ) -> (
        HashMap<AbstractOutputNodeMarker, NodeKey>,
        Vec<(NodeKey, NodeKey)>,
    ) {
        let mut new_nodes = HashMap::new();
        for (marker, node_key) in &self.new_nodes_map {
            let Some(output_marker) = desired_node_output_names.get(&marker) else {
                continue;
            };
            new_nodes.insert(*output_marker, *node_key);
        }
        let mut new_edges = Vec::new();
        let new_node_or_existing = |node_key: &NodeKey| {
            new_nodes.values().any(|&n| n == *node_key)
                || self.subst.mapping.values().any(|&n| n == *node_key)
        };
        // only include edges that belong to nodes that are in new_nodes and/or the existing graph
        for (src_key, dst_key) in &self.new_edges {
            if new_node_or_existing(src_key) || new_node_or_existing(dst_key) {
                new_edges.push((*src_key, *dst_key));
            }
        }
        (new_nodes, new_edges)
    }

    pub fn get_abstract_output<
        S: Semantics<NodeAbstract = G::NodeAttr, EdgeAbstract = G::EdgeAttr>,
    >(
        &self,
        desired_node_output_names: HashMap<NewNodeMarker, AbstractOutputNodeMarker>,
    ) -> AbstractOperationOutput<S> {
        // TODO: does this make sense? why would an operation abstractly add a node but then not return it?
        let (new_nodes, new_edges) =
            self.get_new_nodes_and_edges_from_desired_names(&desired_node_output_names);

        // Only report changed av's for nodes and edges that are params
        // TODO: make sure the changed_av_nodes/edges skip deleted nodes/edges.
        let existing_nodes: HashSet<NodeKey> = self.subst.mapping.values().cloned().collect();
        let mut existing_edges = HashSet::new();
        for (src, dst, _) in self.graph.edges() {
            existing_edges.insert((src, dst));
        }
        let mut changed_abstract_values_nodes = HashMap::new();
        for (node_key, node_av) in &self.changed_node_av {
            if existing_nodes.contains(node_key) {
                changed_abstract_values_nodes.insert(*node_key, node_av.clone());
            }
        }
        let mut changed_abstract_edges = HashMap::new();
        for (&(src, dst), edge_av) in &self.changed_edge_av {
            if existing_edges.contains(&(src, dst)) {
                changed_abstract_edges.insert((src, dst), edge_av.clone());
            }
        }

        AbstractOperationOutput {
            new_nodes,
            // TODO: see if we want to also make the new edges optional?
            new_edges,
            // TODO: make this better? in theory, if a user were to add a node and then remove it again, same for edges, these containers
            //  would contain too much information.
            removed_edges: self.removed_edges.clone(),
            removed_nodes: self.removed_nodes.clone(),
            changed_abstract_values_nodes,
            changed_abstract_values_edges: changed_abstract_edges,
        }
    }

    pub fn get_concrete_output(
        &self,
        desired_node_output_names: HashMap<NewNodeMarker, AbstractOutputNodeMarker>,
    ) -> OperationOutput {
        let (new_nodes, _new_edges) =
            self.get_new_nodes_and_edges_from_desired_names(&desired_node_output_names);

        OperationOutput {
            new_nodes,
            // TODO: again, make sure this makes sense
            removed_nodes: self.removed_nodes.clone(),
        }
    }
}

#[derive(derive_more::Debug)]
pub struct OperationArgument<'a, S: Semantics> {
    pub selected_input_nodes: Cow<'a, [NodeKey]>,
    /// We know this substitution statically already, since we define our parameter substitutions statically.
    /// So we can store it in this struct.
    pub subst: ParameterSubstitution,
    /// Nodes for which an operation already has an in-scope handle.
    /// These nodes may not be matched in shape queries, as they could modify values and break the operation's
    /// abstract guarantees, since changes are not visible to outer operations.
    pub hidden_nodes: HashSet<NodeKey>,
    pub marker_set: &'a RefCell<MarkerSet>,
    #[debug(skip)]
    pub trace: &'a RefCell<Trace<S>>,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, From)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AbstractOutputNodeMarker(pub InternString);
interned_string_newtype!(AbstractOutputNodeMarker);

/// Keeps track of node changes that happened during the operation execution.
///
/// This is mainly useful for keeping track of which nodes still exist after the operation,
/// without needing to scan the entire graph for changes.
///
/// It also allows operations to name their output nodes with `AbstractOutputNodeMarker`.
pub struct OperationOutput {
    pub new_nodes: HashMap<AbstractOutputNodeMarker, NodeKey>,
    // TODO: figure out if this is needed in the concrete? This can be used in the UDO runner to remove nodes from the mapping,
    //  but is there actually a downside to keeping nodes in the mapping? Think about edge cases.
    pub removed_nodes: Vec<NodeKey>,
}

impl OperationOutput {
    pub fn no_changes() -> Self {
        OperationOutput {
            new_nodes: HashMap::new(),
            removed_nodes: Vec::new(),
        }
    }
}

/// The result of [`run_from_concrete`](super::super::run_from_concrete).
pub struct ConcreteOperationOutput<S: Semantics> {
    pub output: OperationOutput,
    pub marker_set: MarkerSet,
    pub trace: Trace<S>,
}

impl<S: Semantics> ConcreteOperationOutput<S> {
    pub fn new_nodes(&self) -> &HashMap<AbstractOutputNodeMarker, NodeKey> {
        &self.output.new_nodes
    }

    pub fn key_of_output_marker(
        &self,
        marker: impl Into<AbstractOutputNodeMarker>,
    ) -> Option<NodeKey> {
        let marker = marker.into();
        self.output.new_nodes.get(&marker).copied()
    }
}

pub struct AbstractOperationOutput<S: Semantics> {
    pub new_nodes: HashMap<AbstractOutputNodeMarker, NodeKey>,
    pub removed_nodes: Vec<NodeKey>,
    pub new_edges: Vec<(NodeKey, NodeKey)>,
    pub removed_edges: Vec<(NodeKey, NodeKey)>,
    /// These maps contain any abstract values that are set (not necessarily changed) during the operation execution.
    pub changed_abstract_values_nodes: HashMap<NodeKey, S::NodeAbstract>,
    pub changed_abstract_values_edges: HashMap<(NodeKey, NodeKey), S::EdgeAbstract>,
}
// TODO(severe): since this is basically an AID output, we must make sure that during *concrete* execution,
//  we don't accidentally overwrite the mapping from AID to NodeKey from some existing operation.
//  This is because the OperationOutput of an `abstract_apply` could be empty, so we *dont know* that
//  we actually got an output node in the concrete graph. If that new concrete graph node has a clashing
//  name, we overwrite the potential existing mapping, which would cause a logic bug.
// TODO: add ^ to problems-testcases.md
// TODO: Would we fix this if we said OperationOutput is stored by userdefinedoperation
//  in the abstract, i.e., it has to store the mapping?
//  Then at concrete execution time, the user defined op only updates mappings for nodes that
//  it actually expects from the abstract output.
// UPDATE: This *may* be a problem. However, it's unlikely to be a problem.
// For this to be a problem, an operation needs to _sometimes_ return a node. Simply adding a node is not enough,
// since the UDO runner only updates the mapping for _returned_ nodes (as told by the ...Output struct family).
// The only operations that can _sometimes_ return a node are builtin operations.
// Hence I would argue this is a documentation issue (or slight client-DX issue), in that we need to document the
// BuiltinOperation trait better to indicate that _returned_ nodes must *always* be returned, and also be specified
// as a returned node in the operation's signature.
