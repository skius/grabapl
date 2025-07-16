use crate::SubstMarker;
use crate::operation::signature::parameter::{
    AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, NodeMarker,
    OperationParameter,
};
use crate::semantics::{AbstractGraph, AbstractJoin, AbstractMatcher, Semantics};
use crate::util::bimap::BiMap;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub mod parameter;
pub mod parameterbuilder;

pub type AbstractSignatureEdgeId = (AbstractSignatureNodeId, AbstractSignatureNodeId);
pub type ParameterEdgeId = (SubstMarker, SubstMarker);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound = "S: crate::serde::SemanticsSerde")
)]
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

impl<S: Semantics> Clone for OperationSignature<S> {
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
    pub fn empty_new(name: impl Into<String>, parameter: OperationParameter<S>) -> Self {
        OperationSignature {
            name: name.into(),
            parameter,
            output: AbstractOutputChanges::new(),
        }
    }

    pub fn new_noop(name: impl Into<String>) -> Self {
        OperationSignature {
            name: name.into(),
            parameter: OperationParameter::new_empty(),
            output: AbstractOutputChanges::new(),
        }
    }
}

/// The changes to the graph that an operation will cause.
#[derive(derive_more::Debug)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound = "S: crate::serde::SemanticsSerde")
)]
pub struct AbstractOutputChanges<S: Semantics> {
    /// New nodes that are guaranteed to be created with a value of the given type.
    pub new_nodes: HashMap<AbstractOutputNodeMarker, S::NodeAbstract>,
    /// New edges that are guaranteed to be created with a value of the given type.
    pub new_edges: HashMap<AbstractSignatureEdgeId, S::EdgeAbstract>,
    /// Pre-existing nodes that may have been modified to be of the given type.
    pub maybe_changed_nodes: HashMap<SubstMarker, S::NodeAbstract>,
    /// Pre-existing edges that may have been modified to be of the given type.
    pub maybe_changed_edges: HashMap<ParameterEdgeId, S::EdgeAbstract>,
    // TODO: think about also having "must_changed_nodes" and "must_changed_edges" here,
    //  which would be useful for:
    //  1. More precise states, since the builder would not have to join the must-written value with the old value - it knows that the old value is not needed!
    //  2. More coherence, since builtin operation currently act this way when executed directly inside an operation.
    //     i.e., they do not join the old value with the new value, but rather overwrite it, since they know for a fact what happens.
    //  And they might be easy to compute:
    //  1. At the end of an operation, if we see that an AID x for which we have maybe written an AV t is statically
    //  known to be of AV t, then we can pretend that it was must-changed.
    //  This is useful in eg such a case:
    //  foo(x: Object) must_change x: Integer { if ... { x = 2 } else { x = 3 } // x: Integer now! }
    //  x: Object. Call foo(x). Now we know x: Integer. Without must_changed, x: Object still.
    /// Pre-existing nodes that may have been deleted by the operation.
    pub maybe_deleted_nodes: HashSet<SubstMarker>,
    /// Pre-existing edges that may have been deleted by the operation.
    pub maybe_deleted_edges: HashSet<ParameterEdgeId>,
}

impl<S: Semantics> Clone for AbstractOutputChanges<S> {
    fn clone(&self) -> Self {
        AbstractOutputChanges {
            new_nodes: self.new_nodes.clone(),
            new_edges: self.new_edges.clone(),
            maybe_changed_nodes: self.maybe_changed_nodes.clone(),
            maybe_changed_edges: self.maybe_changed_edges.clone(),
            maybe_deleted_nodes: self.maybe_deleted_nodes.clone(),
            maybe_deleted_edges: self.maybe_deleted_edges.clone(),
        }
    }
}

