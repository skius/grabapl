use crate::graph::operation::query::{BuiltinQuery, GraphShapeQuery, ShapeNodeIdentifier};
use crate::graph::operation::user_defined::{AbstractNodeId, AbstractOperationArgument, AbstractOperationResultMarker, AbstractUserDefinedOperationOutput, QueryInstructions, UserDefinedOperation};
use crate::graph::operation::{BuiltinOperation, OperationError, get_substitution};
use crate::graph::pattern::{AbstractOutputNodeMarker, GraphWithSubstitution, OperationParameter, ParameterSubstitution};
use crate::graph::semantics::{AbstractGraph, AbstractMatcher, SemanticsClone};
use crate::util::bimap::BiMap;
use crate::{Graph, NodeKey, OperationContext, OperationId, SubstMarker};
use petgraph::dot;
use petgraph::dot::Dot;
use petgraph::prelude::GraphMap;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::iter::Peekable;
use std::marker::PhantomData;
use std::mem;
use std::slice::Iter;
use thiserror::Error;
/*
General overview:

1. While building, the builder just stores the messages sent to it.
We cannot do fancy compile-time checks like "every query has a condition and two branches", because
every step of that (condition, true branch, false branch) should be interruptible and resumable.
E.g., a frontend needs to be able to give intermediate feedback to the user, so that the user
can work with that feedback and send new messages to the builder.

However, to give good feedback for which messages are appropriate, we construct the operation on the fly (TODO: cache this?),
so that errors like invalid identifiers or ending a query without starting one can be caught immediately at message-time.
This is the same routine that can provide state feedback to the user like:
 * right now you're in this branch of that query
 * the abstract graph looks like this
 * more ???

The intermediate state returns a graph and a hashmap from nodes and edges to additional metadata, like their abstract node id.
*/

pub enum BuilderOpLike<S: SemanticsClone> {
    Builtin(S::BuiltinOperation),
    FromOperationId(OperationId),
    Recurse,
}

// TODO: perhaps this should include a "GiveNodeExplicitName" instruction that gives a node a name of a single string?
//  this would need to be a variant of AbstractNodeId.
#[derive(derive_more::Debug)]
enum BuilderInstruction<S: SemanticsClone> {
    #[debug("ExpectParameterNode({_0:?}, ???)")]
    ExpectParameterNode(SubstMarker, S::NodeAbstract),
    #[debug("ExpectContextNode({_0:?}, ???)")]
    ExpectContextNode(SubstMarker, S::NodeAbstract),
    #[debug("ExpectParameterEdge({_0:?}, {_1:?}, ???)")]
    ExpectParameterEdge(SubstMarker, SubstMarker, S::EdgeAbstract),
    #[debug("StartQuery(???, args: {_1:?})")]
    StartQuery(S::BuiltinQuery, Vec<AbstractNodeId>),
    #[debug("EnterTrueBranch")]
    EnterTrueBranch,
    #[debug("EnterFalseBranch")]
    EnterFalseBranch,
    // TODO: think about what happens when we start two shape queries with the same name. the gsq_op_marker if statement below somewhere is a problem.
    //  specifically, when they're nested (eg one with name "foo", true branch, another one with "foo").
    //  potentially could be fine to support, but needs implementation work.
    #[debug("StartShapeQuery({_0:?})")]
    StartShapeQuery(AbstractOperationResultMarker),
    #[debug("EndQuery")]
    EndQuery,
    #[debug("ExpectShapeNode({_0:?}, ???)")]
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:?}, {_1:?}, ???)")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
    #[debug("AddNamedOperation({_0:?}, ???, args: {_2:?})")]
    AddNamedOperation(
        AbstractOperationResultMarker,
        BuilderOpLike<S>,
        Vec<AbstractNodeId>,
    ),
    #[debug("AddOperation(???, args: {_1:?})")]
    AddOperation(BuilderOpLike<S>, Vec<AbstractNodeId>),
    #[debug("ReturnNode({_0:?}, {_1:?}, ???)")]
    ReturnNode(AbstractNodeId, AbstractOutputNodeMarker, S::NodeAbstract)
}

#[derive(Error, Debug, Clone)]
pub enum OperationBuilderError {
    #[error("Expected a new unique subst marker, found repeat: {0:?}")]
    ReusedSubstMarker(SubstMarker),
    #[error("Expected an existing subst marker, but {0:?} was not found")]
    NotFoundSubstMarker(SubstMarker),
    #[error("Expected a new unique subst marker, found repeat: {0:?}")]
    ReusedShapeIdent(ShapeNodeIdentifier),
    #[error("Cannot call this while in a query context")]
    InvalidInQuery,
    #[error("Expected an operation or query")]
    ExpectedOperationOrQuery,
    #[error("Already visited the {0} branch of the active query")]
    AlreadyVisitedBranch(bool),
    #[error("Could not find abstract node id: {0:?}")]
    NotFoundAid(AbstractNodeId),
    #[error("Could not find operation ID: {0}")]
    NotFoundOperationId(OperationId),
    #[error("Could not apply operation due to mismatched arguments: {0}")]
    SubstitutionError(#[from] crate::graph::operation::SubstitutionError),
    #[error("Could not abstractly apply operation {0} due to: {1}")]
    AbstractApplyOperationError(OperationId, OperationError),
    #[error("Superfluous instruction {0}")]
    SuperfluousInstruction(String),
    #[error("Already selected to return node {0:?}")]
    AlreadySelectedReturnNode(AbstractNodeId),
    #[error("Could not find AID {0:?} for return node")]
    NotFoundReturnNode(AbstractNodeId),
    #[error("Invalid return node type for AID {0:?}, must be more generic")]
    InvalidReturnNodeType(AbstractNodeId),
    // TODO: document why this is not allowed ...
    //  in general, add lots more documentation.
    #[error("Returned {0:?} node may have been created by a shape query, which is not allowed")]
    ReturnNodeMayOriginateFromShapeQuery(AbstractNodeId),
    #[error("Cannot return a parameter node: {0:?}")]
    CannotReturnParameter(AbstractNodeId),
}

pub struct OperationBuilder<'a, S: SemanticsClone> {
    op_ctx: &'a OperationContext<S>,
    instructions: Vec<BuilderInstruction<S>>,
}

