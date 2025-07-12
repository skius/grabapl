use crate::operation::signature::parameter::OperationParameter;
use crate::semantics::AbstractGraph;
use crate::util::bimap::BiMap;
use crate::{Graph, NodeKey, Semantics, SubstMarker};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ParameterBuilderError {
    #[error("Source marker not found in the parameter graph: {0:?}")]
    SourceMarkerNotFound(SubstMarker),
    #[error("Destination marker not found in the parameter graph: {0:?}")]
    DestinationMarkerNotFound(SubstMarker),
    #[error("Duplicate marker found in the parameter graph: {0:?}")]
    DuplicateMarker(SubstMarker),
}

pub struct OperationParameterBuilder<S: Semantics> {
    explicit_input_nodes: Vec<SubstMarker>,
    parameter_graph: AbstractGraph<S>,
    subst_to_node_keys: BiMap<SubstMarker, NodeKey>,
}

impl<S: Semantics> OperationParameterBuilder<S> {
    pub fn new() -> Self {
        OperationParameterBuilder {
            explicit_input_nodes: Vec::new(),
            parameter_graph: Graph::new(),
            subst_to_node_keys: BiMap::new(),
        }
    }

    pub fn next_subst_marker(&mut self) -> SubstMarker {
        let mut next_index = self.subst_to_node_keys.len();
        let mut next = next_index.to_string();
        while self
            .subst_to_node_keys
            .contains_left(&SubstMarker::from(&*next))
        {
            next_index += 1;
            next = next_index.to_string();
        }
        SubstMarker::from(next)
    }

    pub fn expect_explicit_input_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        av: S::NodeAbstract,
    ) -> Result<(), ParameterBuilderError> {
        let marker = marker.into();
        if self.subst_to_node_keys.contains_left(&marker) {
            return Err(ParameterBuilderError::DuplicateMarker(marker));
        }
        self.explicit_input_nodes.push(marker.clone());
        let node_key = self.parameter_graph.add_node(av);
        self.subst_to_node_keys.insert(marker, node_key);
        Ok(())
    }

    pub fn expect_context_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        av: S::NodeAbstract,
    ) -> Result<(), ParameterBuilderError> {
        let marker = marker.into();
        // Context nodes are not explicitly input nodes, but they are still part of the parameter graph.
        if self.subst_to_node_keys.contains_left(&marker) {
            return Err(ParameterBuilderError::DuplicateMarker(marker));
        }
        let node_key = self.parameter_graph.add_node(av);
        self.subst_to_node_keys.insert(marker, node_key);
        Ok(())
    }

    pub fn expect_edge(
        &mut self,
        src_marker: impl Into<SubstMarker>,
        dst_marker: impl Into<SubstMarker>,
        edge_attr: S::EdgeAbstract,
    ) -> Result<(), ParameterBuilderError> {
        let src_marker = src_marker.into();
        let dst_marker = dst_marker.into();
        let src_key = self
            .subst_to_node_keys
            .get_left(&src_marker)
            .ok_or(ParameterBuilderError::SourceMarkerNotFound(src_marker))?;
        let dst_key = self
            .subst_to_node_keys
            .get_left(&dst_marker)
            .ok_or(ParameterBuilderError::DestinationMarkerNotFound(dst_marker))?;
        self.parameter_graph.add_edge(*src_key, *dst_key, edge_attr);
        Ok(())
    }

    pub fn build(self) -> Result<OperationParameter<S>, ParameterBuilderError> {
        // TODO: check that all context nodes are linked with edges to explicit input nodes.

        Ok(OperationParameter {
            explicit_input_nodes: self.explicit_input_nodes,
            parameter_graph: self.parameter_graph,
            node_keys_to_subst: self.subst_to_node_keys.into_reversed(),
        })
    }
}
