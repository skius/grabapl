use std::collections::HashMap;
use crate::{Graph, NodeKey, Semantics, SubstMarker, WithSubstMarker};

pub struct OperationParameter<S: Semantics> {
    /// The explicitly selected, ordered input nodes.
    pub explicit_input_nodes: Vec<SubstMarker>,
    /// The initial abstract state that the operation expects.
    // TODO: do we need WithSubstMarker? cant we just use the hashmap?
    pub parameter_graph: Graph<WithSubstMarker<S::NodeAbstract>, S::EdgeAbstract>,
    /// Maps the user-defined substitution markers to the node keys in the pattern graph.
    pub subst_to_node_keys: HashMap<SubstMarker, NodeKey>,
}

/// The result of trying to bind an abstract graph to a parameter graph.
pub struct ParameterSubstition {
    pub mapping: HashMap<SubstMarker, NodeKey>,
}

impl ParameterSubstition {
    pub fn new(mapping: HashMap<SubstMarker, NodeKey>) -> Self {
        ParameterSubstition { mapping }
    }
}

pub struct OperationArgument {
    pub selected_input_nodes: Vec<NodeKey>,
}

