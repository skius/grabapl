use crate::graph::semantics::AbstractGraph;
use crate::{Graph, NodeKey, Semantics, SubstMarker, WithSubstMarker};
use derive_more::From;
use std::collections::HashMap;

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
#[derive(Debug)]
pub struct ParameterSubstitution {
    pub mapping: HashMap<SubstMarker, NodeKey>,
}

impl ParameterSubstitution {
    pub fn new(mapping: HashMap<SubstMarker, NodeKey>) -> Self {
        ParameterSubstitution { mapping }
    }
}

pub struct OperationArgument {
    pub selected_input_nodes: Vec<NodeKey>,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, From)]
pub struct AbstractOutputNodeMarker(pub &'static str);

pub struct OperationOutput {
    // TODO: use OutputMarker instead of SubstMarker?
    pub new_nodes: HashMap<AbstractOutputNodeMarker, NodeKey>,
}