// TODO: all message adding, validate all args by building temp graph
impl<'a, S: SemanticsClone<BuiltinQuery: Clone, BuiltinOperation: Clone>> OperationBuilder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Self {
            instructions: Vec::new(),
            op_ctx,
        }
    }

    // TODO: add undo_one_instruction method that just pops the last instruction.
    pub fn undo_last_instruction(&mut self) {
        if !self.instructions.is_empty() {
            self.instructions.pop();
        }
        self.check_instructions().expect("internal error: a prefix slice of instructions should always be valid");
    }

    pub fn expect_parameter_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let marker = marker.into();
        self.instructions
            .push(BuilderInstruction::ExpectParameterNode(marker, node));
        self.check_instructions_or_rollback()
    }

    pub fn expect_context_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let marker = marker.into();
        self.instructions
            .push(BuilderInstruction::ExpectContextNode(marker, node));
        // TODO: check if subst marker does not exist yet
        self.check_instructions_or_rollback()
    }

    pub fn expect_parameter_edge(
        &mut self,
        source_marker: impl Into<SubstMarker>,
        target_marker: impl Into<SubstMarker>,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let source_marker = source_marker.into();
        let target_marker = target_marker.into();
        self.instructions
            .push(BuilderInstruction::ExpectParameterEdge(
                source_marker,
                target_marker,
                edge,
            ));
        // TODO: check if both subst markers are valid
        self.check_instructions_or_rollback()
    }

    pub fn start_query(
        &mut self,
        query: S::BuiltinQuery,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions
            .push(BuilderInstruction::StartQuery(query, args));
        self.check_instructions_or_rollback()
    }

    pub fn enter_true_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::EnterTrueBranch);
        self.check_instructions_or_rollback()
    }

    pub fn enter_false_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::EnterFalseBranch);
        self.check_instructions_or_rollback()
    }

    // TODO: get rid of AbstractOperationResultMarker requirement. Either completely or make it optional and autogenerate one.
    //  How to specify which shape node? ==> the shape node markers should be unique per path
    pub fn start_shape_query(
        &mut self,
        op_marker: AbstractOperationResultMarker,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions
            .push(BuilderInstruction::StartShapeQuery(op_marker));
        self.check_instructions_or_rollback()
    }

    pub fn end_query(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::EndQuery);
        self.check_instructions_or_rollback()
    }

    // TODO: should expect_*_node really expect a marker? maybe it should instead return a marker?
    //  it could also take an Option<Marker> so that it can autogenerate one if it's none so the caller doesn't have to deal with it.
    pub fn expect_shape_node(
        &mut self,
        marker: AbstractOutputNodeMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO: check that any shape nodes are not free floating. maybe this should be in a GraphShapeQuery validator?
        self.instructions
            .push(BuilderInstruction::ExpectShapeNode(marker, node));
        self.check_instructions_or_rollback()
    }

    pub fn expect_shape_edge(
        &mut self,
        source: AbstractNodeId,
        target: AbstractNodeId,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO:
        self.instructions
            .push(BuilderInstruction::ExpectShapeEdge(source, target, edge));
        self.check_instructions_or_rollback()
    }

    pub fn add_named_operation(
        &mut self,
        name: AbstractOperationResultMarker,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // TODO
        self.instructions
            .push(BuilderInstruction::AddNamedOperation(name, op, args));
        self.check_instructions_or_rollback()
    }

    pub fn add_operation(
        &mut self,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions
            .push(BuilderInstruction::AddOperation(op, args));
        self.check_instructions_or_rollback()
    }

    /// Indicate that a node should be marked in the output with the given abstract value.
    ///
    /// Note that the abstract value must be a supertype of the node's statically determined type.
    /// Also, the node must be visible in the end context of the operation, and must never have
    /// been statically determined by a shape query.
    ///
    /// These instructions must be the very last instructions in the operation builder.
    pub fn return_node(
        &mut self,
        aid: AbstractNodeId,
        output_marker: AbstractOutputNodeMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // dont support returning parameter nodes
        if let AbstractNodeId::ParameterMarker(..) = &aid {
            return Err(OperationBuilderError::CannotReturnParameter(aid));
        }
        self.instructions.push(
            BuilderInstruction::ReturnNode(aid, output_marker, node),
        );
        self.check_instructions_or_rollback()
    }

    // TODO: This should run further post processing checks.
    //  Stuff like Context nodes must be connected, etc.
    pub fn build(
        &self,
        self_op_id: OperationId,
    ) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // Here we would typically finalize the operation and return it.
        // For now, we just return Ok to indicate success.

        let builder_result =
            IntermediateStateBuilder::run(&self.instructions, self.op_ctx)?;

        let param = builder_result.operation_parameter;
        let instructions = builder_result.instructions;

        let mut interpreter =
            IntermediateInterpreter::new_for_self_op_id(self_op_id, param, self.op_ctx);

        let user_def_op = interpreter.create_user_defined_operation(instructions, builder_result.return_nodes)?;

        Ok(user_def_op)
    }

    fn check_instructions_or_rollback(&mut self) -> Result<(), OperationBuilderError> {
        match self.check_instructions() {
            Ok(_) => Ok(()),
            Err(e) => {
                // If the instructions are invalid, we rollback the last instruction.
                // This is a simple rollback mechanism, but could be improved.
                if !self.instructions.is_empty() {
                    self.instructions.pop();
                }
                Err(e)
            }
        }
    }

    fn check_instructions(&self) -> Result<(), OperationBuilderError> {
        let builder_result = IntermediateStateBuilder::run(&self.instructions, self.op_ctx)?;
        // TODO: how do we pass builder_result.return_nodes to the interpreter?
        //  maybe have a check_validity function?
        //  Or we could just call create_user_defined_operation directly here and check its result.
        let mut interpreter = IntermediateInterpreter::new_for_self_op_id(
            0, // Unused. TODO: make prettier...
            builder_result.operation_parameter,
            self.op_ctx,
        );
        let _ = interpreter.create_user_defined_operation(builder_result.instructions, builder_result.return_nodes)?;
        Ok(())
    }
}

impl<
    'a,
    S: SemanticsClone<
            NodeAbstract: Debug,
            EdgeAbstract: Debug,
            BuiltinOperation: Clone,
            BuiltinQuery: Clone,
        >,
> OperationBuilder<'a, S>
{
    fn get_intermediate_state(
        &self,
    ) -> Result<(IntermediateState<S>, Vec<IntermediateStatePath>), OperationBuilderError> {
        let builder_result =
            IntermediateStateBuilder::run(&self.instructions, self.op_ctx)?;
        let mut interpreter = IntermediateInterpreter::new_for_self_op_id(
            0, // TODO: use a real operation ID here
            builder_result.operation_parameter,
            self.op_ctx,
        );

        let (_, interp_instructions) = interpreter.interpret_instructions(builder_result.instructions)?;
        let path = builder_result.state_path;
        let mut path_iter = path.iter().peekable().cloned();

        let mut intermediate_state = get_state_for_path(
            &interpreter.initial_state,
            &interp_instructions,
            &mut path_iter,
        )
        .expect("internal error: Failed to get intermediate state for path");

        let query_path = get_query_path_for_path::<S>(&mut path.iter().peekable().cloned());
        // TODO: make this prettier. should be automatically computed.
        intermediate_state.query_path = query_path;

        // let dot = intermediate_state.graph.dot();
        // let mapping = intermediate_state.node_keys_to_aid.into_inner().0;
        // let query_path = intermediate_state.query_path;

        Ok((intermediate_state, path))
    }

    /// Visualizes the current state of the operation builder.
    /// Provides context on the current nest level as well as the DOT representation of the graph
    /// at the current cursor.
    pub fn show_state(&self) -> Result<IntermediateState<S>, OperationBuilderError> {
        // let (g, subst_to_node_keys) = self.build_debug_graph_at_current_point();
        // let dot = g.dot();
        //
        // let mut result = String::new();
        //
        // result.push_str(&"Current Operation Builder State:\n".to_string());
        // result.push_str(&"Graph at current point:\n".to_string());
        // result.push_str(&dot);
        // result

        Ok(self.get_intermediate_state()?.0)
    }

    pub fn format_state(&self) -> String {
        // TODO: should probably return a Result
        let (state, path) = self.get_intermediate_state().unwrap();
        let dot = state.graph.dot();
        let mapping = state.node_keys_to_aid.into_inner().0;
        let query_path = state.query_path;
        format!("\nIntermediate State:\n{dot}\nmapping: {mapping:#?}\nTODO query path")
    }

    fn build_debug_graph_at_current_point(
        &self,
    ) -> (
        Graph<S::NodeAbstract, S::EdgeAbstract>,
        HashMap<SubstMarker, NodeKey>,
    ) {
        let mut g = Graph::new();
        let mut subst_to_node_keys: HashMap<SubstMarker, NodeKey> = HashMap::new();

        for instruction in &self.instructions {
            match instruction {
                BuilderInstruction::ExpectParameterNode(marker, node) => {
                    let key = g.add_node(node.clone());
                    subst_to_node_keys.insert(marker.clone(), key);
                }
                BuilderInstruction::ExpectContextNode(marker, node) => {
                    let key = g.add_node(node.clone());
                    subst_to_node_keys.insert(marker.clone(), key);
                }
                BuilderInstruction::ExpectParameterEdge(source_marker, target_marker, edge) => {
                    let source_key = *subst_to_node_keys
                        .get(source_marker)
                        .expect("Source marker not found in subst_to_node_keys");
                    let target_key = *subst_to_node_keys
                        .get(target_marker)
                        .expect("Target marker not found in subst_to_node_keys");
                    g.add_edge(source_key, target_key, edge.clone());
                }
                _ => {
                    eprintln!("Skipping instruction");
                }
            }
        }

        (g, subst_to_node_keys)
    }
}

struct IntermediateStateBuilder<'a, S: SemanticsClone> {
    path: Vec<IntermediateStatePath>,
    _phantom_data: PhantomData<&'a S>,
}

use super::user_defined::Instruction as UDInstruction;

#[derive(derive_more::Debug)]
enum IntermediateInstruction<S: SemanticsClone> {
    OpLike(IntermediateOpLike<S>),
    #[debug("GraphShapeQuery({_0:#?}, {_1:#?}, {_2:#?})")]
    GraphShapeQuery(
        AbstractOperationResultMarker,
        Vec<GraphShapeQueryInstruction<S>>,
        IntermediateQueryInstructions<S>,
    ),
    #[debug("BuiltinQuery(???, {_1:#?}, {_2:#?})")]
    BuiltinQuery(
        S::BuiltinQuery,
        Vec<AbstractNodeId>,
        IntermediateQueryInstructions<S>,
    ),
}

#[derive(derive_more::Debug)]
enum IntermediateOpLike<S: SemanticsClone> {
    #[debug("Builtin(???, {_1:#?})")]
    Builtin(S::BuiltinOperation, Vec<AbstractNodeId>),
    Operation(OperationId, Vec<AbstractNodeId>),
    Recurse(Vec<AbstractNodeId>),
}

