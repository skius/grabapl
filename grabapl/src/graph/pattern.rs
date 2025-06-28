use std::borrow::Cow;
use crate::graph::operation::{OperationError, OperationResult};
use crate::graph::semantics::{AbstractGraph, SemanticsClone};
use crate::{Graph, NodeKey, Semantics, SubstMarker, WithSubstMarker};
use derive_more::From;
use std::collections::HashMap;
use crate::graph::GraphTrait;
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
}

pub struct GraphWithSubstitution<'a, G: GraphTrait> {
    pub graph: &'a mut G,
    /// Maps operation-defined SubstMarkers to the dynamic graph node keys.
    pub subst: &'a ParameterSubstitution,
    /// Maps newly created nodes to their SubstMarker.
    new_nodes_subst: HashMap<SubstMarker, NodeKey>,
    /// Keeps track of changes done to the graph.
    new_nodes: Vec<NodeKey>,
    new_edges: Vec<(NodeKey, NodeKey)>,
    removed_nodes: Vec<NodeKey>,
    removed_edges: Vec<(NodeKey, NodeKey)>,
    changed_node_av: HashMap<NodeKey, G::NodeAttr>,
    changed_edge_av: HashMap<(NodeKey, NodeKey), G::EdgeAttr>,
}

impl<'a, G: GraphTrait<NodeAttr: Clone, EdgeAttr: Clone>> GraphWithSubstitution<'a, G> {
    pub fn new(
        graph: &'a mut G,
        subst: &'a ParameterSubstitution,
    ) -> Self {
        GraphWithSubstitution {
            graph,
            subst,
            new_nodes_subst: HashMap::new(),
            new_nodes: Vec::new(),
            new_edges: Vec::new(),
            removed_nodes: Vec::new(),
            removed_edges: Vec::new(),
            changed_node_av: HashMap::new(),
            changed_edge_av: HashMap::new(),
        }
    }

    fn get_node_key(
        &self,
        marker: SubstMarker,
    ) -> Option<NodeKey> {
        let key = self.subst.mapping.get(&marker).copied().or_else(|| {
            // If the marker is not in the substitution, we can try to find it in the new nodes.
            self.new_nodes_subst.get(&marker).copied()
        });
        if let Some(key) = key {
            if self.removed_nodes.contains(&key) {
                return None;
            }
        }
        key
    }

    // TODO: disgusting. switch from SubstMarker to an explicit NewMarker or something in a separate namespace.
    //  these substs have nothing to do with ParameterSubstitution, they are just used to refer to give newly added nodes
    //  a name other than their NodeKey.
    pub fn new_subst_marker(&mut self) -> SubstMarker {
        // Generate a new SubstMarker that is not already in the substitution mapping.
        let mut max_marker = 0;
        for marker in self.subst.mapping.keys() {
            if *marker > max_marker {
                max_marker = *marker;
            }
        }
        max_marker + 1
    }

