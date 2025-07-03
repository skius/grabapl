use std::collections::{HashMap, HashSet};
use derive_more::From;
use crate::graph::pattern::{AbstractOutputNodeMarker, NewNodeMarker, OperationParameter};
use crate::{Semantics, SubstMarker};
use crate::graph::semantics::{AbstractMatcher, SemanticsClone};

pub type AbstractSignatureEdgeId = (AbstractSignatureNodeId, AbstractSignatureNodeId);
pub type ParameterEdgeId = (SubstMarker, SubstMarker);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
pub enum AbstractSignatureNodeId {
    /// References a pre-existing node in the parameter graph.
    ExistingNode(SubstMarker),
    /// References a new node that will be created by the operation.
    NewNode(AbstractOutputNodeMarker),
}

/// The entirety of an operation's abstract effect.
///
/// This includes requirements on the caller, i.e., parameter shape and types, as well as
/// post-conditions in the form of a to-be-propagated new graph shape and types.
///
/// Post-conditions are necessary due to the mutable pass-by-reference semantics of this library.
///
/// A signature is a "must-occur" description of an operation, i.e., it describes the
/// effects that *will occur for every invocation* of the operation.
///
/// Operations with the same signature should be soundly interchangeable, with only the concrete
/// implementation differing.
pub struct OperationSignature<S: Semantics> {
    /// The operation's name.
    // TODO: decide if this should be unique and used in place of OperationId.
    //  If so, interning it would probably be sensible.
    pub name: String,
    /// The operation's parameter, i.e., the expected input nodes, edges, and their types.
    pub parameter: OperationParameter<S>,
    /// The operation's output, i.e., the deleted nodes and edges, potential new nodes and edges,
    /// and changes to existing nodes and edges.
    pub output: AbstractOutputChanges<S>,
}

impl<S: SemanticsClone> Clone for OperationSignature<S> {
    fn clone(&self) -> Self {
        OperationSignature {
            name: self.name.clone(),
            parameter: self.parameter.clone(),
            output: self.output.clone(),
        }
    }
}

impl<S: Semantics> OperationSignature<S> {
    // mostly an unsound hack to get things running for the time being
    // TODO: fix/remove
    pub fn empty_new(
        name: impl Into<String>,
        parameter: OperationParameter<S>,
    ) -> Self {
        OperationSignature {
            name: name.into(),
            parameter,
            output: AbstractOutputChanges {
                new_nodes: HashMap::new(),
                new_edges: HashMap::new(),
                changed_nodes: HashMap::new(),
                changed_edges: HashMap::new(),
                deleted_nodes: HashSet::new(),
                deleted_edges: HashSet::new(),
            },
        }
    }
}

/// The changes to the graph that an operation will cause.
#[derive(derive_more::Debug)]
pub struct AbstractOutputChanges<S: Semantics> {
    /// New nodes that are guaranteed to be created with a value of the given type.
    pub new_nodes: HashMap<AbstractOutputNodeMarker, S::NodeAbstract>,
    /// New edges that are guaranteed to be created with a value of the given type.
    pub new_edges: HashMap<AbstractSignatureEdgeId, S::EdgeAbstract>,
    /// Pre-existing nodes that may have been modified to be of the given type.
    pub changed_nodes: HashMap<SubstMarker, S::NodeAbstract>,
    /// Pre-existing edges that may have been modified to be of the given type.
    pub changed_edges: HashMap<ParameterEdgeId, S::EdgeAbstract>,
    /// Pre-existing nodes that may have been deleted by the operation.
    pub deleted_nodes: HashSet<SubstMarker>,
    /// Pre-existing edges that may have been deleted by the operation.
    pub deleted_edges: HashSet<ParameterEdgeId>,
}

impl<S: SemanticsClone> Clone for AbstractOutputChanges<S> {
    fn clone(&self) -> Self {
        AbstractOutputChanges {
            new_nodes: self.new_nodes.clone(),
            new_edges: self.new_edges.clone(),
            changed_nodes: self.changed_nodes.clone(),
            changed_edges: self.changed_edges.clone(),
            deleted_nodes: self.deleted_nodes.clone(),
            deleted_edges: self.deleted_edges.clone(),
        }
    }
}