#[derive(derive_more::Debug)]
struct IntermediateQueryInstructions<S: SemanticsClone> {
    #[debug("[{}]", true_branch.iter().map(|(opt, inst)| format!("({opt:#?}, {:#?})", inst)).collect::<Vec<_>>().join(", "))]
    true_branch: Vec<(
        Option<AbstractOperationResultMarker>,
        IntermediateInstruction<S>,
    )>,
    #[debug("[{}]", false_branch.iter().map(|(opt, inst)| format!("({opt:#?}, {:#?})", inst)).collect::<Vec<_>>().join(", "))]
    false_branch: Vec<(
        Option<AbstractOperationResultMarker>,
        IntermediateInstruction<S>,
    )>,
}

#[derive(derive_more::Debug)]
enum GraphShapeQueryInstruction<S: SemanticsClone> {
    #[debug("ExpectShapeNode({_0:#?})")]
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:#?}, {_1:#?})")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
}

struct BuilderResult<S: SemanticsClone> {
    operation_parameter: OperationParameter<S>,
    instructions: Vec<(
        Option<AbstractOperationResultMarker>,
        IntermediateInstruction<S>,
    )>,
    state_path: Vec<IntermediateStatePath>,
    return_nodes: HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
}

// TODO: maybe this is not *intermediate* but actually the final state as well potentially?
impl<'a, S: SemanticsClone<BuiltinOperation: Clone, BuiltinQuery: Clone>>
    IntermediateStateBuilder<'a, S>
{
    fn run(
        builder_instructions: &'a [BuilderInstruction<S>],
        op_ctx: &'a OperationContext<S>,
    ) -> Result<
        BuilderResult<S>,
        OperationBuilderError,
    > {
        /*
        General idea:
        Whenever we see start_query (or start_shape_query), we push a query state onto a stack.
        When we see

        */

        //
        // enum QueryBranchState {
        //     // if we haven't encountered an enter_*_branch message yet
        //     NoBranch,
        //     TrueBranch,
        //     FalseBranch,
        // }
        // struct QueryState<S: SemanticsClone> {
        //     true_instructions: Vec<UDInstruction<S>>,
        //     false_instructions: Vec<UDInstruction<S>>,
        //     current_branch: QueryBranchState,
        // }
        //
        // enum StackState {
        //
        // }
        //
        // #[derive(Clone, Copy, Debug)]
        // enum State {
        //     BuildingParameterGraph,
        //     ExpectingInstruction,
        //     BuildingQuery,
        //     BuildingShapeQuery,
        // }
        //
        // let mut current_query_branch_state: Option<QueryBranchState> = None;
        // let mut current_state = State::BuildingParameterGraph;
        //
        // let mut operation_parameter = OperationParameter::<S> {
        //     explicit_input_nodes: Vec::new(),
        //     parameter_graph: AbstractGraph::<S>::new(),
        //     subst_to_node_keys: HashMap::new(),
        //     node_keys_to_subst: HashMap::new(),
        // };
        //
        // // unsure if we need these.
        // let mut abstract_graph = AbstractGraph::<S>::new();
        // let mut aid_to_node_keys: HashMap<AbstractNodeId, NodeKey> = HashMap::new();
        // let mut node_keys_to_aid: HashMap<NodeKey, AbstractNodeId> = HashMap::new();
        //
        // // build a partial UserDefinedOperation.
        // // This UserDefinedOperation is what we will use to build the partial abstract graph at that state.
        //
        // // This is a stack of instruction vectors.
        // let mut instructions_vec_stack: Vec<Vec<UDInstruction<S>>> = Vec::new();
        // // We push any new instructions onto this vector.
        // let mut current_instructions_vec: Vec<UDInstruction<S>> = Vec::new();
        //
        //
        // for instruction in instructions {
        //     let mut next_state = current_state;
        //     match (current_state, instruction) {
        //         (State::BuildingParameterGraph, BuilderInstruction::ExpectParameterNode(marker, node_abstract)) => {
        //             if operation_parameter.subst_to_node_keys.contains_key(marker) {
        //                 return Err(OperationBuilderError::ReusedSubstMarker(*marker));
        //             }
        //             let key = operation_parameter.parameter_graph.add_node(node_abstract.clone());
        //             operation_parameter.subst_to_node_keys.insert(*marker, key);
        //             operation_parameter.node_keys_to_subst.insert(key, *marker);
        //             operation_parameter.explicit_input_nodes.push(*marker);
        //         }
        //         (State::BuildingParameterGraph, BuilderInstruction::ExpectContextNode(marker, node_abstract)) => {
        //             if operation_parameter.subst_to_node_keys.contains_key(marker) {
        //                 return Err(OperationBuilderError::ReusedSubstMarker(*marker));
        //             }
        //             let key = operation_parameter.parameter_graph.add_node(node_abstract.clone());
        //             operation_parameter.subst_to_node_keys.insert(*marker, key);
        //             operation_parameter.node_keys_to_subst.insert(key, *marker);
        //         }
        //         (State::BuildingParameterGraph, BuilderInstruction::ExpectParameterEdge(source_marker, target_marker, edge_abstract)) => {
        //             let source_key = operation_parameter.subst_to_node_keys.get(source_marker)
        //                 .ok_or(OperationBuilderError::NotFoundSubstMarker(*source_marker))?;
        //             let target_key = operation_parameter.subst_to_node_keys.get(target_marker)
        //                 .ok_or(OperationBuilderError::NotFoundSubstMarker(*target_marker))?;
        //             operation_parameter.parameter_graph.add_edge(*source_key, *target_key, edge_abstract.clone());
        //         }
        //         (State::ExpectingInstruction | State::BuildingParameterGraph, BuilderInstruction::AddInstruction(instruction, args)) => {
        //             next_state = State::ExpectingInstruction;
        //
        //             match instruction {
        //                 Instruction::Builtin(builtin_op) => {
        //                     // Here we would typically apply the builtin operation to the abstract graph.
        //                     // For now, we just log it.
        //                     println!("Applying builtin operation: {:?} args: {args:?}", builtin_op);
        //
        //                     current_instructions_vec.push(UDInstruction::Builtin(builtin_op.clone(), args.clone()));
        //                 }
        //                 Instruction::FromOperationId(op_id) => {
        //                     // Here we would typically look up the operation by its ID and apply it.
        //                     // For now, we just log it.
        //                     println!("Applying operation with ID: {:?} args: {args:?}", op_id);
        //
        //                     current_instructions_vec.push(UDInstruction::Operation(op_id.clone(), args.clone()));
        //                 }
        //                 Instruction::Recurse => {
        //                     // This would typically mean we need to recurse into another operation.
        //                     // For now, we just log it.
        //                     println!("Recursing into self with args: {args:?}");
        //
        //                     // TODO: somehow denote 'self' instead of 0
        //                     current_instructions_vec.push(UDInstruction::Operation(0, args.clone()));
        //                 }
        //             }
        //         }
        //         (State::ExpectingInstruction | State::BuildingParameterGraph, BuilderInstruction::StartQuery(query, args)) => {
        //             next_state = State::BuildingQuery;
        //
        //             // Start a new query state
        //             current_query_branch_state = Some(QueryBranchState::NoBranch);
        //             instructions_vec_stack.push(current_instructions_vec);
        //             current_instructions_vec = Vec::new();
        //             // TODO: try continue here. the 'parsing' style below is easier, but this stack based version would allow easy caching.
        //         }
        //         _ => {}
        //     }
        //     current_state = next_state;
        // }

        let mut iter = builder_instructions.iter().peekable();

        let op_parameter = Self::build_operation_parameter(&mut iter)?;

        let mut builder = Self {
            _phantom_data: PhantomData,
            path: Vec::new(),
        };

        let instructions = builder.build_many_instructions(&mut iter)?;

        let mut return_nodes = HashMap::new();
        // if we are outside all queries, check for ReturnNode instructions.
        if !builder.path.iter().any(|i| matches!(i, IntermediateStatePath::StartQuery(..))) {
            // we are outside all queries
            return_nodes = Self::collect_return_node_instructions(&mut iter)?;
            // TODO: validate that we have not encountered a Recurse instruction. In recursive queries we cannot statically return.
        }

        // assert our iter is empty
        if let Some(next_instruction) = iter.peek() {
            return Err(OperationBuilderError::SuperfluousInstruction(format!(
                "{next_instruction:?}"
            )));
        }

        Ok(BuilderResult {
            operation_parameter: op_parameter,
            instructions,
            state_path: builder.path,
            return_nodes,
        })
    }

    fn build_many_instructions(
        &mut self,
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<
        Vec<(
            Option<AbstractOperationResultMarker>,
            IntermediateInstruction<S>,
        )>,
        OperationBuilderError,
    > {
        let mut instructions = Vec::new();

        while let Some(instr) = iter.peek() {
            // break on control flow instructions and don't consume
            if matches!(
                instr,
                BuilderInstruction::EndQuery
                    | BuilderInstruction::EnterTrueBranch
                    | BuilderInstruction::EnterFalseBranch
                    | BuilderInstruction::ReturnNode(..)
            ) {
                break;
            }
            instructions.push(self.build_instruction(iter)?);
        }
        Ok(instructions)
    }

    fn build_instruction(
        &mut self,
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<
        (
            Option<AbstractOperationResultMarker>,
            IntermediateInstruction<S>,
        ),
        OperationBuilderError,
    > {
        let next_instruction = iter
            .peek()
            .expect("should only be called when there is an instruction");
        match next_instruction {
            BuilderInstruction::AddNamedOperation(_, oplike, args)
            | BuilderInstruction::AddOperation(oplike, args) => {
                let name =
                    if let BuilderInstruction::AddNamedOperation(name, _, _) = next_instruction {
                        Some(name.clone())
                    } else {
                        None
                    };
                iter.next();

                let oplike = match oplike {
                    BuilderOpLike::Builtin(builtin_op) => {
                        IntermediateOpLike::Builtin(builtin_op.clone(), args.clone())
                    }
                    BuilderOpLike::FromOperationId(op_id) => {
                        IntermediateOpLike::Operation(op_id.clone(), args.clone())
                    }
                    BuilderOpLike::Recurse => IntermediateOpLike::Recurse(args.clone()),
                };
                self.path.push(IntermediateStatePath::Advance);
                Ok((name, IntermediateInstruction::OpLike(oplike)))
            }
            BuilderInstruction::StartQuery(query, args) => {
                iter.next();
                // Start a new query state
                self.path.push(IntermediateStatePath::StartQuery(None));
                let query_instructions = self.build_query_instruction(iter)?;
                Ok((
                    None,
                    IntermediateInstruction::BuiltinQuery(
                        query.clone(),
                        args.clone(),
                        query_instructions,
                    ),
                ))
            }
            BuilderInstruction::StartShapeQuery(op_marker) => {
                iter.next();
                self.path
                    .push(IntermediateStatePath::StartQuery(Some(format!(
                        "{op_marker:?}"
                    ))));
                // Start a new shape query state
                let (gsq_instructions, branch_instructions) =
                    self.build_shape_query(iter, op_marker.clone())?;
                // Ok((Some(*op_marker), UDInstruction::ShapeQuery()))
                Ok((
                    Some(op_marker.clone()), // NOTE: this marker is needed as well for the _concrete_ execution
                    IntermediateInstruction::GraphShapeQuery(
                        op_marker.clone(),
                        gsq_instructions,
                        branch_instructions,
                    ),
                ))
            }
            _ => Err(OperationBuilderError::ExpectedOperationOrQuery),
        }
    }

    fn build_shape_query(
        &mut self,
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
        operation_marker: AbstractOperationResultMarker,
    ) -> Result<
        (
            Vec<GraphShapeQueryInstruction<S>>,
            IntermediateQueryInstructions<S>,
        ),
        OperationBuilderError,
    > {
        // we just consumed a StartShapeQuery instruction.

        // let mut gsq = GraphShapeQuery {
        //     parameter: OperationParameter {
        //         explicit_input_nodes: vec![],
        //         parameter_graph: Graph::new(),
        //         subst_to_node_keys: Default::default(),
        //         node_keys_to_subst: Default::default(),
        //     },
        //     expected_graph: Graph::new(),
        //     node_keys_to_shape_idents: Default::default(),
        //     shape_idents_to_node_keys: Default::default(),
        // };

        let mut gsq_instructions = vec![];

        let mut true_branch_instructions = None;
        let mut false_branch_instructions = None;

        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::EnterTrueBranch => {
                    iter.next();
                    if true_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(true));
                    }
                    // we are entering a true branch, so we remove the false branch from the path
                    self.remove_until_branch(false);
                    self.path.push(IntermediateStatePath::EnterTrue);
                    true_branch_instructions = Some(self.build_many_instructions(iter)?);
                }
                BuilderInstruction::EnterFalseBranch => {
                    iter.next();
                    if false_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(false));
                    }
                    // we are entering a false branch, so we remove the true branch from the path
                    self.remove_until_branch(true);
                    self.path.push(IntermediateStatePath::EnterFalse);
                    false_branch_instructions = Some(self.build_many_instructions(iter)?);
                }
                BuilderInstruction::EndQuery => {
                    iter.next();
                    // we are ending the query, so we remove the current query state from the path
                    self.remove_until_query_start();
                    self.path.push(IntermediateStatePath::SkipQuery);
                    break;
                }
                BuilderInstruction::ExpectShapeNode(marker, abstract_value) => {
                    // TODO: do we want to assert that we haven't entered any branches yet? probably...
                    iter.next();
                    // let shape_node_ident = marker.0.into();
                    // if gsq.shape_idents_to_node_keys.contains_key(&shape_node_ident) {
                    //     return Err(OperationBuilderError::ReusedShapeIdent(shape_node_ident));
                    // }
                    // let key = gsq.expected_graph.add_node(abstract_value.clone());
                    // gsq.node_keys_to_shape_idents.insert(key, shape_node_ident);
                    // gsq.shape_idents_to_node_keys.insert(shape_node_ident, key);

                    gsq_instructions.push(GraphShapeQueryInstruction::ExpectShapeNode(
                        marker.clone(),
                        abstract_value.clone(),
                    ));
                }
                BuilderInstruction::ExpectShapeEdge(source, target, abstract_value) => {
                    iter.next();
                    // TODO: we need a current view of the abstract graph (or, well, AID mappings) so that we can build the GraphShapeQuery here which requires
                    //  an actual `Graph`.

                    // instead, switch to deferred approach by just passing along the instructions
                    gsq_instructions.push(GraphShapeQueryInstruction::ExpectShapeEdge(
                        source.clone(),
                        target.clone(),
                        abstract_value.clone(),
                    ));
                }
                _ => {
                    return Err(OperationBuilderError::InvalidInQuery);
                }
            }
        }

        Ok((
            gsq_instructions,
            IntermediateQueryInstructions {
                true_branch: true_branch_instructions.unwrap_or_default(),
                false_branch: false_branch_instructions.unwrap_or_default(),
            },
        ))
    }

    fn build_query_instruction(
        &mut self,
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<IntermediateQueryInstructions<S>, OperationBuilderError> {
        // we just consumed a StartQuery instruction.
        let mut true_branch_instructions = None;
        let mut false_branch_instructions = None;
        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::EnterTrueBranch => {
                    iter.next();
                    if true_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(true));
                    }
                    // we are entering a true branch, so we remove the false branch from the path
                    self.remove_until_branch(false);
                    self.path.push(IntermediateStatePath::EnterTrue);
                    true_branch_instructions = Some(self.build_many_instructions(iter)?);
                }
                BuilderInstruction::EnterFalseBranch => {
                    iter.next();
                    if false_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(false));
                    }
                    // we are entering a false branch, so we remove the true branch from the path
                    self.remove_until_branch(true);
                    self.path.push(IntermediateStatePath::EnterFalse);
                    false_branch_instructions = Some(self.build_many_instructions(iter)?);
                }
                BuilderInstruction::EndQuery => {
                    iter.next();
                    // we are ending the query, so we remove the current query state from the path
                    self.remove_until_query_start();
                    self.path.push(IntermediateStatePath::SkipQuery);
                    break;
                }
                _ => {
                    return Err(OperationBuilderError::InvalidInQuery);
                }
            }
        }
        let true_branch = true_branch_instructions.unwrap_or_default();
        let false_branch = false_branch_instructions.unwrap_or_default();
        Ok(IntermediateQueryInstructions {
            true_branch,
            false_branch,
        })
    }

    fn build_operation_parameter(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<OperationParameter<S>, OperationBuilderError> {
        let mut operation_parameter = OperationParameter::<S> {
            explicit_input_nodes: Vec::new(),
            parameter_graph: AbstractGraph::<S>::new(),
            subst_to_node_keys: HashMap::new(),
            node_keys_to_subst: HashMap::new(),
        };

        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::ExpectParameterNode(marker, node_abstract) => {
                    iter.next();
                    if operation_parameter.subst_to_node_keys.contains_key(marker) {
                        return Err(OperationBuilderError::ReusedSubstMarker(marker.clone()));
                    }
                    let key = operation_parameter
                        .parameter_graph
                        .add_node(node_abstract.clone());
                    operation_parameter.subst_to_node_keys.insert(marker.clone(), key);
                    operation_parameter.node_keys_to_subst.insert(key, marker.clone());
                    operation_parameter.explicit_input_nodes.push(marker.clone());
                }
                BuilderInstruction::ExpectContextNode(marker, node_abstract) => {
                    iter.next();
                    if operation_parameter.subst_to_node_keys.contains_key(marker) {
                        return Err(OperationBuilderError::ReusedSubstMarker(marker.clone()));
                    }
                    let key = operation_parameter
                        .parameter_graph
                        .add_node(node_abstract.clone());
                    operation_parameter.subst_to_node_keys.insert(marker.clone(), key);
                    operation_parameter.node_keys_to_subst.insert(key, marker.clone());
                }
                BuilderInstruction::ExpectParameterEdge(
                    source_marker,
                    target_marker,
                    edge_abstract,
                ) => {
                    iter.next();
                    let source_key = operation_parameter
                        .subst_to_node_keys
                        .get(source_marker)
                        .ok_or(OperationBuilderError::NotFoundSubstMarker(source_marker.clone()))?;
                    let target_key = operation_parameter
                        .subst_to_node_keys
                        .get(target_marker)
                        .ok_or(OperationBuilderError::NotFoundSubstMarker(target_marker.clone()))?;
                    operation_parameter.parameter_graph.add_edge(
                        *source_key,
                        *target_key,
                        edge_abstract.clone(),
                    );
                }
                _ => {
                    break;
                }
            }
        }

        Ok(operation_parameter)
    }

    fn collect_return_node_instructions(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>, OperationBuilderError> {
        let mut return_nodes = HashMap::new();
        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::ReturnNode(aid, output_marker, node) => {
                    iter.next();
                    if return_nodes.contains_key(aid) {
                        return Err(OperationBuilderError::AlreadySelectedReturnNode(aid.clone()));
                    }
                    return_nodes.insert(aid.clone(), (output_marker.clone(), node.clone()));
                }
                _ => break,
            }
        }
        Ok(return_nodes)
    }

    fn remove_until_branch(&mut self, branch: bool) {
        // need to check that a branch is actually there in the region until the last skip_query
        let branch_to_find = if branch {
            IntermediateStatePath::EnterTrue
        } else {
            IntermediateStatePath::EnterFalse
        };

        let mut found = false;
        for last in self.path.iter().rev() {
            if last == &branch_to_find {
                // we found the branch, so we can stop
                found = true;
            }
            if matches!(last, IntermediateStatePath::StartQuery(..)) {
                // we reached the start of the query, so we cannot find the branch
                break;
            }
        }
        if !found {
            // we did not find the branch, so we cannot remove it
            return;
        }

        while let Some(last) = self.path.pop() {
            if (branch && last == IntermediateStatePath::EnterTrue)
                || (!branch && last == IntermediateStatePath::EnterFalse)
            {
                break;
            }
        }
    }

    fn remove_until_query_start(&mut self) {
        while let Some(last) = self.path.pop() {
            if matches!(last, IntermediateStatePath::StartQuery(..)) {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IntermediateStatePath {
    // advance by one regular instruction. don't go in.
    Advance,
    EnterTrue,
    EnterFalse,
    // TODO: is this the same as Advance?
    SkipQuery,
    StartQuery(Option<String>), // the query name, if any
}

// TODO: here make the intermediate state interpreter have points at which it knows the state
//  eg at every query branch point... hmm maybe it should be passed an argument of _where_ we want to know the state?
//  some path like entering the true/false branch, leaving a query...

/*
What kind of information do we want to give the user when they ask for the current state of the operation?

1. Current abstract graph
 * Realistically, this should be formatted by ignoring NodeKeys and only showing AbstractNodeId
 ==> We need a NodeKey => AbstractNodeId mapping
2. Available AbstractNodeIds and their abstract values
 * We can do this by mapping AbstractNodeId to NodeKey and then looking up the node in the graph.
 ==> We need an AbstractNodeId => NodeKey mapping
3. Current query state
 * How should this be represented?
 * Some path? Can we "visualize" queries?
 * then we could have paths like: "GtZero on AID_1 true branch, ShapeQuery Y (Shape queries will be difficult to visualize)
   on AID_2 and AID_3 false branch, EqValues on AID_3 and AID_4 no branch yet"


How do we store intermediate representation?
To do this memory-efficiently, some incremental representation would be nice. Like "this instruction added this AID".
But, for time reasons, let's just store a copy of the entire state from above after each instruction.
*/

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryPath {
    Query(String),
    TrueBranch,
    FalseBranch,
}

// TODO: Store more information like:
//  - Are we still building the parameter graph?
//  - If we are inside a query, which branches have we not entered yet?
//  - Are we making a shape/non-shape query?

pub struct IntermediateState<S: SemanticsClone> {
    pub graph: AbstractGraph<S>,
    pub node_keys_to_aid: BiMap<NodeKey, AbstractNodeId>,
    // TODO: Somehow remove AIDs from this set if they're completely overwritten by something non-shape-query.
    //  could be done by, whenever adding a new node, unconditionally removing the AID from this set as long as we're not in a shape query.
    //  since we have a different state at that point, it would get merged correctly (assuming we take the union).
    pub may_originate_from_shape_query: HashSet<AbstractNodeId>,
    // TODO: keep track of may_deleted_nodes etc.

    // TODO: make query path
    pub query_path: Vec<QueryPath>,
}

// TODO: unfortunately, we cannot derive Clone, since it implies a `S: Clone` bound.
//  - in theory, we could add that bound, since a Semantics as a value does not really store much. So clone should be fine.
impl<S: SemanticsClone> Clone for IntermediateState<S> {
    fn clone(&self) -> Self {
        IntermediateState {
            graph: self.graph.clone(),
            node_keys_to_aid: self.node_keys_to_aid.clone(),
            may_originate_from_shape_query: self.may_originate_from_shape_query.clone(),
            query_path: self.query_path.clone(),
        }
    }
}

impl<S: SemanticsClone> IntermediateState<S> {
    pub fn node_av_of_aid(&self, aid: &AbstractNodeId) -> Option<&S::NodeAbstract> {
        let node_key = self.node_keys_to_aid.get_right(aid)?;
        self.graph.get_node_attr(*node_key)
    }

    pub fn edge_av_of_aid(
        &self,
        source: &AbstractNodeId,
        target: &AbstractNodeId,
    ) -> Option<&S::EdgeAbstract> {
        let source_key = self.node_keys_to_aid.get_right(source)?;
        let target_key = self.node_keys_to_aid.get_right(target)?;
        self.graph.get_edge_attr((*source_key, *target_key))
    }
}

impl<S: SemanticsClone<NodeAbstract: Debug, EdgeAbstract: Debug>> IntermediateState<S> {
    pub fn dot_with_aid(&self) -> String {
        struct PrettyAid<'a>(&'a AbstractNodeId);

        impl Debug for PrettyAid<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.0 {
                    AbstractNodeId::ParameterMarker(subst) => write!(f, "P({})", subst.0),
                    AbstractNodeId::DynamicOutputMarker(marker, node_marker) => {
                        let op_marker = match marker {
                            AbstractOperationResultMarker::Custom(c) => c,
                            AbstractOperationResultMarker::Implicit(num) => "<unnamed>",
                        };
                        write!(f, "O({}, {})", op_marker, node_marker.0)
                    }
                }
            }
        }

        // TODO: handle edge order...

        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|g, (src, target, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("label = \"{dbg_attr_replaced}\"")
                },
                &|g, (node, _)| {
                    let aid = self
                        .node_keys_to_aid
                        .get_left(&node)
                        .expect("NodeKey not found in node_keys_to_aid");
                    let aid = PrettyAid(aid);
                    let aid = format!("{aid:?}");
                    let aid_replaced = aid.escape_debug();
                    let av = self
                        .graph
                        .get_node_attr(node)
                        .expect("NodeKey not found in graph");
                    let dbg_attr_format = format!("{:?}", av);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();

                    format!("label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                }
            )
        )
    }
}

