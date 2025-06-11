use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;
use crate::{Graph, NodeKey, OperationContext, SubstMarker};
use crate::graph::operation::user_defined::UserDefinedOperation;
use crate::graph::semantics::SemanticsClone;

enum BuilderInstruction<S: SemanticsClone> {
    ExpectParameterNode(SubstMarker, S::NodeAbstract),
    ExpectContextNode(SubstMarker, S::NodeAbstract),
    ExpectParameterEdge(SubstMarker, SubstMarker, S::EdgeAbstract),
}

pub struct OperationBuilder<'a, S: SemanticsClone> {
    op_ctx: &'a OperationContext<S>,
    instructions: Vec<BuilderInstruction<S>>,
}

#[derive(Error, Debug)]
pub enum OperationBuilderError {
    #[error("Expected a new unique subst marker, found repeat: {0}")]
    ReusedSubstMarker(SubstMarker),
    #[error("Expected an existing subst marker, but {0} was not found")]
    NotFoundSubstMarker(SubstMarker),
}

impl<'a, S: SemanticsClone> OperationBuilder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Self {
            instructions: Vec::new(),
            op_ctx,
        }
    }

    pub fn expect_parameter_node(
        &mut self,
        marker: SubstMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions.push(BuilderInstruction::ExpectParameterNode(marker, node));
        Ok(())
    }

    pub fn expect_context_node(
        &mut self,
        marker: SubstMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions.push(BuilderInstruction::ExpectContextNode(marker, node));
        // TODO: check if subst marker does not exist yet
        Ok(())
    }

    pub fn expect_parameter_edge(
        &mut self,
        source_marker: SubstMarker,
        target_marker: SubstMarker,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions.push(BuilderInstruction::ExpectParameterEdge(
            source_marker, target_marker, edge,
        ));
        // TODO: check if both subst markers are valid
        Ok(())
    }
    
    pub fn start_query(
        &mut self,
        query: S::BuiltinQuery,
        args: Vec<SubstMarker>
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        Ok(())
    }
    
    pub fn enter_true_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        Ok(())
    }
    
    pub fn enter_false_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        Ok(())
    }
    
    pub fn add_instruction(
        &mut self,
        instruction: S::BuiltinOperation,
        args: Vec<SubstMarker>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        Ok(())
    }
    
    pub fn build(self, self_op_id: SubstMarker) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // Here we would typically finalize the operation and return it.
        // For now, we just return Ok to indicate success.
        todo!()
    }
}

impl<'a, S: SemanticsClone<NodeAbstract: Debug, EdgeAbstract: Debug>> OperationBuilder<'a, S> {
    /// Visualizes the current state of the operation builder.
    /// Provides context on the current nest level as well as the DOT representation of the graph
    /// at the current cursor.
    pub fn show_state(&self) -> String {
        let (g, subst_to_node_keys) = self.build_debug_graph_at_current_point();
        let dot = g.dot();


        let mut result = String::new();

        result.push_str(&"Current Operation Builder State:\n".to_string());
        result.push_str(&"Graph at current point:\n".to_string());
        result.push_str(&dot);
        result
    }

    fn build_debug_graph_at_current_point(
        &self,
    ) -> (Graph<S::NodeAbstract, S::EdgeAbstract>, HashMap<SubstMarker, NodeKey>) {
        let mut g = Graph::new();
        let mut subst_to_node_keys: HashMap<SubstMarker, NodeKey> = HashMap::new();

        for instruction in &self.instructions {
            match instruction {
                BuilderInstruction::ExpectParameterNode(marker, node) => {
                    let key = g.add_node(node.clone());
                    subst_to_node_keys.insert(*marker, key);
                }
                BuilderInstruction::ExpectContextNode(marker, node) => {
                    let key = g.add_node(node.clone());
                    subst_to_node_keys.insert(*marker, key);
                }
                BuilderInstruction::ExpectParameterEdge(source_marker, target_marker, edge) => {
                    let source_key = *subst_to_node_keys
                        .get(source_marker)
                        .expect("Source marker not found in subst_to_node_keys");
                    let target_key = *subst_to_node_keys
                        .get(target_marker)
                        .expect("Target marker not found in subst_to_node_keys");
                    g.add_edge(
                        source_key,
                        target_key,
                        edge.clone(),
                    );
                }
            }
        }

        (g, subst_to_node_keys)
    }
}
