impl<S: Semantics> AbstractOutputChanges<S> {
    /// Returns `true` if `self` can be used wherever `other` is expected, i.e., `self <: other`.
    pub fn is_subtype_of(&self, other: &Self) -> bool {
        // All new nodes and edges from other must be present in self, with a subtype of their
        // counterpart in other.
        for (marker, other_type) in &other.new_nodes {
            if let Some(self_type) = self.new_nodes.get(marker) {
                if !S::NodeMatcher::matches(self_type, other_type) {
                    // New node is not a subtype.
                    // Any caller working with the assumption of `other` would assume an incorrect type.
                    return false;
                }
            } else {
                // Missing new node
                // Any caller working with the assumption of `other` would incorrectly assume that this node exists and crash.
                return false;
            }
        }
        for (edge_id, other_type) in &other.new_edges {
            if let Some(self_type) = self.new_edges.get(edge_id) {
                if !S::EdgeMatcher::matches(self_type, other_type) {
                    // New edge is not a subtype.
                    // Any caller working with the assumption of `other` would assume an incorrect type.
                    return false;
                }
            } else {
                // Missing new edge.
                // Any caller working with the assumption of `other` would incorrectly assume that this edge exists and crash.
                return false;
            }
        }

        // All changed nodes and edges from `self` must be present in `other`, with a supertype of their
        // counterpart in `self`.
        for (marker, self_type) in &self.changed_nodes {
            if let Some(other_type) = other.changed_nodes.get(marker) {
                if !S::NodeMatcher::matches(self_type, other_type) {
                    // Changed node is not a subtype of the one in `other`.
                    // Any caller working with the assumption of `other` would assume an incorrect type.
                    return false;
                }
            } else {
                // Missing changed node.
                // `self` would change values unbeknownst to any caller working with the assumption of `other`.
                return false;
            }
        }
        for (edge_id, self_type) in &self.changed_edges {
            if let Some(other_type) = other.changed_edges.get(edge_id) {
                if !S::EdgeMatcher::matches(self_type, other_type) {
                    // Changed edge is not a subtype of the one in `other`.
                    // Any caller working with the assumption of `other` would assume an incorrect type.
                    return false;
                }
            } else {
                // Missing changed edge.
                // `self` would change values unbeknownst to any caller working with the assumption of `other`.
                return false;
            }
        }

        // All deleted nodes and edges from `self` must be present in `other`.
        for marker in &self.deleted_nodes {
            if !other.deleted_nodes.contains(marker) {
                // `self` would delete a node that `other` expects to be present.
                // Any caller working with the assumption of `other` would assume this node exists and crash.
                return false;
            }
        }
        for edge_id in &self.deleted_edges {
            if !other.deleted_edges.contains(edge_id) {
                // `self` would delete an edge that `other` expects to be present.
                // Any caller working with the assumption of `other` would assume this edge exists and crash.
                return false;
            }
        }

        true
    }
}

impl<S: Semantics> OperationParameter<S> {
    /// Returns `true` if `self` can be used *as a parameter* wherever `other` is expected, i.e., `self <: other`.
    /// Note that this is a parameter, so it is contravariant when looked at as part of a function type.
    pub fn is_subtype_of(
        &self,
        other: &OperationParameter<S>,
    ) -> bool {
        // Situation: We expect to be calling an operation with a parameter of `other`.
        // Can we call it with `self`?

        // All explicit input nodes in `other` must be present in `self`, with a *supertype* of their
        // counterpart in `other`. Note that this is contravariant - otherwise we could be calling
        // an operation (self) with a parameter that is less specific than it expects.
        if self.explicit_input_nodes.len() != other.explicit_input_nodes.len() {
            return false;
        }
        for (self_subst, other_subst) in self.explicit_input_nodes.iter().zip(other.explicit_input_nodes.iter()) {
            let self_key = self.subst_to_node_keys.get(self_subst).expect("internal error: missing subst marker in self");
            let other_key = other.subst_to_node_keys.get(other_subst).expect("internal error: missing subst marker in other");
            let self_type = self.parameter_graph.get_node_attr(*self_key).expect("internal error: missing node attribute in self");
            let other_type = other.parameter_graph.get_node_attr(*other_key).expect("internal error: missing node attribute in other");
            if !S::NodeMatcher::matches(other_type, self_type) { // NB: must have other <: self, not the opposite way!
                // Self's type is not a supertype of other's type.
                // Any caller working with the assumption of `other` could pass `self` a value that is not compatible with what it expects.
                return false;
            }
        }

        // Self's context graph must be a subgraph of other's context graph, with node types of self being supertypes of those in other.
        // Same for edges.
        // There is a subtle detail here though, which is that we currently statically store the context graph mapping at every call site.
        // This means somewhere we must also adapt that mapping to self, if we're exchanging the operation.
        // The "straightforward" way to make this automatic would be to have the context graph's IDs be externally visible and part
        // of the signature, then `self` would only be a subtype of `other` if all of its context graph's IDs are also present in `other`'s context graph,
        // with the same connections/shape.
        // TODO: implement above?

        // TODO: implement subgraph checks for context graph.

        true
    }
}