enum InterpretedInstruction<S: SemanticsClone> {
    OpLike,
    Query(InterpretedQueryInstructions<S>),
}

struct InterpretedQueryInstructions<S: SemanticsClone> {
    initial_state_true_branch: IntermediateState<S>,
    initial_state_false_branch: IntermediateState<S>,
    true_branch: InterpretedInstructions<S>,
    false_branch: InterpretedInstructions<S>,
}

struct InterpretedInstructionWithState<S: SemanticsClone> {
    instruction: InterpretedInstruction<S>,
    state_after: IntermediateState<S>,
}

struct IntermediateInterpreter<'a, S: SemanticsClone> {
    self_op_id: OperationId,
    op_ctx: &'a OperationContext<S>,
    op_param: OperationParameter<S>,
    initial_state: IntermediateState<S>,
    current_state: IntermediateState<S>,
    /// A counter to generate unique operation result markers.
    counter: u64,
}

type UDInstructionsWithMarker<S> = Vec<(Option<AbstractOperationResultMarker>, UDInstruction<S>)>;

type InterpretedInstructions<S> = Vec<(
    Option<AbstractOperationResultMarker>,
    InterpretedInstructionWithState<S>,
)>;

impl<'a, S: SemanticsClone> IntermediateInterpreter<'a, S> {
    fn new_for_self_op_id(
        self_op_id: OperationId,
        op_param: OperationParameter<S>,
        op_ctx: &'a OperationContext<S>,
    ) -> Self {
        let initial_graph = op_param.parameter_graph.clone();

        let mut initial_mapping = BiMap::new();

        for (key, subst) in op_param.node_keys_to_subst.iter() {
            let aid = AbstractNodeId::ParameterMarker(subst.clone());
            initial_mapping.insert(*key, aid);
        }

        let initial_state = IntermediateState {
            graph: initial_graph,
            node_keys_to_aid: initial_mapping,
            may_originate_from_shape_query: HashSet::new(),
            query_path: Vec::new(),
        };

        let current_state = initial_state.clone();

        let interpreter = IntermediateInterpreter {
            self_op_id,
            op_ctx,
            op_param,
            initial_state,
            current_state,
            counter: 0,
        };

        interpreter
    }

