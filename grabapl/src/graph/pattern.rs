use crate::graph::operation::{OperationError, OperationResult};
use crate::graph::semantics::{AbstractGraph, SemanticsClone};
use crate::{Graph, NodeKey, Semantics, SubstMarker, WithSubstMarker};
use derive_more::From;
use std::collections::HashMap;
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

// TODO: maybe this is not needed and ParameterSubstitution is already enough?
#[derive(Debug)]
pub struct OperationArgument {
    pub selected_input_nodes: Vec<NodeKey>,
    /// We know this substitution statically already, since we define our parameter substitutions statically.
    /// So we can store it in this struct.
    pub subst: ParameterSubstitution,
}

impl OperationArgument {
    pub fn infer_explicit_for_param(
        selected_nodes: Vec<NodeKey>,
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
            selected_input_nodes: selected_nodes,
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
