use crate::graph::operation::{OperationError, OperationResult};
use crate::graph::semantics::AbstractGraph;
use crate::{Graph, NodeKey, Semantics, SubstMarker, WithSubstMarker};
use derive_more::From;
use std::collections::HashMap;
// TODO: rename/move these structs and file. 'pattern.rs' is an outdated term.

pub struct OperationParameter<S: Semantics> {
    /// The ordered input nodes that must be explicitly selected.
    pub explicit_input_nodes: Vec<SubstMarker>,
    /// The initial abstract state that the operation expects.
    // TODO: do we need WithSubstMarker? cant we just use the hashmap?
    pub parameter_graph: AbstractGraph<S>,
    // TODO: Use a BidiHashMap
    // TODO: Actually, because an operation may accept the same node multiple times, we may want to to have the inverse actually be a multimap? so NodeKey -> Vec<SubstMarker>
    /// Maps the user-defined substitution markers to the node keys in the pattern graph.
    pub subst_to_node_keys: HashMap<SubstMarker, NodeKey>,
    /// Maps node keys in the pattern graph to the user-defined substitution markers.
    pub node_keys_to_subst: HashMap<NodeKey, SubstMarker>,
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

pub struct OperationOutput {
    // TODO: use OutputMarker instead of SubstMarker?
    pub new_nodes: HashMap<AbstractOutputNodeMarker, NodeKey>,
}