    fn create_user_defined_operation(
        &mut self,
        intermediate_instructions: Vec<(
            Option<AbstractOperationResultMarker>,
            IntermediateInstruction<S>,
        )>,
        return_nodes: HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
    ) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        let (ud_instructions, _interp_instructions) =
            self.interpret_instructions(intermediate_instructions)?;

        // need to determine validity of return_nodes
        let mut output = AbstractUserDefinedOperationOutput::new();
        for (aid, (output_marker, node_abstract)) in return_nodes {
            let Some(key) = self.current_state.node_keys_to_aid.get_right(&aid) else {
                return Err(OperationBuilderError::NotFoundReturnNode(aid));
            };
            // make sure type we're deciding to return is a valid supertype
            let inferred_av = self
                .current_state
                .node_av_of_aid(&aid)
                .ok_or(OperationBuilderError::NotFoundReturnNode(aid.clone()))?;
            if !S::NodeMatcher::matches(inferred_av, &node_abstract) {
                return Err(OperationBuilderError::InvalidReturnNodeType(
                    aid,
                ));
            }
            if self.current_state.may_originate_from_shape_query.contains(&aid) {
                return Err(OperationBuilderError::ReturnNodeMayOriginateFromShapeQuery(aid));
            }
            output.new_nodes.insert(aid, (output_marker, node_abstract));
        }

