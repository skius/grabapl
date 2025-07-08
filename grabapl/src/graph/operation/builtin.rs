use std::collections::HashMap;
use crate::graph::operation::parameterbuilder::OperationParameterBuilder;
use crate::graph::pattern::{AbstractOperationOutput, GraphWithSubstitution, OperationOutput, OperationParameter};
use crate::graph::semantics::{AbstractGraph, ConcreteGraph, ConcreteToAbstract, SemanticsClone};
use crate::{Semantics, SubstMarker};

/// Operations that are available for every semantics.
pub enum LibBuiltinOperation<S: Semantics> {
    AddNode {
        value: S::NodeConcrete,
    },
    AddEdge {
        node_param: S::NodeAbstract,
        value: S::EdgeConcrete,
    },
    RemoveNode {
        param: S::NodeAbstract,
    },
    RemoveEdge {
        node_param: S::NodeAbstract,
        edge_param: S::EdgeAbstract,
    },
    SetNode {
        param: S::NodeAbstract,
        value: S::NodeConcrete,
    },
}

impl<S: SemanticsClone> LibBuiltinOperation<S> {
    pub fn parameter(&self) -> OperationParameter<S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            LibBuiltinOperation::AddNode { value } => {
                
            }
            LibBuiltinOperation::AddEdge { node_param, value } => {
                param_builder.expect_explicit_input_node("src", node_param.clone()).unwrap();
                param_builder.expect_explicit_input_node("dst", node_param.clone()).unwrap();
            }
            LibBuiltinOperation::RemoveNode { param } => {
                param_builder.expect_explicit_input_node("node", param.clone()).unwrap();
            }
            LibBuiltinOperation::RemoveEdge { node_param, edge_param } => {
                param_builder.expect_explicit_input_node("src", node_param.clone()).unwrap();
                param_builder.expect_explicit_input_node("dst", node_param.clone()).unwrap();
                param_builder.expect_edge("src", "dst", edge_param.clone()).unwrap();
            }
            LibBuiltinOperation::SetNode { param, value } => {
                param_builder.expect_explicit_input_node("node", param.clone()).unwrap();
            }
        }
        param_builder.build().unwrap()
    }
    
    pub fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<S>>) -> AbstractOperationOutput<S> {
        let mut new_node_names = HashMap::new();
        match self {
            LibBuiltinOperation::AddNode { value } => {
                g.add_node("new", S::NodeConcreteToAbstract::concrete_to_abstract(value));
                new_node_names.insert("new".into(), "new".into());
            }
            LibBuiltinOperation::AddEdge { node_param, value } => {
                g.add_edge(SubstMarker::from("src"), SubstMarker::from("dst"), S::EdgeConcreteToAbstract::concrete_to_abstract(value));
            }
            LibBuiltinOperation::RemoveNode { param } => {
                g.delete_node(SubstMarker::from("node"));
            }
            LibBuiltinOperation::RemoveEdge { node_param, edge_param } => {
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"));
            }
            LibBuiltinOperation::SetNode { param, value } => {
                g.set_node_value(SubstMarker::from("node"), S::NodeConcreteToAbstract::concrete_to_abstract(value));
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
                g.add_edge(SubstMarker::from("src"), SubstMarker::from("dst"), value.clone());
            }
            LibBuiltinOperation::RemoveNode { param } => {
                g.delete_node(SubstMarker::from("node"));
            }
            LibBuiltinOperation::RemoveEdge { node_param, edge_param } => {
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"));
            }
            LibBuiltinOperation::SetNode { param, value } => {
                g.set_node_value(SubstMarker::from("node"), value.clone());
            }
        }
        g.get_concrete_output(new_node_names)
    }
}