use crate::operation::BuiltinOperation;
use crate::operation::signature::parameterbuilder::OperationParameterBuilder;
use crate::operation::signature::parameter::{
    AbstractOperationOutput, GraphWithSubstitution, OperationOutput, OperationParameter,
};
use crate::semantics::{AbstractGraph, ConcreteGraph, ConcreteToAbstract};
use crate::{Semantics, SubstMarker};
use std::collections::HashMap;

/// Operations that are available for every semantics.
#[derive(derive_more::Debug)]
pub enum LibBuiltinOperation<S: Semantics> {
    #[debug("AddNode")]
    AddNode { value: S::NodeConcrete },
    #[debug("AddEdge")]
    AddEdge {
        node_param: S::NodeAbstract,
        value: S::EdgeConcrete,
    },
    #[debug("RemoveNode")]
    RemoveNode { param: S::NodeAbstract },
    #[debug("RemoveEdge")]
    RemoveEdge {
        node_param: S::NodeAbstract,
        edge_param: S::EdgeAbstract,
    },
    #[debug("SetNode")]
    SetNode {
        param: S::NodeAbstract,
        value: S::NodeConcrete,
    },
}

// TODO: could potentially make this prettier.
//  Problem is derive(Clone) does not work since it requires S: Clone.
//  We could factor out all the Node/Edge Abstract/Concrete assoc types into a separate trait, and then
//  support making that clone.
//  Or we just require Semantics: Clone.
impl<S: Semantics> Clone for LibBuiltinOperation<S> {
    fn clone(&self) -> Self {
        match self {
            LibBuiltinOperation::AddNode { value } => LibBuiltinOperation::AddNode {
                value: value.clone(),
            },
            LibBuiltinOperation::AddEdge { node_param, value } => LibBuiltinOperation::AddEdge {
                node_param: node_param.clone(),
                value: value.clone(),
            },
            LibBuiltinOperation::RemoveNode { param } => LibBuiltinOperation::RemoveNode {
                param: param.clone(),
            },
            LibBuiltinOperation::RemoveEdge {
                node_param,
                edge_param,
            } => LibBuiltinOperation::RemoveEdge {
                node_param: node_param.clone(),
                edge_param: edge_param.clone(),
            },
            LibBuiltinOperation::SetNode { param, value } => LibBuiltinOperation::SetNode {
                param: param.clone(),
                value: value.clone(),
            },
        }
    }
}

impl<S: Semantics> LibBuiltinOperation<S> {
    pub fn parameter(&self) -> OperationParameter<S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            LibBuiltinOperation::AddNode { value } => {}
            LibBuiltinOperation::AddEdge { node_param, value } => {
                param_builder
                    .expect_explicit_input_node("src", node_param.clone())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", node_param.clone())
                    .unwrap();
            }
            LibBuiltinOperation::RemoveNode { param } => {
                param_builder
                    .expect_explicit_input_node("node", param.clone())
                    .unwrap();
            }
            LibBuiltinOperation::RemoveEdge {
                node_param,
                edge_param,
            } => {
                param_builder
                    .expect_explicit_input_node("src", node_param.clone())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", node_param.clone())
                    .unwrap();
                param_builder
                    .expect_edge("src", "dst", edge_param.clone())
                    .unwrap();
            }
            LibBuiltinOperation::SetNode { param, value } => {
                param_builder
                    .expect_explicit_input_node("node", param.clone())
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    pub fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> AbstractOperationOutput<S> {
        let mut new_node_names = HashMap::new();
        match self {
            LibBuiltinOperation::AddNode { value } => {
                g.add_node(
                    "new",
                    S::NodeConcreteToAbstract::concrete_to_abstract(value),
                );
                new_node_names.insert("new".into(), "new".into());
            }
            LibBuiltinOperation::AddEdge { node_param, value } => {
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    S::EdgeConcreteToAbstract::concrete_to_abstract(value),
                );
            }
            LibBuiltinOperation::RemoveNode { param } => {
                g.delete_node(SubstMarker::from("node"));
            }
            LibBuiltinOperation::RemoveEdge {
                node_param,
                edge_param,
            } => {
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"));
            }
            LibBuiltinOperation::SetNode { param, value } => {
                g.set_node_value(
                    SubstMarker::from("node"),
                    S::NodeConcreteToAbstract::concrete_to_abstract(value),
                );
            }
        }
        g.get_abstract_output(new_node_names)
    }

    pub fn apply(&self, g: &mut GraphWithSubstitution<ConcreteGraph<S>>) -> OperationOutput {
        let mut new_node_names = HashMap::new();
        match self {
            LibBuiltinOperation::AddNode { value } => {
                g.add_node("new", value.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            LibBuiltinOperation::AddEdge { node_param, value } => {
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    value.clone(),
                );
            }
            LibBuiltinOperation::RemoveNode { param } => {
                g.delete_node(SubstMarker::from("node"));
            }
            LibBuiltinOperation::RemoveEdge {
                node_param,
                edge_param,
            } => {
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"));
            }
            LibBuiltinOperation::SetNode { param, value } => {
                g.set_node_value(SubstMarker::from("node"), value.clone());
            }
        }
        g.get_concrete_output(new_node_names)
    }
}

impl<S: Semantics> BuiltinOperation for LibBuiltinOperation<S> {
    type S = S;

    fn parameter(&self) -> OperationParameter<S> {
        self.parameter()
    }

    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> AbstractOperationOutput<S> {
        self.apply_abstract(g)
    }

    fn apply(&self, g: &mut GraphWithSubstitution<ConcreteGraph<S>>) -> OperationOutput {
        self.apply(g)
    }
}