    pub fn add_node(
        &mut self,
        marker: SubstMarker,
        value: G::NodeAttr,
    ) {
        // TODO: make this error
        if self.get_node_value(marker).is_some() {
            panic!("Marker {} already exists in the substitution mapping", marker);
        }
        let node_key = self.graph.add_node(value);
        self.new_nodes.push(node_key);
        self.new_nodes_subst.insert(marker, node_key);
    }
    pub fn delete_node(
        &mut self,
        marker: SubstMarker,
    ) -> Option<G::NodeAttr> {
        let Some(node_key) = self.get_node_key(marker) else {
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
        src_marker: SubstMarker,
        dst_marker: SubstMarker,
        value: G::EdgeAttr,
    ) -> Option<G::EdgeAttr> {
        let src_key = self.get_node_key(src_marker)?;
        let dst_key = self.get_node_key(dst_marker)?;
        self.new_edges.push((src_key, dst_key));
        self.graph.add_edge(src_key, dst_key, value)
    }

    pub fn delete_edge(
        &mut self,
        src_marker: SubstMarker,
        dst_marker: SubstMarker,
    ) -> Option<G::EdgeAttr> {
        let src_key = self.get_node_key(src_marker)?;
        let dst_key = self.get_node_key(dst_marker)?;
        let removed_value = self.graph.delete_edge(src_key, dst_key);
        if removed_value.is_some() {
            self.removed_edges.push((src_key, dst_key));
        }
        removed_value
    }

    pub fn get_node_value(
        &self,
        marker: SubstMarker,
    ) -> Option<&G::NodeAttr> {
        self.get_node_key(marker).and_then(|node_key| {
            self.graph.get_node_attr(node_key)
        })
    }

    pub fn set_node_value(
        &mut self,
        marker: SubstMarker,
        value: G::NodeAttr,
    ) -> Option<G::NodeAttr> {
        let node_key = self.get_node_key(marker)?;
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
        src_marker: SubstMarker,
        dst_marker: SubstMarker,
    ) -> Option<&G::EdgeAttr> {
        let src_key = self.get_node_key(src_marker)?;
        let dst_key = self.get_node_key(dst_marker)?;
        self.graph.get_edge_attr((src_key, dst_key))
    }

    pub fn set_edge_value(
        &mut self,
        src_marker: SubstMarker,
        dst_marker: SubstMarker,
        value: G::EdgeAttr,
    ) -> Option<G::EdgeAttr> {
        let src_key = self.get_node_key(src_marker)?;
        let dst_key = self.get_node_key(dst_marker)?;
        self.changed_edge_av.insert((src_key, dst_key), value.clone());
        let old_value = self.graph.set_edge_attr((src_key, dst_key), value.clone());
        if old_value.is_some() {
            // we only changed it if it exists, by semantics of set_edge_attr
            self.changed_edge_av.insert((src_key, dst_key), value);
        }
        old_value
    }

    fn get_new_nodes_and_edges_from_desired_names(
        &self,
        desired_node_output_names: &HashMap<SubstMarker, AbstractOutputNodeMarker>
    ) -> (HashMap<AbstractOutputNodeMarker, NodeKey>, Vec<(NodeKey, NodeKey)>) {
        let mut new_nodes = HashMap::new();
        for (marker, node_key) in &self.new_nodes_subst {
            let Some(output_marker) = desired_node_output_names.get(marker) else {
                continue;
            };
            new_nodes.insert(*output_marker, *node_key);
        }
        let mut new_edges = Vec::new();
        // only include edges that belong to nodes that are in new_nodes and/or the existing graph
        for (src_key, dst_key) in &self.new_edges {
            if new_nodes.values().any(|&n| n == *src_key) || new_nodes.values().any(|&n| n == *dst_key) {
                new_edges.push((*src_key, *dst_key));
            }
        }
        (new_nodes, new_edges)
    }

    pub fn get_abstract_output<S: Semantics<NodeAbstract = G::NodeAttr, EdgeAbstract = G::EdgeAttr>>(
        &self,
        desired_node_output_names: HashMap<SubstMarker, AbstractOutputNodeMarker>
    ) -> AbstractOperationOutput<S> {
        let (new_nodes, new_edges) = self.get_new_nodes_and_edges_from_desired_names(&desired_node_output_names);

        // Only report changed av's for nodes and edges that are in the new_nodes and new_edges.
        let mut changed_abstract_values_nodes = HashMap::new();
        for (node_key, node_av) in &self.changed_node_av {
            if new_nodes.values().any(|&n| n == *node_key) {
                changed_abstract_values_nodes.insert(*node_key, node_av.clone());
            }
        }
        let mut changed_abstract_edges = HashMap::new();
        for (edge_key, edge_av) in &self.changed_edge_av {
            if new_edges.iter().any(|(src, dst)| *src == edge_key.0 && *dst == edge_key.1) {
                changed_abstract_edges.insert(*edge_key, edge_av.clone());
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
            changed_abstract_edges,
        }
    }

    pub fn get_concrete_output(
        &self,
        desired_node_output_names: HashMap<SubstMarker, AbstractOutputNodeMarker>
    ) -> OperationOutput {
        let (new_nodes, _new_edges) = self.get_new_nodes_and_edges_from_desired_names(&desired_node_output_names);

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
}

impl<'a> OperationArgument<'a> {
    pub fn infer_explicit_for_param(
        selected_nodes: &'a [NodeKey],
        param: &OperationParameter<impl Semantics>,
    ) -> OperationResult<Self> {
        if param.explicit_input_nodes.len() != selected_nodes.len() {
            return Err(OperationError::InvalidOperationArgumentCount {
                expected: param.explicit_input_nodes.len(),
                actual: selected_nodes.len(),
            });
        }

        let subst = param
            .explicit_input_nodes
            .iter()
            .zip(selected_nodes.iter())
            .map(|(subst_marker, node_key)| (*subst_marker, *node_key))
            .collect();
        Ok(OperationArgument {
            selected_input_nodes: selected_nodes.into(),
            subst: ParameterSubstitution::new(subst),
        })
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, From)]
pub struct AbstractOutputNodeMarker(pub &'static str);

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
    pub changed_abstract_edges: HashMap<(NodeKey, NodeKey), S::EdgeAbstract>,
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