impl<S: Semantics> AbstractOutputChanges<S> {
    pub fn new() -> Self {
        AbstractOutputChanges {
            new_nodes: HashMap::new(),
            new_edges: HashMap::new(),
            maybe_changed_nodes: HashMap::new(),
            maybe_changed_edges: HashMap::new(),
            maybe_deleted_nodes: HashSet::new(),
            maybe_deleted_edges: HashSet::new(),
        }
    }

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
        for (marker, self_type) in &self.maybe_changed_nodes {
            if let Some(other_type) = other.maybe_changed_nodes.get(marker) {
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
        for (edge_id, self_type) in &self.maybe_changed_edges {
            if let Some(other_type) = other.maybe_changed_edges.get(edge_id) {
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
        for marker in &self.maybe_deleted_nodes {
            if !other.maybe_deleted_nodes.contains(marker) {
                // `self` would delete a node that `other` expects to be present.
                // Any caller working with the assumption of `other` would assume this node exists and crash.
                return false;
            }
        }
        for edge_id in &self.maybe_deleted_edges {
            if !other.maybe_deleted_edges.contains(edge_id) {
                // `self` would delete an edge that `other` expects to be present.
                // Any caller working with the assumption of `other` would assume this edge exists and crash.
                return false;
            }
        }

        true
    }

    pub fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> AbstractOperationOutput<S> {
        let mut output_names = BiMap::new();

        // handle new nodes
        for (name, av) in &self.new_nodes {
            let nnm = g.new_node_marker();
            g.add_node(nnm.clone(), av.clone());
            output_names.insert(nnm, name.clone());
        }

        let sig_id_to_node_marker = |sig_id: AbstractSignatureNodeId| {
            match sig_id {
                AbstractSignatureNodeId::ExistingNode(subst) => NodeMarker::Subst(subst),
                AbstractSignatureNodeId::NewNode(name) => {
                    // find in output_names
                    let nnm = output_names
                        .get_right(&name)
                        .expect("internal error: signature node not found in output names");
                    NodeMarker::New(*nnm)
                }
            }
        };

        // handle new edges
        for ((src, dst), av) in &self.new_edges {
            let src_marker = sig_id_to_node_marker(*src);
            let dst_marker = sig_id_to_node_marker(*dst);
            g.add_edge(src_marker, dst_marker, av.clone());
        }

        // Important for the maybe-changed values:
        // Since they're only maybe-changed, it could be that they're not changed.
        // In other words, we need to indicate that the join of the old value and the new value is the most precise abstract value to give.

        // handle changed nodes
        for (subst, av) in &self.maybe_changed_nodes {
            let node_marker = NodeMarker::Subst(*subst);
            g.maybe_set_node_value(node_marker, av.clone(), S::NodeJoin::join)
                .unwrap();
        }
        // handle changed edges
        for ((src, dst), av) in &self.maybe_changed_edges {
            let src_marker = NodeMarker::Subst(*src);
            let dst_marker = NodeMarker::Subst(*dst);
            g.maybe_set_edge_value(src_marker, dst_marker, av.clone(), S::EdgeJoin::join)
                .unwrap();
        }

        // handle removed nodes
        for subst in &self.maybe_deleted_nodes {
            let node_marker = NodeMarker::Subst(*subst);
            g.delete_node(node_marker);
        }
        // handle removed edges
        for (src, dst) in &self.maybe_deleted_edges {
            let src_marker = NodeMarker::Subst(*src);
            let dst_marker = NodeMarker::Subst(*dst);
            g.delete_edge(src_marker, dst_marker);
        }

        let (output_names, _) = output_names.into_inner();
        g.get_abstract_output(output_names)
    }
}

impl<S: Semantics> OperationParameter<S> {
    /// Returns `true` if `self` can be used *as a parameter* wherever `other` is expected, i.e., `self <: other`.
    /// Note that this is a parameter, so it is contravariant when looked at as part of a function type.
    pub fn is_subtype_of(&self, other: &OperationParameter<S>) -> bool {
        // Situation: We expect to be calling an operation with a parameter of `other`.
        // Can we call it with `self`?

        // All explicit input nodes in `other` must be present in `self`, with a *supertype* of their
        // counterpart in `other`. Note that this is contravariant - otherwise we could be calling
        // an operation (self) with a parameter that is less specific than it expects.
        if self.explicit_input_nodes.len() != other.explicit_input_nodes.len() {
            return false;
        }
        for (self_subst, other_subst) in self
            .explicit_input_nodes
            .iter()
            .zip(other.explicit_input_nodes.iter())
        {
            let self_key = self
                .node_keys_to_subst
                .get_right(self_subst)
                .expect("internal error: missing subst marker in self");
            let other_key = other
                .node_keys_to_subst
                .get_right(other_subst)
                .expect("internal error: missing subst marker in other");
            let self_type = self
                .parameter_graph
                .get_node_attr(*self_key)
                .expect("internal error: missing node attribute in self");
            let other_type = other
                .parameter_graph
                .get_node_attr(*other_key)
                .expect("internal error: missing node attribute in other");
            if !S::NodeMatcher::matches(other_type, self_type) {
                // NB: must have other <: self, not the opposite way!
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
        // TODO: implement above? (it's kind of implemented - UDOps store the context graph mapping by context node ID.)

        // TODO: implement subgraph checks for context graph.

        true
    }
}
