use crate::graph::GraphTrait;
use crate::graph::operation::{OperationError, OperationResult};
use crate::graph::semantics::{AbstractGraph, SemanticsClone};
use crate::util::log;
use crate::{Graph, NodeKey, Semantics, SubstMarker, WithSubstMarker, interned_string_newtype};
use derive_more::From;
use internment::Intern;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
// TODO: rename/move these structs and file. 'pattern.rs' is an outdated term.

pub struct OperationParameter<S: Semantics> {
    /// The ordered input nodes that must be explicitly selected.
    pub explicit_input_nodes: Vec<SubstMarker>,
    /// The initial abstract state that the operation expects.
    pub parameter_graph: AbstractGraph<S>,
    // TODO: Use a BidiHashMap
    // TODO: Actually, because an operation may accept the same node multiple times, we may want to to have the inverse actually be a multimap? so NodeKey -> Vec<SubstMarker>
    /// Maps the user-defined substitution markers to the node keys in the pattern graph.
    pub subst_to_node_keys: HashMap<SubstMarker, NodeKey>,
    /// Maps node keys in the pattern graph to the user-defined substitution markers.
    pub node_keys_to_subst: HashMap<NodeKey, SubstMarker>,
}

impl<S: SemanticsClone> Clone for OperationParameter<S> {
    fn clone(&self) -> Self {
        OperationParameter {
            explicit_input_nodes: self.explicit_input_nodes.clone(),
            parameter_graph: self.parameter_graph.clone(),
            subst_to_node_keys: self.subst_to_node_keys.clone(),
            node_keys_to_subst: self.node_keys_to_subst.clone(),
        }
    }
}

impl<S: Semantics> OperationParameter<S> {
    pub fn new_empty() -> Self {
        OperationParameter {
            explicit_input_nodes: Vec::new(),
            parameter_graph: AbstractGraph::<S>::new(),
            subst_to_node_keys: HashMap::new(),
            node_keys_to_subst: HashMap::new(),
        }
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
            return Err(OperationError::InvalidOperationArgumentCount {
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
pub enum NewNodeMarker {
    Named(Intern<String>),
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

    fn get_node_key(&self, marker: &NodeMarker) -> Option<NodeKey> {
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
        // only include edges that belong to nodes that are in new_nodes and/or the existing graph
        for (src_key, dst_key) in &self.new_edges {
            if new_nodes.values().any(|&n| n == *src_key)
                || new_nodes.values().any(|&n| n == *dst_key)
            {
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

// TODO: maybe this is not needed and ParameterSubstitution is already enough?
#[derive(Debug)]
pub struct OperationArgument<'a> {
    pub selected_input_nodes: Cow<'a, [NodeKey]>,
    /// We know this substitution statically already, since we define our parameter substitutions statically.
    /// So we can store it in this struct.
    pub subst: ParameterSubstitution,
    /// Nodes for which an operation already has an in-scope handle.
    /// These nodes may not be matched in shape queries, as they could modify values and break the operation's
    /// abstract guarantees, since changes are not visible to outer operations.
    pub hidden_nodes: HashSet<NodeKey>,
}

// impl<'a> OperationArgument<'a> {
//     pub fn infer_explicit_for_param(
//         selected_nodes: &'a [NodeKey],
//         param: &OperationParameter<impl Semantics>,
//     ) -> OperationResult<Self> {
//         let subst = ParameterSubstitution::infer_explicit_for_param(selected_nodes, param)?;
//         Ok(OperationArgument {
//             selected_input_nodes: selected_nodes.into(),
//             subst,
//         })
//     }
// }

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, From)]
pub struct AbstractOutputNodeMarker(pub Intern<String>);
interned_string_newtype!(AbstractOutputNodeMarker);

// TODO: OperationOutput should also include substractive changes to the graph,
//  i.e.:
//  * nodes that were removed
//  * edges that were removed
//  * abstract values whose attributes were changed
// TODO: this last point seems tricky. How can we know which attrs were changed?
//  I guess: for Builtins, we can just run the apply_abstract and try to do some
//  'merge'. Well, actually, the apply_abstract does the merge for us.
//  For UserDefinedOp, we need to determine the least common ancestor
/// Keeps track of node changes that happened during the operation execution.
///
/// This is mainly useful for keeping track of which nodes still exist after the operation,
/// without needing to scan the entire graph for changes.
///
/// It also allows operations to name their output nodes with `AbstractOutputNodeMarker`.
pub struct OperationOutput {
    pub new_nodes: HashMap<AbstractOutputNodeMarker, NodeKey>,
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

// TODO: this is a "signature" arguably. rename?
pub struct AbstractOperationOutput<S: Semantics> {
    pub new_nodes: HashMap<AbstractOutputNodeMarker, NodeKey>,
    pub removed_nodes: Vec<NodeKey>,
    pub new_edges: Vec<(NodeKey, NodeKey)>,
    pub removed_edges: Vec<(NodeKey, NodeKey)>,
    // TODO: we actually do need to keep track of changed abstract values.
    //  The reason for this is so that we can determine an userdefined operation's abstract changes as well,
    //  without needing to simulate it every time we want to abstractly apply it.
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
