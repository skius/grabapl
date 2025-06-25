use thiserror::Error;
use crate::{Graph, NodeKey, Semantics, SubstMarker};
use crate::graph::pattern::OperationParameter;
use crate::graph::semantics::AbstractGraph;
use crate::util::bimap::BiMap;

#[derive(Debug, Error)]
pub enum ParameterBuilderError {
    #[error("Source marker not found in the parameter graph: {0}")]
    SourceMarkerNotFound(SubstMarker),
    #[error("Destination marker not found in the parameter graph: {0}")]
    DestinationMarkerNotFound(SubstMarker),
    #[error("Duplicate marker found in the parameter graph: {0}")]
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

    pub fn expect_explicit_input_node(&mut self, marker: SubstMarker, av: S::NodeAbstract) -> Result<(), ParameterBuilderError> {
        if self.subst_to_node_keys.contains_left(&marker) {
            return Err(ParameterBuilderError::DuplicateMarker(marker));
        }
        self.explicit_input_nodes.push(marker);
        let node_key = self.parameter_graph.add_node(av);
        self.subst_to_node_keys.insert(marker, node_key);
        Ok(())
    }

    pub fn expect_context_node(&mut self, marker: SubstMarker, av: S::NodeAbstract) -> Result<(), ParameterBuilderError> {
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
        src_marker: SubstMarker,
        dst_marker: SubstMarker,
        edge_attr: S::EdgeAbstract,
    ) -> Result<(), ParameterBuilderError> {
        let src_key = self.subst_to_node_keys.get_left(&src_marker)
            .ok_or(ParameterBuilderError::SourceMarkerNotFound(src_marker))?;
        let dst_key = self.subst_to_node_keys.get_left(&dst_marker)
            .ok_or(ParameterBuilderError::DestinationMarkerNotFound(dst_marker))?;
        self.parameter_graph.add_edge(*src_key, *dst_key, edge_attr);
        Ok(())
    }

    pub fn build(self) -> Result<OperationParameter<S>, ParameterBuilderError> {
        // TODO: check that all context nodes are linked with edges to explicit input nodes.
        
        let (subst_to_node_keys, node_keys_to_subst) = self.subst_to_node_keys.into_inner();
        Ok(OperationParameter {
            explicit_input_nodes: self.explicit_input_nodes,
            parameter_graph: self.parameter_graph,
            subst_to_node_keys,
            node_keys_to_subst,
        })
    }
}