        Ok(UserDefinedOperation {
            parameter: self.op_param.clone(),
            instructions: ud_instructions,
            output_changes: output,
        })
    }

    fn interpret_instructions(
        &mut self,
        intermediate_instructions: Vec<(
            Option<AbstractOperationResultMarker>,
            IntermediateInstruction<S>,
        )>,
    ) -> Result<(UDInstructionsWithMarker<S>, InterpretedInstructions<S>), OperationBuilderError>
    {
        let mut ud_instructions = Vec::new();
        let mut interpreted_instructions = Vec::new();
        for (marker, instruction) in intermediate_instructions {
            let (ud_instruction, interpreted_instruction) =
                self.interpret_single_instruction(marker.clone(), instruction)?;
            ud_instructions.push((marker.clone(), ud_instruction));
            interpreted_instructions.push((
                marker,
                InterpretedInstructionWithState {
                    instruction: interpreted_instruction,
                    state_after: self.current_state.clone(),
                },
            ));
        }
        Ok((ud_instructions, interpreted_instructions))
    }

    fn interpret_single_instruction(
        &mut self,
        marker: Option<AbstractOperationResultMarker>,
        instruction: IntermediateInstruction<S>,
    ) -> Result<(UDInstruction<S>, InterpretedInstruction<S>), OperationBuilderError> {
        match instruction {
            IntermediateInstruction::OpLike(oplike) => Ok((
                self.interpret_op_like(marker, oplike)?,
                InterpretedInstruction::OpLike,
            )),
            IntermediateInstruction::BuiltinQuery(query, args, query_instructions) => {
                self.interpret_builtin_query(query, args, query_instructions)
            }
            IntermediateInstruction::GraphShapeQuery(
                op_marker,
                gsq_instructions,
                query_instructions,
            ) => self.interpret_graph_shape_query(op_marker, gsq_instructions, query_instructions),
        }
    }

    fn interpret_op_like(
        &mut self,
        marker: Option<AbstractOperationResultMarker>,
        oplike: IntermediateOpLike<S>,
    ) -> Result<UDInstruction<S>, OperationBuilderError> {
        match oplike {
            IntermediateOpLike::Builtin(op, args) => {
                let param = op.parameter();
                let (subst, abstract_arg) = self.get_current_substitution(&param, args)?;

                // now apply op and store result
                let operation_output = {
                    let mut gws = GraphWithSubstitution::new(&mut self.current_state.graph, &subst);
                    op.apply_abstract(&mut gws)
                };
                // go over new nodes
                let marker =
                    marker.unwrap_or_else(|| self.get_new_unnamed_abstract_operation_marker());
                for (node_marker, node_key) in operation_output.new_nodes {
                    let aid = AbstractNodeId::DynamicOutputMarker(marker.clone(), node_marker);
                    self.current_state.node_keys_to_aid.insert(node_key, aid);
                }
                for node_key in operation_output.removed_nodes {
                    // remove the node from the mapping
                    self.current_state.node_keys_to_aid.remove_left(&node_key);
                }

                Ok(UDInstruction::Builtin(op, abstract_arg))
            }
            IntermediateOpLike::Operation(id, args) => {
                let op = self
                    .op_ctx
                    .get(id)
                    .ok_or(OperationBuilderError::NotFoundOperationId(id))?;
                let param = op.parameter();
                let (subst, abstract_arg) = self.get_current_substitution(&param, args)?;

                let operation_output = {
                    let mut gws = GraphWithSubstitution::new(&mut self.current_state.graph, &subst);
                    op.apply_abstract(self.op_ctx, &mut gws)
                };
                // go over new nodes
                let marker =
                    marker.unwrap_or_else(|| self.get_new_unnamed_abstract_operation_marker());
                let operation_output = operation_output
                    .map_err(|e| OperationBuilderError::AbstractApplyOperationError(id, e))?;
                for (node_marker, node_key) in operation_output.new_nodes {
                    let aid = AbstractNodeId::DynamicOutputMarker(marker.clone(), node_marker);
                    self.current_state.node_keys_to_aid.insert(node_key, aid);
                }
                for node_key in operation_output.removed_nodes {
                    // remove the node from the mapping
                    self.current_state.node_keys_to_aid.remove_left(&node_key);
                }
                Ok(UDInstruction::Operation(id, abstract_arg))
            }
            IntermediateOpLike::Recurse(args) => {
                // TODO: recursion is actually tricky. because at this point we have not finished interpreting the current operation yet.
                //  So how are we supposed to know the abstract changes?

                // TODO: use approach from `problems-testcases.md`

                let (subst, abstract_arg) = self.get_current_substitution(&self.op_param, args)?;
                // apply the operation to the current graph
                // TODO: apply op
                // this should probably use some pre-defined (at the beginning) abstract changes to the graph.
                Ok(UDInstruction::Operation(self.self_op_id, abstract_arg))
            }
        }
    }

    fn interpret_builtin_query(
        &mut self,
        query: S::BuiltinQuery,
        args: Vec<AbstractNodeId>,
        query_instructions: IntermediateQueryInstructions<S>,
    ) -> Result<(UDInstruction<S>, InterpretedInstruction<S>), OperationBuilderError> {
        let param = query.parameter();
        let (subst, arg) = self.get_current_substitution(&param, args)?;

        // apply the query to the current graph
        query.apply_abstract(&mut GraphWithSubstitution::new(&mut self.current_state.graph, &subst));

        // TODO: is this right? do we want to snapshot the state _after_ the query?
        //  I think so, because right now (weirdly enough) the query can modify. and the modifications
        //  are applied to both branches and what comes after.

        let state_before = self.current_state.clone();
        let false_branch_state = self.current_state.clone();

        let initial_true_branch_state = self.current_state.clone();
        let initial_false_branch_state = self.current_state.clone();

        // interpret the instructions in the true and false branches
        let (ud_true_branch, interp_true_branch) =
            self.interpret_instructions(query_instructions.true_branch)?;
        let after_true_branch_state = mem::replace(&mut self.current_state, false_branch_state);
        let (ud_false_branch, interp_false_branch) =
            self.interpret_instructions(query_instructions.false_branch)?;
        let after_false_branch_state = mem::replace(&mut self.current_state, state_before);

        // TODO: update current state etc...

        // TODO: reconcile states of both true and false branch!
        // TODO: reconciliation should probably be done via having the same AID for the same node in both branches.
        //  all other ones will be ignored.
        //  ==> we must manually change the abstract graph ourselves here!
        //  ==> we must reconcile into self.current_state

        let merged_state = merge_states(false, &after_true_branch_state, &after_false_branch_state);
        self.current_state = merged_state;

        let ud_instr = UDInstruction::BuiltinQuery(
            query,
            arg,
            QueryInstructions {
                taken: ud_true_branch,
                not_taken: ud_false_branch,
            },
        );

        let interp_instruction = InterpretedInstruction::Query(InterpretedQueryInstructions {
            initial_state_true_branch: initial_true_branch_state,
            initial_state_false_branch: initial_false_branch_state,
            true_branch: interp_true_branch,
            false_branch: interp_false_branch,
        });

        Ok((ud_instr, interp_instruction))
    }

    fn interpret_graph_shape_query(
        &mut self,
        gsq_op_marker: AbstractOperationResultMarker,
        gsq_instructions: Vec<GraphShapeQueryInstruction<S>>,
        query_instructions: IntermediateQueryInstructions<S>,
    ) -> Result<(UDInstruction<S>, InterpretedInstruction<S>), OperationBuilderError> {
        let mut state_before = self.current_state.clone();

        // preparation for false branch
        let false_branch_state = self.current_state.clone();
        let initial_false_branch_state = false_branch_state.clone();

        // first pass: collect the initial graph (the parameter)
        let mut param = OperationParameter::<S> {
            explicit_input_nodes: vec![],
            parameter_graph: AbstractGraph::<S>::new(),
            subst_to_node_keys: HashMap::new(),
            node_keys_to_subst: HashMap::new(),
        };

        let mut abstract_args = Vec::new();

        let mut arg_aid_to_param_subst: BiMap<AbstractNodeId, SubstMarker> = BiMap::new();
        let mut arg_aid_to_node_keys: BiMap<AbstractNodeId, NodeKey> = BiMap::new();

        /// Collects the AID and adds it to all relevant mappings.
        let mut collect_aid = |aid: AbstractNodeId| -> Result<(), OperationBuilderError> {
            if arg_aid_to_param_subst.contains_left(&aid) {
                // we already processed this
                return Ok(());
            }
            let subst_marker = SubstMarker::from((param.explicit_input_nodes.len() as u32).to_string());
            let key = self
                .current_state
                .node_keys_to_aid
                .get_right(&aid)
                .cloned()
                .ok_or(OperationBuilderError::NotFoundAid(aid.clone()))?;
            let abstract_value = self
                .current_state
                .graph
                .get_node_attr(key)
                .expect(
                    "internal error: node key should be in state graph since it is in the mapping",
                )
                .clone();
            let param_key = param.parameter_graph.add_node(abstract_value);
            param.subst_to_node_keys.insert(subst_marker.clone(), param_key);
            param.node_keys_to_subst.insert(param_key, subst_marker.clone());
            param.explicit_input_nodes.push(subst_marker.clone());
            abstract_args.push(aid.clone());
            arg_aid_to_param_subst.insert(aid.clone(), subst_marker.clone());
            arg_aid_to_node_keys.insert(aid.clone(), key);
            Ok(())
        };

        /// Collects the AID if it is part of the pre-existing graph.
        let mut collect_non_shape_ident =
            |aid: &AbstractNodeId| -> Result<(), OperationBuilderError> {
                match aid {
                    AbstractNodeId::ParameterMarker(_) => {
                        // we need this.
                        collect_aid(aid.clone())?;
                    }
                    AbstractNodeId::DynamicOutputMarker(orm, node_marker) => {
                        // we need this, but only if it is not from the current graph shape query.
                        if orm != &gsq_op_marker {
                            collect_aid(aid.clone())?;
                        }
                    }
                }
                Ok(())
            };

        for instruction in &gsq_instructions {
            match instruction {
                GraphShapeQueryInstruction::ExpectShapeNode(_, _) => {
                    // Skip. this does not affect the initial graph.
                }
                GraphShapeQueryInstruction::ExpectShapeEdge(src, target, _) => {
                    // we need both src and target to be in the initial graph, assuming they dont come from `gsq_op_marker`
                    collect_non_shape_ident(src)?;
                    collect_non_shape_ident(target)?;
                }
            }
        }

        // second pass:
        // modify to have the expected graph as well as shape ident mappings.
        // simultaneously, also modify the *current state graph* to prepare it for the true branch.
        // make a copy before that though, for the false branch.

        let mut expected_graph = param.parameter_graph.clone();
        let mut node_keys_to_shape_idents: BiMap<NodeKey, ShapeNodeIdentifier> = BiMap::new();

        // let aid_to_node_key = |aid| -> Result<NodeKey, OperationBuilderError> {
        //     arg_aid_to_node_keys.get_left(&aid)
        //         .cloned()
        //         .or_else(|| {
        //             if let AbstractNodeId::DynamicOutputMarker(orm, node_marker) = aid {
        //                 if orm == gsq_op_marker {
        //                     // this is a new node from the graph shape query.
        //                     let sni: ShapeNodeIdentifier = node_marker.0.into();
        //                     node_keys_to_shape_idents.get_right(&sni).copied()
        //                 } else {
        //                     None
        //                 }
        //             } else {
        //                 None
        //             }
        //         })
        //         .ok_or(OperationBuilderError::NotFoundAid(aid))
        // };

        // TODO: ugly. fix. needed because the above closure approach does not work due to borrowing issues.
        macro_rules! aid_to_node_key_hack {
            ($aid:expr) => {
                arg_aid_to_node_keys
                    .get_left(&$aid)
                    .cloned()
                    .or_else(|| {
                        if let AbstractNodeId::DynamicOutputMarker(orm, node_marker) = $aid {
                            if orm == gsq_op_marker {
                                // this is a new node from the graph shape query.
                                let sni: ShapeNodeIdentifier = node_marker.0.into();
                                node_keys_to_shape_idents.get_right(&sni).copied()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .ok_or(OperationBuilderError::NotFoundAid($aid))
            };
        }

        for instruction in gsq_instructions {
            match instruction {
                GraphShapeQueryInstruction::ExpectShapeNode(marker, av) => {
                    let key = expected_graph.add_node(av.clone());
                    let shape_node_ident = marker.0.clone().into();
                    // TODO: insert is panicking and therefore we should return an error instead here.
                    // TODO: make bimap::insert fallible? return a must_use Option<()>?
                    node_keys_to_shape_idents.insert(key, shape_node_ident);

                    // now update the state for the true branch.
                    let state_key = self.current_state.graph.add_node(av);
                    let aid = AbstractNodeId::DynamicOutputMarker(gsq_op_marker.clone(), marker.clone());
                    self.current_state
                        .node_keys_to_aid
                        .insert(state_key, aid.clone());
                    self.current_state.may_originate_from_shape_query.insert(aid.clone());
                }
                GraphShapeQueryInstruction::ExpectShapeEdge(src, target, av) => {
                    let src_key = aid_to_node_key_hack!(src.clone())?;
                    let target_key = aid_to_node_key_hack!(target.clone())?;
                    expected_graph.add_edge(src_key, target_key, av.clone());

                    // now update the state for the true branch.
                    let state_src_key = *self
                        .current_state
                        .node_keys_to_aid
                        .get_right(&src)
                        .ok_or(OperationBuilderError::NotFoundAid(src))?;
                    let state_target_key = *self
                        .current_state
                        .node_keys_to_aid
                        .get_right(&target)
                        .ok_or(OperationBuilderError::NotFoundAid(target.clone()))?;
                    self.current_state
                        .graph
                        .add_edge(state_src_key, state_target_key, av);
                }
            }
        }

        let (node_keys_to_shape_idents, shape_idents_to_node_keys) =
            node_keys_to_shape_idents.into_inner();
        let gsq = GraphShapeQuery {
            parameter: param,
            expected_graph,
            node_keys_to_shape_idents,
            shape_idents_to_node_keys,
        };

        // TODO: need to validate GSQ somewhere.
        //  Most importantly, that there are no free floating shape nodes.

        let mut initial_true_branch_state = self.current_state.clone();

        let (ud_true_branch, interp_true_branch) =
            self.interpret_instructions(query_instructions.true_branch)?;
        // switch back to the other state
        let after_true_branch_state = mem::replace(&mut self.current_state, false_branch_state);
        let (ud_false_branch, interp_false_branch) =
            self.interpret_instructions(query_instructions.false_branch)?;
        let after_false_branch_state = mem::replace(&mut self.current_state, state_before);
        // TODO: reconcile the states of both branches. same as in query.

        // current situation: self.current_state is before both branches, and we have the true and false branch states
        // available to reconcile.

        let merged_state = merge_states(true, &after_true_branch_state, &after_false_branch_state);
        self.current_state = merged_state;

        let ud_instruction = UDInstruction::ShapeQuery(
            gsq,
            abstract_args,
            QueryInstructions {
                taken: ud_true_branch,
                not_taken: ud_false_branch,
            },
        );

        let interp_instruction = InterpretedInstruction::Query(InterpretedQueryInstructions {
            initial_state_true_branch: initial_true_branch_state,
            initial_state_false_branch: initial_false_branch_state,
            true_branch: interp_true_branch,
            false_branch: interp_false_branch,
        });

        Ok((ud_instruction, interp_instruction))
    }

    fn get_current_substitution(
        &self,
        param: &OperationParameter<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(ParameterSubstitution, AbstractOperationArgument), OperationBuilderError> {
        let selected_inputs = args
            .iter()
            .map(|aid| {
                self.current_state
                    .node_keys_to_aid
                    .get_right(aid)
                    .cloned()
                    .ok_or(OperationBuilderError::NotFoundAid(aid.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let subst = get_substitution(&self.current_state.graph, &param, &selected_inputs)?;
        let subst_to_aid = subst.mapping.iter().map(|(subst, key)| {
            let aid = self.current_state.node_keys_to_aid.get_left(&key).cloned()
                .expect("node key should be in mapping, because all node keys from the abstract graph should be in the mapping. internal error");
            (subst.clone(), aid)
        }).collect();

        let abstract_arg = AbstractOperationArgument {
            selected_input_nodes: args,
            subst_to_aid,
        };

        Ok((subst, abstract_arg))
    }

    fn get_new_unnamed_abstract_operation_marker(&mut self) -> AbstractOperationResultMarker {
        let val = self.counter;
        self.counter += 1;
        AbstractOperationResultMarker::Implicit(val)
    }
}

fn get_state_for_path<S: SemanticsClone>(
    initial_state: &IntermediateState<S>,
    interpreted_instructions: &InterpretedInstructions<S>,
    path: &mut impl Iterator<Item = IntermediateStatePath>,
) -> Option<IntermediateState<S>> {
    let mut current_state = initial_state;

    for (_, instruction) in interpreted_instructions {
        match path.next() {
            None => {
                // no more path, we are done
                return Some(current_state.clone());
            }
            Some(path_element) => {
                match path_element {
                    IntermediateStatePath::Advance | IntermediateStatePath::SkipQuery => {
                        current_state = &instruction.state_after;
                    }
                    IntermediateStatePath::EnterTrue | IntermediateStatePath::EnterFalse => {
                        // this should not happen
                        panic!(
                            "internal error: unexpected path element: {:?}",
                            path_element
                        );
                    }
                    IntermediateStatePath::StartQuery(..) => {
                        if let InterpretedInstruction::Query(query_instructions) =
                            &instruction.instruction
                        {
                            // we are entering a query, so we need to check the true branch
                            // TODO: perhaps here we should have a third option .state_inside_query_view ?
                            current_state = &query_instructions.initial_state_true_branch;

                            // now we need either enter true or enter false
                            match path.next() {
                                Some(IntermediateStatePath::EnterTrue) => {
                                    // we are entering the true branch, so we need to check the true branch instructions
                                    return get_state_for_path(
                                        &current_state,
                                        &query_instructions.true_branch,
                                        path,
                                    );
                                }
                                Some(IntermediateStatePath::EnterFalse) => {
                                    // we are entering the false branch, so we need to check the false branch instructions
                                    current_state = &query_instructions.initial_state_false_branch;
                                    return get_state_for_path(
                                        &current_state,
                                        &query_instructions.false_branch,
                                        path,
                                    );
                                }
                                _ => {
                                    // we are not entering any branch, so we just return the current state
                                    return Some(current_state.clone());
                                }
                            }
                        } else {
                            // this should not happen, since we only enter queries here
                            return None;
                        }
                    }
                }
            }
        }
    }

    Some(current_state.clone())
}

fn get_query_path_for_path<S: SemanticsClone>(
    path: &mut impl Iterator<Item = IntermediateStatePath>,
) -> Vec<QueryPath> {
    let mut query_path = Vec::new();

    for pe in path {
        match pe {
            IntermediateStatePath::EnterTrue => query_path.push(QueryPath::TrueBranch),
            IntermediateStatePath::EnterFalse => query_path.push(QueryPath::FalseBranch),
            IntermediateStatePath::StartQuery(name) => {
                query_path.push(QueryPath::Query(
                    name.unwrap_or("<unnamed query>".to_string()),
                ));
            }
            _ => {}
        }
    }

    query_path
}

/// Takes two intermediate states and computes the smallest subgraph and most general abstract values
/// such that the resulting state is a sound approximation of the two states.
///
/// Nodes are only merged if they have exactly the same abstract node ID in both branches.
///
/// Also, abstract type merging is fallible, so if two nodes are incompatible with each other, they don't appear in the resulting state.
///
/// # Example:
/// 1. Initial state is `P(0)|String`
/// 2. We branch:
/// 2a. True branch ends with graph `P(0)|String -> O(c1)|String -> O(c2)|String`
/// 2b. False branch ends with graph `P(0)|String -> O(c1)|Integer -> O(c3)|Object`
/// 3. The resulting state will be `P(0)|String -> O(c1)|Object`
///
/// Note how the second added node from the true branch is not present *with the same name* in the false branch, and
/// therefore is not present in the resulting state. Same for `O(c3)` from the false branch.
/// Also note how the node that exists in both branches, `O(c1)`, is present in the resulting state with the
/// least common supertype of the two branches, which is `Object` in this case.
fn merge_states<S: SemanticsClone>(
    is_true_shape: bool,
    state_true: &IntermediateState<S>,
    state_false: &IntermediateState<S>,
) -> IntermediateState<S> {
    // TODO: handle `is_true_shape`.

    let mut new_state = IntermediateState {
        graph: Graph::new(),
        node_keys_to_aid: BiMap::new(),
        may_originate_from_shape_query: HashSet::new(),
        // TODO: should probably remove query_path from the state struct, and add it to a final returned StateWithQueryPath struct?
        query_path: Vec::new(),
    };

    let mut common_aids = HashSet::new();
    // First, collect all AIDs that are present in both states.
    for aid in state_true.node_keys_to_aid.right_values() {
        if state_false.node_keys_to_aid.contains_right(aid) {
            common_aids.insert(aid.clone());
        }
    }

    // if aid may come from a shape query in either branch, it may come from a shape query in the merged state.
    for aid in &common_aids {
        if state_true.may_originate_from_shape_query.contains(aid)
            || state_false.may_originate_from_shape_query.contains(aid)
        {
            new_state.may_originate_from_shape_query.insert(aid.clone());
        }
    }

    // Now, for each common AID, we need to merge the nodes from both states.
    for aid in common_aids {
        let key_true = *state_true
            .node_keys_to_aid
            .get_right(&aid)
            .expect("internal error: AID should be in mapping");
        let key_false = *state_false
            .node_keys_to_aid
            .get_right(&aid)
            .expect("internal error: AID should be in mapping");

        // Get the abstract values from both states.
        let av_true = state_true
            .graph
            .get_node_attr(key_true)
            .expect("internal error: Key should be in graph");
        let av_false = state_false
            .graph
            .get_node_attr(key_false)
            .expect("internal error: Key should be in graph");

        // Merge the abstract values.
        let Some(merged_av) = S::join_nodes(av_true, av_false) else {
            // If we cannot merge the abstract values, we skip this AID.
            continue;
        };

        // Add the merged node to the new state.
        let new_key = new_state.graph.add_node(merged_av);
        new_state.node_keys_to_aid.insert(new_key, aid.clone());
    }

    // Now we merge the edges.
    for (from_key_true, to_key_true, attr) in state_true.graph.graph.all_edges() {
        let from_aid = state_true
            .node_keys_to_aid
            .get_left(&from_key_true)
            .expect("internal error: from key should be in mapping");
        let to_aid = state_true
            .node_keys_to_aid
            .get_left(&to_key_true)
            .expect("internal error: to key should be in mapping");
        let Some(from_key_merged) = new_state.node_keys_to_aid.get_right(from_aid) else {
            // If the from AID has not been merged, we skip this edge.
            continue;
        };
        let Some(to_key_merged) = new_state.node_keys_to_aid.get_right(to_aid) else {
            // If the to AID has not been merged, we skip this edge.
            continue;
        };
        let av_true = state_true
            .graph
            .get_edge_attr((from_key_true, to_key_true))
            .expect("internal error: edge should be in graph");

        // Skip edges whose endpoints are not in the common AIDs.
        // because of the above new_state let else check, this should always succeed, though.
        let Some(from_key_false) = state_false.node_keys_to_aid.get_right(from_aid) else {
            continue;
        };
        let Some(to_key_false) = state_false.node_keys_to_aid.get_right(to_aid) else {
            continue;
        };

        // Check if the edge exists in the false state.
        if let Some(av_false) = state_false
            .graph
            .get_edge_attr((*from_key_false, *to_key_false))
        {
            // Try to merge the edges.
            if let Some(merged_av) = S::join_edges(av_true, av_false) {
                // If we can merge the edges, add the merged edge to the new state.
                new_state
                    .graph
                    .add_edge(*from_key_merged, *to_key_merged, merged_av);
            }
        }

        // TODO: edge orders need to be handled here.
    }

    new_state
}
