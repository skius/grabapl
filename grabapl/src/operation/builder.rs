use crate::operation::builtin::LibBuiltinOperation;
use crate::operation::marker::Marker;
use crate::operation::query::{BuiltinQuery, GraphShapeQuery, ShapeNodeIdentifier};
use crate::operation::signature::parameter::{
    AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, OperationParameter,
    ParameterSubstitution,
};
use crate::operation::signature::parameterbuilder::OperationParameterBuilder;
use crate::operation::signature::{AbstractSignatureNodeId, OperationSignature};
use crate::operation::user_defined::{
    AbstractNodeId, AbstractOperationArgument, AbstractOperationResultMarker,
    AbstractUserDefinedOperationOutput, NamedMarker, OpLikeInstruction, QueryInstructions,
    UserDefinedOperation,
};
use crate::operation::{
    BuiltinOperation, Operation, OperationError, OperationResult, get_substitution,
};
use crate::prelude::*;
use crate::semantics::{AbstractGraph, AbstractMatcher};
use crate::util::bimap::BiMap;
use crate::{NodeKey, Semantics, SubstMarker};
use error_stack::{FutureExt, Result, ResultExt, bail, report};
use petgraph::dot;
use petgraph::dot::Dot;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::iter::Peekable;
use std::marker::PhantomData;
use std::mem;
use std::slice::Iter;
use thiserror::Error;

mod programming_by_demonstration;
pub mod stack_based_builder;
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

/// An operation that can be applied abstractly
enum AbstractOperation<'a, S: Semantics> {
    Op(Operation<'a, S>),
    Partial(&'a OperationSignature<S>),
}

impl<'a, S: Semantics> AbstractOperation<'a, S> {
    fn parameter(&self) -> OperationParameter<S> {
        match self {
            AbstractOperation::Op(op) => op.parameter(),
            AbstractOperation::Partial(sig) => sig.parameter.clone(),
        }
    }

    fn apply_abstract(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> OperationResult<AbstractOperationOutput<S>> {
        match self {
            AbstractOperation::Op(op) => op.apply_abstract(op_ctx, g),
            AbstractOperation::Partial(sig) => Ok(sig.output.apply_abstract(g)),
        }
    }

    // hack to make the Inefficient operation builder still compile without too many changes.
    // TODO: delete the inefficient operation builder and this method after all TODOs have been moved
    fn from_operation(op: Operation<'a, S>) -> AbstractOperation<'a, S> {
        AbstractOperation::Op(op)
    }
}

pub enum BuilderOpLike<S: Semantics> {
    Builtin(S::BuiltinOperation),
    LibBuiltin(LibBuiltinOperation<S>),
    FromOperationId(OperationId),
    Recurse,
}

impl<S: Semantics> BuilderOpLike<S> {
    fn as_abstract_operation<'a>(
        &'a self,
        op_ctx: &'a OperationContext<S>,
        partial_self_signature: &'a OperationSignature<S>,
    ) -> Result<AbstractOperation<'a, S>, OperationBuilderError> {
        let op = match self {
            BuilderOpLike::Builtin(op) => AbstractOperation::Op(Operation::Builtin(op)),
            BuilderOpLike::LibBuiltin(op) => AbstractOperation::Op(Operation::LibBuiltin(op)),
            BuilderOpLike::FromOperationId(id) => {
                let op = op_ctx
                    .get(*id)
                    .ok_or_else(|| OperationBuilderError::NotFoundOperationId(*id))?;
                AbstractOperation::Op(op)
            }
            BuilderOpLike::Recurse => AbstractOperation::Partial(partial_self_signature),
        };
        Ok(op)
    }

    fn to_op_like_instruction(self, self_op_id: OperationId) -> OpLikeInstruction<S> {
        match self {
            BuilderOpLike::Builtin(op) => OpLikeInstruction::Builtin(op),
            BuilderOpLike::LibBuiltin(op) => OpLikeInstruction::LibBuiltin(op),
            BuilderOpLike::FromOperationId(id) => OpLikeInstruction::Operation(id),
            BuilderOpLike::Recurse => OpLikeInstruction::Operation(self_op_id),
        }
    }
}

impl<S: Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>> Clone for BuilderOpLike<S> {
    fn clone(&self) -> Self {
        match self {
            BuilderOpLike::Builtin(op) => BuilderOpLike::Builtin(op.clone()),
            BuilderOpLike::LibBuiltin(op) => BuilderOpLike::LibBuiltin(op.clone()),
            BuilderOpLike::FromOperationId(id) => BuilderOpLike::FromOperationId(*id),
            BuilderOpLike::Recurse => BuilderOpLike::Recurse,
        }
    }
}

// TODO: rename to BuilderMessage? since Instruction is already used in the user-defined operation context.
#[derive(derive_more::Debug)]
pub enum BuilderInstruction<S: Semantics> {
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
    // TODO: maybe should be renamed to ExpectNewShapeNode?
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeNodeChange({_0:?}, ???)")]
    ExpectShapeNodeChange(AbstractNodeId, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:?}, {_1:?}, ???)")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
    #[debug("SkipMarker({_0:?})")]
    SkipMarker(Marker),
    #[debug("SkipAllMarkers")]
    SkipAllMarkers,
    #[debug("AddNamedOperation({_0:?}, ???, args: {_2:?})")]
    AddNamedOperation(
        AbstractOperationResultMarker,
        BuilderOpLike<S>,
        Vec<AbstractNodeId>,
    ),
    // the same as AddNamedOperation, but without enforces the output to have a single node, and uses that node
    // to create a AbstractNodeId::named node to bind to it.
    #[debug("AddBangOperation({_0:?}, ???, args: {_2:?})")]
    AddBangOperation(NamedMarker, BuilderOpLike<S>, Vec<AbstractNodeId>),
    #[debug("AddOperation(???, args: {_1:?})")]
    AddOperation(BuilderOpLike<S>, Vec<AbstractNodeId>),
    #[debug("ReturnNode({_0:?}, {_1:?}, ???)")]
    ReturnNode(AbstractNodeId, AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ReturnEdge({_0:?}, {_1:?}, ???)")]
    ReturnEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
    #[debug("RenameNode({_0:?}, {_1:?})")]
    /// Rename a dynamic output marker.
    /// Invariants in the interpreter require that this is never a parameter node. (E.g., since we may want to return it)
    RenameNode(AbstractNodeId, NamedMarker),
    Finalize,
    /// Asserts that the current operation will return a node with the given abstract value and name.
    #[debug("SelfReturnNode({_0:?}, ???)")]
    SelfReturnNode(AbstractOutputNodeMarker, S::NodeAbstract),
}

impl<S: Semantics> BuilderInstruction<S> {
    /// Returns true if this is an instruction that is valid to break out of a body of query/operation
    /// instructions.
    fn can_break_body(&self) -> bool {
        use BuilderInstruction::*;
        match self {
            EnterTrueBranch | EnterFalseBranch | EndQuery | ReturnNode(..) | ReturnEdge(..)
            | Finalize => true,
            _ => false,
        }
    }
}

impl<S: Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>> Clone for BuilderInstruction<S> {
    fn clone(&self) -> Self {
        use BuilderInstruction::*;
        match self {
            ExpectParameterNode(marker, node) => ExpectParameterNode(marker.clone(), node.clone()),
            ExpectContextNode(marker, node) => ExpectContextNode(marker.clone(), node.clone()),
            ExpectParameterEdge(source_marker, target_marker, edge) => {
                ExpectParameterEdge(source_marker.clone(), target_marker.clone(), edge.clone())
            }
            StartQuery(query, args) => StartQuery(query.clone(), args.clone()),
            EnterTrueBranch => EnterTrueBranch,
            EnterFalseBranch => EnterFalseBranch,
            StartShapeQuery(op_marker) => StartShapeQuery(op_marker.clone()),
            EndQuery => EndQuery,
            ExpectShapeNode(marker, node) => ExpectShapeNode(marker.clone(), node.clone()),
            ExpectShapeNodeChange(aid, node) => ExpectShapeNodeChange(aid.clone(), node.clone()),
            ExpectShapeEdge(source, target, edge) => {
                ExpectShapeEdge(source.clone(), target.clone(), edge.clone())
            }
            SkipMarker(marker) => SkipMarker(marker.clone()),
            SkipAllMarkers => SkipAllMarkers,
            AddNamedOperation(name, op, args) => {
                AddNamedOperation(name.clone(), op.clone(), args.clone())
            }
            AddBangOperation(name, op, args) => {
                AddBangOperation(name.clone(), op.clone(), args.clone())
            }
            AddOperation(op, args) => AddOperation(op.clone(), args.clone()),
            ReturnNode(aid, output_marker, node) => {
                ReturnNode(aid.clone(), output_marker.clone(), node.clone())
            }
            ReturnEdge(src, dst, edge) => ReturnEdge(src.clone(), dst.clone(), edge.clone()),
            RenameNode(old_aid, new_name) => RenameNode(old_aid.clone(), new_name.clone()),
            Finalize => Finalize,
            SelfReturnNode(marker, node) => SelfReturnNode(marker.clone(), node.clone()),
        }
    }
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
    SubstitutionError(#[from] crate::operation::SubstitutionError),
    #[error("Could not apply operation due to mismatched arguments")]
    SubstitutionErrorNew,
    #[error("Could not abstractly apply operation {0} due to: {1}")]
    AbstractApplyOperationErrorWithId(OperationId, OperationError),
    #[error("Could not abstractly apply operation due to: {0}")]
    AbstractApplyOperationError(OperationError),
    #[error("Could not abstractly apply operation")]
    AbstractApplyOperationError2,
    #[error("Superfluous instruction {0}")]
    SuperfluousInstruction(String),
    #[error("Already selected to return node {0:?}")]
    AlreadySelectedReturnNode(AbstractNodeId),
    #[error("Already selected to return edge {0:?}->{1:?}")]
    AlreadySelectedReturnEdge(AbstractNodeId, AbstractNodeId),
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
    #[error("Could not find AID {0:?} for return edge source")]
    NotFoundReturnEdgeSource(AbstractNodeId),
    #[error("Could not find AID {0:?} for return edge target")]
    NotFoundReturnEdgeTarget(AbstractNodeId),
    #[error("Could not statically determine edge {0:?}->{1:?} to be available")]
    NotFoundReturnEdge(AbstractNodeId, AbstractNodeId),
    #[error("Invalid return edge type for AID {0:?}->{1:?}, must be more generic")]
    InvalidReturnEdgeType(AbstractNodeId, AbstractNodeId),
    #[error(
        "Return edge {0:?}->{1:?} may have been created by a shape query, which is not allowed"
    )]
    ReturnEdgeMayOriginateFromShapeQuery(AbstractNodeId, AbstractNodeId),
    #[error("internal error: {0}")]
    InternalError(&'static str),
    #[error("Explicitly selected input AID not found")]
    SelectedInputsNotFoundAid,
    #[error("Shape edge target node not found")]
    ShapeEdgeTargetNotFound,
    #[error("Shape edge source node not found")]
    ShapeEdgeSourceNotFound,
    #[error(
        "Cannot rename parameter node {0:?}, only new nodes from operation calls can be renamed"
    )]
    CannotRenameParameterNode(AbstractNodeId),
    #[error("Invalid parameter")]
    InvalidParameter,
    // just for testing of the explicit stack-based builder
    #[error("New builder error")]
    NewBuilderError,
}

// type alias to switch between implementations globally
// pub type OperationBuilder<'a, S> = OperationBuilderInefficient<'a, S>;
pub type OperationBuilder<'a, S> = stack_based_builder::OperationBuilder2<'a, S>;

pub struct OperationBuilderInefficient<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    self_op_id: OperationId,
    instructions: Vec<BuilderInstruction<S>>,
    // hack for recursion
    previous_user_defined_operation: RefCell<UserDefinedOperation<S>>,
}

impl<'a, S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>>
    OperationBuilderInefficient<'a, S>
{
    pub fn new(op_ctx: &'a OperationContext<S>, self_op_id: OperationId) -> Self {
        Self {
            self_op_id,
            instructions: Vec::new(),
            op_ctx,
            previous_user_defined_operation: RefCell::new(UserDefinedOperation::new_noop()),
        }
    }

    pub fn undo_last_instruction(&mut self) {
        if !self.instructions.is_empty() {
            self.instructions.pop();
        }
        self.check_instructions()
            .expect("internal error: a prefix slice of instructions should always be valid");
    }

    pub fn rename_node(
        &mut self,
        old_aid: AbstractNodeId,
        new_name: impl Into<NamedMarker>,
    ) -> Result<(), OperationBuilderError> {
        let new_name = new_name.into();
        self.instructions
            .push(BuilderInstruction::RenameNode(old_aid, new_name));
        self.check_instructions_or_rollback()
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
    // TODO: Shape queries cannot shape-test for abstract values of existing nodes yet!
    // TODO: Also add test for existing edges between existing nodes.
    pub fn start_shape_query(
        &mut self,
        op_marker: impl Into<AbstractOperationResultMarker>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions
            .push(BuilderInstruction::StartShapeQuery(op_marker.into()));
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

    pub fn expect_shape_node_change(
        &mut self,
        aid: AbstractNodeId,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions
            .push(BuilderInstruction::ExpectShapeNodeChange(aid, node));
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

    pub fn add_bang_operation(
        &mut self,
        name: impl Into<NamedMarker>,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // TODO
        self.instructions
            .push(BuilderInstruction::AddBangOperation(name.into(), op, args));
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
        self.check_instructions_or_rollback()?;
        Ok(())
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
            bail!(OperationBuilderError::CannotReturnParameter(aid));
        }
        self.instructions
            .push(BuilderInstruction::ReturnNode(aid, output_marker, node));
        self.check_instructions_or_rollback()
    }

    /// Indicate that an edge should be marked in the output with the given abstract value.
    ///
    /// Note that the edge must be a supertype of the edge's statically determined type.
    /// Also, the edge must be visible in the end context of the operation, and must never have
    /// been statically determined by a shape query.
    ///
    /// Further, new edges may only be returned if both endpoints of the edge are either parameter
    /// nodes or new nodes also returned by the operation.
    ///
    /// These instructions must be the very last instructions in the operation builder.
    pub fn return_edge(
        &mut self,
        src: AbstractNodeId,
        dst: AbstractNodeId,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO: validate that the edge did not already exist in the param graph anyway.
        self.instructions
            .push(BuilderInstruction::ReturnEdge(src, dst, edge));
        self.check_instructions_or_rollback()
    }

    // TODO: This should run further post processing checks.
    //  Stuff like Context nodes must be connected, etc.
    pub fn build(&self) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // Here we would typically finalize the operation and return it.
        // For now, we just return Ok to indicate success.

        let builder_result = IntermediateStateBuilder::run(&self.instructions, self.op_ctx)?;

        let param = builder_result.operation_parameter;
        let instructions = builder_result.instructions;
        let prev_user_ref = self.previous_user_defined_operation.borrow();
        let mut interpreter = IntermediateInterpreter::new_for_self_op_id(
            self.self_op_id,
            param,
            self.op_ctx,
            &prev_user_ref,
        );

        let user_def_op = interpreter.create_user_defined_operation(
            instructions,
            builder_result.return_nodes,
            builder_result.return_edges,
        )?;

        // TODO: this is bad. we check validity of parameter both here and when encountering the next instructions.
        //  would be nicer if we had some way of indicating "validate this once current input phase is over"
        //  e.g., validate parameter once the parameter definition phase is over OR the function needs to be built.
        // check if the parameter is valid:
        user_def_op
            .signature
            .parameter
            .check_validity()
            .change_context(OperationBuilderError::InvalidParameter)?;

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

        let partial_user_def_op = {
            let prev_user_ref = self.previous_user_defined_operation.borrow();
            let mut interpreter = IntermediateInterpreter::new_for_self_op_id(
                0, // Unused. TODO: make prettier...
                builder_result.operation_parameter,
                self.op_ctx,
                &prev_user_ref,
            );
            interpreter.create_user_defined_operation(
                builder_result.instructions,
                builder_result.return_nodes,
                builder_result.return_edges,
            )?
        };
        *self.previous_user_defined_operation.borrow_mut() = partial_user_def_op;

        // in theory we should run it again with the new instruction.
        Ok(())
    }
}

impl<
    'a,
    S: Semantics<
            NodeAbstract: Debug,
            EdgeAbstract: Debug,
            BuiltinOperation: Clone,
            BuiltinQuery: Clone,
        >,
> OperationBuilderInefficient<'a, S>
{
    fn get_intermediate_state(
        &self,
    ) -> Result<(IntermediateState<S>, Vec<IntermediateStatePath>), OperationBuilderError> {
        let builder_result = IntermediateStateBuilder::run(&self.instructions, self.op_ctx)?;
        let prev_user_ref = self.previous_user_defined_operation.borrow();
        let mut interpreter = IntermediateInterpreter::new_for_self_op_id(
            0, // TODO: use a real operation ID here
            builder_result.operation_parameter,
            self.op_ctx,
            &prev_user_ref,
        );

        let (_, interp_instructions) =
            interpreter.interpret_instructions(builder_result.instructions)?;
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
        // TODO: an error implies the builder contains incorrect partial information.
        //  in such a case, it should have rolled back the last instruction.
        //  hence we should be fine to unwrap and not return a Result here.
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
}

struct IntermediateStateBuilder<'a, S: Semantics> {
    path: Vec<IntermediateStatePath>,
    _phantom_data: PhantomData<&'a S>,
}

use super::user_defined::Instruction as UDInstruction;

#[derive(derive_more::Debug)]
enum IntermediateInstruction<S: Semantics> {
    OpLike(IntermediateOpLike<S>),
    // #[debug("GraphShapeQuery({marker:#?}, {graph_instructions:#?}, {query_instructions:#?})")]
    GraphShapeQuery {
        /// Is the query finished and should we check it for connectedness?
        is_finished: bool,
        marker: AbstractOperationResultMarker,
        graph_instructions: Vec<GraphShapeQueryInstruction<S>>,
        query_instructions: IntermediateQueryInstructions<S>,
    },
    #[debug("BuiltinQuery(???, {_1:#?}, {_2:#?})")]
    BuiltinQuery(
        S::BuiltinQuery,
        Vec<AbstractNodeId>,
        IntermediateQueryInstructions<S>,
    ),
    // TODO: move to oplike?
    RenameNode {
        aid: AbstractNodeId,
        new_name: NamedMarker,
    },
}

#[derive(derive_more::Debug)]
enum IntermediateOpLike<S: Semantics> {
    #[debug("Builtin(???, {_1:#?})")]
    Builtin(S::BuiltinOperation, Vec<AbstractNodeId>),
    LibBuiltin(LibBuiltinOperation<S>, Vec<AbstractNodeId>),
    Operation(OperationId, Vec<AbstractNodeId>),
    Recurse(Vec<AbstractNodeId>),
}

#[derive(derive_more::Debug)]
struct IntermediateQueryInstructions<S: Semantics> {
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
enum GraphShapeQueryInstruction<S: Semantics> {
    #[debug("ExpectShapeNode({_0:#?})")]
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeNodeChange({_0:#?})")]
    ExpectShapeNodeChange(AbstractNodeId, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:#?}, {_1:#?})")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
}

struct BuilderResult<S: Semantics> {
    operation_parameter: OperationParameter<S>,
    instructions: Vec<(
        Option<AbstractOperationResultMarker>,
        IntermediateInstruction<S>,
    )>,
    state_path: Vec<IntermediateStatePath>,
    return_nodes: HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
    return_edges: HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,
}

// TODO: maybe this is not *intermediate* but actually the final state as well potentially?
impl<'a, S: Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>>
    IntermediateStateBuilder<'a, S>
{
    fn run(
        builder_instructions: &'a [BuilderInstruction<S>],
        op_ctx: &'a OperationContext<S>,
    ) -> Result<BuilderResult<S>, OperationBuilderError> {
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

        // check validity of the parameter if there's more instructions, since that implies the parameter should be done.
        if iter.peek().is_some() {
            op_parameter
                .check_validity()
                .change_context(OperationBuilderError::InvalidParameter)?;
        }

        let mut builder = Self {
            _phantom_data: PhantomData,
            path: Vec::new(),
        };

        let instructions = builder.build_many_instructions(&mut iter)?;

        let mut return_nodes = HashMap::new();
        let mut return_edges = HashMap::new();
        // if we are outside all queries, check for ReturnNode instructions.
        if !builder
            .path
            .iter()
            .any(|i| matches!(i, IntermediateStatePath::StartQuery(..)))
        {
            // we are outside all queries
            (return_nodes, return_edges) = Self::collect_return_instructions(&mut iter)?;
            // TODO: validate that we have not encountered a Recurse instruction. In recursive queries we cannot statically return.
        }

        // assert our iter is empty
        if let Some(next_instruction) = iter.peek() {
            bail!(OperationBuilderError::SuperfluousInstruction(format!(
                "{next_instruction:?}"
            )));
        }

        Ok(BuilderResult {
            operation_parameter: op_parameter,
            instructions,
            state_path: builder.path,
            return_nodes,
            return_edges,
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
            if instr.can_break_body() {
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
                    BuilderOpLike::LibBuiltin(lib_builtin_op) => {
                        IntermediateOpLike::LibBuiltin(lib_builtin_op.clone(), args.clone())
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
                let (is_finished, gsq_instructions, branch_instructions) =
                    self.build_shape_query(iter, op_marker.clone())?;
                // Ok((Some(*op_marker), UDInstruction::ShapeQuery()))
                Ok((
                    Some(op_marker.clone()), // NOTE: this marker is needed as well for the _concrete_ execution
                    IntermediateInstruction::GraphShapeQuery {
                        is_finished,
                        marker: op_marker.clone(),
                        graph_instructions: gsq_instructions,
                        query_instructions: branch_instructions,
                    },
                ))
            }
            BuilderInstruction::RenameNode(old_aid, new_name) => {
                iter.next();
                self.path.push(IntermediateStatePath::Advance);
                Ok((
                    None,
                    IntermediateInstruction::RenameNode {
                        aid: *old_aid,
                        new_name: *new_name,
                    },
                ))
            }
            _ => bail!(OperationBuilderError::ExpectedOperationOrQuery),
        }
    }

    fn build_shape_query(
        &mut self,
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
        operation_marker: AbstractOperationResultMarker,
    ) -> Result<
        (
            bool,
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

        // did the user finish the query and should we check that it's finished?
        // TODO: this doesn't capture the case where the user finishes the entire UDOp without entering a branch or ending the query.
        let mut did_query_finish = false;

        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::EnterTrueBranch => {
                    iter.next();
                    if true_branch_instructions.is_some() {
                        bail!(OperationBuilderError::AlreadyVisitedBranch(true));
                    }
                    // we are entering a true branch, so we remove the false branch from the path
                    self.remove_until_branch(false);
                    self.path.push(IntermediateStatePath::EnterTrue);
                    true_branch_instructions = Some(self.build_many_instructions(iter)?);
                    did_query_finish = true;
                }
                BuilderInstruction::EnterFalseBranch => {
                    iter.next();
                    if false_branch_instructions.is_some() {
                        bail!(OperationBuilderError::AlreadyVisitedBranch(false));
                    }
                    // we are entering a false branch, so we remove the true branch from the path
                    self.remove_until_branch(true);
                    self.path.push(IntermediateStatePath::EnterFalse);
                    false_branch_instructions = Some(self.build_many_instructions(iter)?);
                    did_query_finish = true;
                }
                BuilderInstruction::EndQuery => {
                    iter.next();
                    // we are ending the query, so we remove the current query state from the path
                    self.remove_until_query_start();
                    self.path.push(IntermediateStatePath::SkipQuery);
                    did_query_finish = true;
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
                BuilderInstruction::ExpectShapeNodeChange(aid, new_av) => {
                    iter.next();
                    gsq_instructions.push(GraphShapeQueryInstruction::ExpectShapeNodeChange(
                        aid.clone(),
                        new_av.clone(),
                    ));
                }
                BuilderInstruction::ExpectShapeEdge(source, target, abstract_value) => {
                    iter.next();
                    // Note: we need a current view of the abstract graph (or, well, AID mappings) so that we can build the GraphShapeQuery here which requires
                    //  an actual `Graph`.
                    // So we just follow a deferred approach by just passing along the instructions
                    gsq_instructions.push(GraphShapeQueryInstruction::ExpectShapeEdge(
                        source.clone(),
                        target.clone(),
                        abstract_value.clone(),
                    ));
                }
                _ => {
                    bail!(OperationBuilderError::InvalidInQuery);
                }
            }
        }

        Ok((
            did_query_finish,
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
                        bail!(OperationBuilderError::AlreadyVisitedBranch(true));
                    }
                    // we are entering a true branch, so we remove the false branch from the path
                    self.remove_until_branch(false);
                    self.path.push(IntermediateStatePath::EnterTrue);
                    true_branch_instructions = Some(self.build_many_instructions(iter)?);
                }
                BuilderInstruction::EnterFalseBranch => {
                    iter.next();
                    if false_branch_instructions.is_some() {
                        bail!(OperationBuilderError::AlreadyVisitedBranch(false));
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
                    bail!(OperationBuilderError::InvalidInQuery);
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
        let mut builder = OperationParameterBuilder::new();

        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::ExpectParameterNode(marker, node_abstract) => {
                    iter.next();
                    builder
                        .expect_explicit_input_node(*marker, node_abstract.clone())
                        .change_context(OperationBuilderError::InvalidParameter)?;
                }
                BuilderInstruction::ExpectContextNode(marker, node_abstract) => {
                    iter.next();
                    builder
                        .expect_context_node(*marker, node_abstract.clone())
                        .change_context(OperationBuilderError::InvalidParameter)?;
                }
                BuilderInstruction::ExpectParameterEdge(
                    source_marker,
                    target_marker,
                    edge_abstract,
                ) => {
                    iter.next();
                    builder
                        .expect_edge(*source_marker, *target_marker, edge_abstract.clone())
                        .change_context(OperationBuilderError::InvalidParameter)?;
                }
                _ => {
                    break;
                }
            }
        }

        builder
            .build()
            .change_context(OperationBuilderError::InvalidParameter)
    }

    // TODO: also collect ReturnEdge
    fn collect_return_instructions(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<
        (
            HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
            HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,
        ),
        OperationBuilderError,
    > {
        let mut return_nodes = HashMap::new();
        let mut return_edges = HashMap::new();
        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::ReturnNode(aid, output_marker, node) => {
                    iter.next();
                    if return_nodes.contains_key(aid) {
                        bail!(OperationBuilderError::AlreadySelectedReturnNode(
                            aid.clone(),
                        ));
                    }
                    return_nodes.insert(aid.clone(), (output_marker.clone(), node.clone()));
                }
                BuilderInstruction::ReturnEdge(source, target, edge) => {
                    iter.next();
                    if return_edges.contains_key(&(source.clone(), target.clone())) {
                        bail!(OperationBuilderError::AlreadySelectedReturnEdge(
                            source.clone(),
                            target.clone(),
                        ));
                    }
                    return_edges.insert((source.clone(), target.clone()), edge.clone());
                }

                _ => break,
            }
        }
        Ok((return_nodes, return_edges))
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct IntermediateStateAbstractOutputResult {
    new_aids: Vec<AbstractNodeId>,
    removed_aids: Vec<AbstractNodeId>,
}

// TODO: Store more information like:
//  - Are we still building the parameter graph?
//  - If we are inside a query, which branches have we not entered yet?
//  - Are we making a shape/non-shape query?

// TODO: should this be named "AbstractBuilderState"? since it's the state, which is abstract, which is used by the builder.
pub struct IntermediateState<S: Semantics> {
    pub graph: AbstractGraph<S>,
    pub node_keys_to_aid: BiMap<NodeKey, AbstractNodeId>,
    // TODO: Somehow remove AIDs from this set if they're completely overwritten by something non-shape-query.
    //  could be done by, whenever adding a new node, unconditionally removing the AID from this set as long as we're not in a shape query.
    //  since we have a different state at that point, it would get merged correctly (assuming we take the union).
    pub node_may_originate_from_shape_query: HashSet<AbstractNodeId>,
    pub edge_may_originate_from_shape_query: HashSet<(AbstractNodeId, AbstractNodeId)>,

    /// The most generic abstract type that may be written to each node, if any.
    pub node_may_be_written_to: HashMap<AbstractNodeId, S::NodeAbstract>,
    /// The most generic abstract type that may be written to each edge, if any.
    pub edge_may_be_written_to: HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,

    // TODO: make query path
    // TODO: should probably remove query_path from the state struct, and add it to a final returned StateWithQueryPath struct?
    pub query_path: Vec<QueryPath>,

    pub op_marker_counter: u64,
}

// TODO: unfortunately, we cannot derive Clone, since it implies a `S: Clone` bound.
//  - in theory, we could add that bound, since a Semantics as a value does not really store much. So clone should be fine.
impl<S: Semantics> Clone for IntermediateState<S> {
    fn clone(&self) -> Self {
        IntermediateState {
            graph: self.graph.clone(),
            node_keys_to_aid: self.node_keys_to_aid.clone(),
            node_may_originate_from_shape_query: self.node_may_originate_from_shape_query.clone(),
            edge_may_originate_from_shape_query: self.edge_may_originate_from_shape_query.clone(),
            node_may_be_written_to: self.node_may_be_written_to.clone(),
            edge_may_be_written_to: self.edge_may_be_written_to.clone(),
            query_path: self.query_path.clone(),
            op_marker_counter: self.op_marker_counter,
        }
    }
}

impl<S: Semantics> IntermediateState<S> {
    fn new() -> Self {
        IntermediateState {
            graph: AbstractGraph::<S>::new(),
            node_keys_to_aid: BiMap::new(),
            node_may_originate_from_shape_query: HashSet::new(),
            edge_may_originate_from_shape_query: HashSet::new(),
            node_may_be_written_to: HashMap::new(),
            edge_may_be_written_to: HashMap::new(),
            query_path: Vec::new(),
            op_marker_counter: 50000,
        }
    }

    fn from_param(param: &OperationParameter<S>) -> Self {
        let initial_graph = param.parameter_graph.clone();

        let mut initial_mapping = BiMap::new();

        for (key, subst) in param.node_keys_to_subst.iter() {
            let aid = AbstractNodeId::ParameterMarker(*subst);
            initial_mapping.insert(*key, aid);
        }

        IntermediateState {
            graph: initial_graph,
            node_keys_to_aid: initial_mapping,
            node_may_originate_from_shape_query: HashSet::new(),
            edge_may_originate_from_shape_query: HashSet::new(),
            node_may_be_written_to: HashMap::new(),
            edge_may_be_written_to: HashMap::new(),
            query_path: Vec::new(),
            op_marker_counter: 50000,
        }
    }

    fn get_next_op_result_marker(&mut self) -> AbstractOperationResultMarker {
        let marker = AbstractOperationResultMarker::Implicit(self.op_marker_counter);
        self.op_marker_counter += 1;
        marker
    }

    fn add_node(
        &mut self,
        aid: AbstractNodeId,
        node_abstract: S::NodeAbstract,
        from_shape_query: bool,
    ) {
        let node_key = self.graph.add_node(node_abstract);
        self.node_keys_to_aid.insert(node_key, aid);
        if from_shape_query {
            self.node_may_originate_from_shape_query.insert(aid);
        } else {
            // TODO: might be able to remove the AID.
        }
    }

    fn add_edge(
        &mut self,
        source: AbstractNodeId,
        target: AbstractNodeId,
        edge_abstract: S::EdgeAbstract,
        from_shape_query: bool,
    ) -> Result<(), OperationBuilderError> {
        let source_key = self
            .node_keys_to_aid
            .get_right(&source)
            .ok_or(OperationBuilderError::NotFoundAid(source))?;
        let target_key = self
            .node_keys_to_aid
            .get_right(&target)
            .ok_or(OperationBuilderError::NotFoundAid(target))?;

        self.graph.add_edge(*source_key, *target_key, edge_abstract);

        if from_shape_query {
            self.edge_may_originate_from_shape_query
                .insert((source, target));
        } else {
            // TODO: might be able to remove the AID.
        }
        Ok(())
    }

    fn set_node_av(
        &mut self,
        aid: AbstractNodeId,
        node_abstract: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let node_key = self
            .node_keys_to_aid
            .get_right(&aid)
            .ok_or(OperationBuilderError::NotFoundAid(aid))?;
        self.graph.set_node_attr(*node_key, node_abstract);
        Ok(())
    }

    fn contains_aid(&self, aid: &AbstractNodeId) -> bool {
        self.node_keys_to_aid.contains_right(aid)
    }

    fn contains_edge(&self, source: &AbstractNodeId, target: &AbstractNodeId) -> bool {
        let Some(source_key) = self.node_keys_to_aid.get_right(source) else {
            return false;
        };
        let Some(target_key) = self.node_keys_to_aid.get_right(target) else {
            return false;
        };
        self.graph
            .get_edge_attr((*source_key, *target_key))
            .is_some()
    }

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

    /// Modifies all mappings so that all mentions of `old_aid` are replaced with `new_aid`.
    fn rename_aid(
        &mut self,
        old_aid: AbstractNodeId,
        new_aid: AbstractNodeId,
    ) -> Result<(), OperationBuilderError> {
        // Update the mappings
        if let Some(node_key) = self.node_keys_to_aid.remove_right(&old_aid) {
            self.node_keys_to_aid.insert(node_key, new_aid);
        } else {
            bail!(OperationBuilderError::NotFoundAid(old_aid));
        }

        // Update the shape query sets
        if self.node_may_originate_from_shape_query.remove(&old_aid) {
            self.node_may_originate_from_shape_query.insert(new_aid);
        }
        // edges too
        self.edge_may_originate_from_shape_query = self
            .edge_may_originate_from_shape_query
            .iter()
            .map(|&(src, dst)| {
                let new_src = if src == old_aid { new_aid } else { src };
                let new_dst = if dst == old_aid { new_aid } else { dst };
                (new_src, new_dst)
            })
            .collect();

        // Update writes
        if let Some(node_av) = self.node_may_be_written_to.remove(&old_aid) {
            self.node_may_be_written_to.insert(new_aid, node_av);
        }
        self.edge_may_be_written_to = self
            .edge_may_be_written_to
            .iter()
            .map(|(&(src, dst), edge_av)| {
                let new_src = if src == old_aid { new_aid } else { src };
                let new_dst = if dst == old_aid { new_aid } else { dst };
                ((new_src, new_dst), edge_av.clone())
            })
            .collect();

        Ok(())
    }

    /// Returns the abstract changes from applying the op as well as the new AIDs
    fn interpret_op(
        &mut self,
        op_ctx: &OperationContext<S>,
        marker: Option<AbstractOperationResultMarker>,
        op: AbstractOperation<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(AbstractOperationArgument, IntermediateStateAbstractOutputResult), OperationBuilderError> {
        let param = op.parameter();
        let (subst, abstract_arg) = self.get_substitution(&param, args)?;

        // now apply op and store result
        let operation_output = {
            let mut gws = GraphWithSubstitution::new(&mut self.graph, &subst);
            op.apply_abstract(op_ctx, &mut gws)
                .change_context(OperationBuilderError::AbstractApplyOperationError2)?
        };
        let output = self.handle_abstract_output_changes(marker, operation_output)?;

        Ok((abstract_arg, output))
    }

    fn interpret_builtin_query(
        &mut self,
        query: &S::BuiltinQuery,
        args: Vec<AbstractNodeId>,
    ) -> Result<AbstractOperationArgument, OperationBuilderError> {
        let param = query.parameter();
        let (subst, abstract_arg) = self.get_substitution(&param, args)?;
        // now apply the query and store result
        let mut gws = GraphWithSubstitution::new(&mut self.graph, &subst);
        query.apply_abstract(&mut gws);
        Ok(abstract_arg)
    }

    /// Returns the newly added AIDs
    fn handle_abstract_output_changes(
        &mut self,
        marker: Option<AbstractOperationResultMarker>,
        operation_output: AbstractOperationOutput<S>,
    ) -> Result<IntermediateStateAbstractOutputResult, OperationBuilderError> {
        // go over new nodes
        let mut new_aids = Vec::new();
        for (node_marker, node_key) in operation_output.new_nodes {
            if let Some(op_marker) = marker {
                let aid = AbstractNodeId::DynamicOutputMarker(op_marker, node_marker);
                // TODO: override the may_come_from_shape_query set here! remove the node - it's a non-shape-query node.
                self.node_keys_to_aid.insert(node_key, aid);
                new_aids.push(aid);
            } else {
                // we don't keep track of it, so better remove it from the graph
                self.graph.remove_node(node_key);
            }
        }
        let mut removed_aids = Vec::new();
        for node_key in &operation_output.removed_nodes {
            // remove the node from the mapping
            if let Some(removed_aid) = self.node_keys_to_aid.remove_left(&node_key) {
                removed_aids.push(removed_aid);
            }
        }

        // collect changes
        // TODO: What is a good idea regarding changes abstract values?
        //  I think it's a good idea to just propagate what we know for a fact _could_ be written (but in its most precise form).
        //  If instead we said "merge it with the current value", then we make it potentially join with the parameter.
        for (key, node_abstract) in operation_output.changed_abstract_values_nodes {
            let aid = self
                .get_aid_from_key(&key)
                .expect("internal error: changed node not found in mapping");
            self.node_may_be_written_to.insert(aid, node_abstract);
        }
        for ((source, target), edge_abstract) in operation_output.changed_abstract_values_edges {
            let source_aid = self
                .get_aid_from_key(&source)
                .expect("internal error: changed edge source not found in mapping");
            let target_aid = self
                .get_aid_from_key(&target)
                .expect("internal error: changed edge target not found in mapping");
            self.edge_may_be_written_to
                .insert((source_aid, target_aid), edge_abstract);
        }

        Ok(IntermediateStateAbstractOutputResult {
            new_aids,
            removed_aids,
        })
    }

    fn get_substitution(
        &self,
        param: &OperationParameter<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(ParameterSubstitution, AbstractOperationArgument), OperationBuilderError> {
        let selected_inputs = args
            .iter()
            .map(|aid| self.get_key_from_aid(aid))
            .collect::<Result<Vec<_>, _>>()
            .change_context(OperationBuilderError::SelectedInputsNotFoundAid)?;
        let subst = get_substitution(&self.graph, &param, &selected_inputs)
            .change_context(OperationBuilderError::SubstitutionErrorNew)?;
        let subst_to_aid = subst.mapping.iter().map(|(subst, key)| {
            let aid = self.get_aid_from_key(key)
                .change_context(OperationBuilderError::InternalError("node key should be in mapping, because all node keys from the abstract graph should be in the mapping"))
                .unwrap();
            (subst.clone(), aid)
        }).collect();

        let abstract_arg = AbstractOperationArgument {
            selected_input_nodes: args,
            subst_to_aid,
        };

        Ok((subst, abstract_arg))
    }

    fn get_key_from_aid(&self, aid: &AbstractNodeId) -> Result<NodeKey, OperationBuilderError> {
        self.node_keys_to_aid
            .get_right(aid)
            .cloned()
            .ok_or(report!(OperationBuilderError::NotFoundAid(*aid)))
    }

    fn get_aid_from_key(&self, key: &NodeKey) -> Result<AbstractNodeId, OperationBuilderError> {
        self.node_keys_to_aid.get_left(key).cloned().ok_or(report!(
            OperationBuilderError::InternalError("could not find node key")
        ))
    }

    fn as_param_for_shape_query(&self) -> (OperationParameter<S>, AbstractOperationArgument) {
        let param_graph = self.graph.clone();

        let mut all_node_keys = param_graph
            .node_attr_map
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        all_node_keys.sort_unstable(); // sort to ensure deterministic order

        let mut node_keys_to_subst: BiMap<NodeKey, SubstMarker> = BiMap::new();
        let mut explicit_input_nodes = Vec::new();
        let mut aid_args = Vec::new();
        let mut subst_to_aid = HashMap::new();
        for key in all_node_keys {
            let subst = SubstMarker::from(format!("{:?}", key));
            node_keys_to_subst.insert(key, subst);
            explicit_input_nodes.push(subst);
            // collect the AID for this key
            let aid = self.get_aid_from_key(&key).unwrap();
            aid_args.push(aid);
            subst_to_aid.insert(subst, aid);
        }

        let abstract_args = AbstractOperationArgument {
            selected_input_nodes: aid_args,
            subst_to_aid,
        };

        (
            OperationParameter {
                explicit_input_nodes,
                parameter_graph: param_graph,
                node_keys_to_subst,
            },
            abstract_args,
        )
    }
}

impl<S: Semantics<NodeAbstract: Debug, EdgeAbstract: Debug>> IntermediateState<S> {
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
                    AbstractNodeId::Named(name) => {
                        write!(f, "{:?}", name)
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

                    // format!("label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                    // format!("label = \"{dbg_attr_replaced}\", xlabel = \"{aid_replaced}\"")
                    format!("shape=Mrecord, label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                }
            )
        )
    }
}

enum InterpretedInstruction<S: Semantics> {
    OpLike,
    Query(InterpretedQueryInstructions<S>),
}

struct InterpretedQueryInstructions<S: Semantics> {
    initial_state_true_branch: IntermediateState<S>,
    initial_state_false_branch: IntermediateState<S>,
    true_branch: InterpretedInstructions<S>,
    false_branch: InterpretedInstructions<S>,
}

struct InterpretedInstructionWithState<S: Semantics> {
    instruction: InterpretedInstruction<S>,
    state_after: IntermediateState<S>,
}

struct IntermediateInterpreter<'a, S: Semantics> {
    self_op_id: OperationId,
    op_ctx: &'a OperationContext<S>,
    op_param: OperationParameter<S>,
    initial_state: IntermediateState<S>,
    current_state: IntermediateState<S>,
    /// Primarily used for hacking abstract changes due to recursive calls.
    partial_self_user_defined_op: &'a UserDefinedOperation<S>,
    /// A counter to generate unique operation result markers.
    counter: u64,
}

type UDInstructionsWithMarker<S> = Vec<(Option<AbstractOperationResultMarker>, UDInstruction<S>)>;

type InterpretedInstructions<S> = Vec<(
    Option<AbstractOperationResultMarker>,
    InterpretedInstructionWithState<S>,
)>;

impl<'a, S: Semantics> IntermediateInterpreter<'a, S> {
    fn new_for_self_op_id(
        self_op_id: OperationId,
        op_param: OperationParameter<S>,
        op_ctx: &'a OperationContext<S>,
        partial_self_user_defined_op: &'a UserDefinedOperation<S>,
    ) -> Self {
        let initial_state = IntermediateState::from_param(&op_param);

        let current_state = initial_state.clone();

        let interpreter = IntermediateInterpreter {
            self_op_id,
            op_ctx,
            op_param,
            initial_state,
            current_state,
            partial_self_user_defined_op,
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
        return_edges: HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,
    ) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        let (ud_instructions, _interp_instructions) =
            self.interpret_instructions(intermediate_instructions)?;

        // self.current_state is now the final inferred state.

        let (ud_output, signature) = self.determine_signature(return_nodes, return_edges)?;

        Ok(UserDefinedOperation {
            // parameter: self.op_param.clone(),
            instructions: ud_instructions,
            output_changes: ud_output,
            signature,
        })
    }

    // Note: must be called after interpreting all instructions.
    fn determine_signature(
        &self,
        return_nodes: HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
        return_edges: HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,
    ) -> Result<(AbstractUserDefinedOperationOutput, OperationSignature<S>), OperationBuilderError>
    {
        // this struct stores an instruction for user defined operations on *how* to return nodes.
        let mut ud_output = AbstractUserDefinedOperationOutput::new();
        // this stores in general *what* the operation is doing.
        let mut signature = OperationSignature::empty_new("name", self.op_param.clone());

        // need to determine validity of return_nodes
        for (aid, (output_marker, node_abstract)) in return_nodes {
            // make sure type we're deciding to return is a valid supertype
            let inferred_av = self
                .current_state
                .node_av_of_aid(&aid)
                .ok_or(OperationBuilderError::NotFoundReturnNode(aid.clone()))?;
            if !S::NodeMatcher::matches(inferred_av, &node_abstract) {
                bail!(OperationBuilderError::InvalidReturnNodeType(aid));
            }
            if self
                .current_state
                .node_may_originate_from_shape_query
                .contains(&aid)
            {
                bail!(OperationBuilderError::ReturnNodeMayOriginateFromShapeQuery(
                    aid,
                ));
            }
            ud_output.new_nodes.insert(aid, output_marker);
            // Add to signature
            signature
                .output
                .new_nodes
                .insert(output_marker, node_abstract);
        }

        let get_param_or_output_sig_id = |aid: &AbstractNodeId| {
            match *aid {
                AbstractNodeId::ParameterMarker(s) => Ok(AbstractSignatureNodeId::ExistingNode(s)),
                AbstractNodeId::DynamicOutputMarker(_, _) | AbstractNodeId::Named(..) => {
                    // we must be returning this node if we want to return an incident edge.
                    let Some(output_marker) = ud_output.new_nodes.get(aid) else {
                        bail!(OperationBuilderError::NotFoundReturnNode(aid.clone()));
                    };
                    Ok(AbstractSignatureNodeId::NewNode(output_marker.clone()))
                }
            }
        };

        // need to determine validity of return_edges
        for ((source_aid, target_aid), edge_abstract) in return_edges {
            let Some(source_key) = self.current_state.node_keys_to_aid.get_right(&source_aid)
            else {
                bail!(OperationBuilderError::NotFoundReturnEdgeSource(source_aid));
            };
            let Some(target_key) = self.current_state.node_keys_to_aid.get_right(&target_aid)
            else {
                bail!(OperationBuilderError::NotFoundReturnEdgeTarget(target_aid));
            };
            let inferred_edge_av = self
                .current_state
                .edge_av_of_aid(&source_aid, &target_aid)
                .ok_or(OperationBuilderError::NotFoundReturnEdge(
                    source_aid.clone(),
                    target_aid.clone(),
                ))?;
            if !S::EdgeMatcher::matches(inferred_edge_av, &edge_abstract) {
                bail!(OperationBuilderError::InvalidReturnEdgeType(
                    source_aid, target_aid,
                ));
            }
            if self
                .current_state
                .edge_may_originate_from_shape_query
                .contains(&(source_aid, target_aid))
            {
                bail!(OperationBuilderError::ReturnEdgeMayOriginateFromShapeQuery(
                    source_aid, target_aid,
                ));
            }

            // Add to signature
            let source_sig_id = get_param_or_output_sig_id(&source_aid)?;
            let target_sig_id = get_param_or_output_sig_id(&target_aid)?;
            signature
                .output
                .new_edges
                .insert((source_sig_id, target_sig_id), edge_abstract.clone());
        }

        // deleted nodes and edges can be inferred from what's missing from the current state vs. op_param.

        let initial_subst_nodes = self
            .op_param
            .node_keys_to_subst
            .right_values()
            .cloned()
            .collect::<HashSet<_>>();
        let current_subst_nodes = self
            .current_state
            .node_keys_to_aid
            .right_values()
            .filter_map(|aid| {
                if let AbstractNodeId::ParameterMarker(subst) = aid {
                    Some(subst.clone())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        // deleted nodes are those that were in the initial substitution but not in the current state
        let deleted_nodes: HashSet<_> = initial_subst_nodes
            .difference(&current_subst_nodes)
            .cloned()
            .collect();
        signature.output.maybe_deleted_nodes = deleted_nodes;

        let mut initial_edges = HashSet::new();
        for (source, target, _) in self.op_param.parameter_graph.graph.all_edges() {
            let Some(source_subst) = self.op_param.node_keys_to_subst.get_left(&source) else {
                continue; // should not happen, but just in case
            };
            let Some(target_subst) = self.op_param.node_keys_to_subst.get_left(&target) else {
                continue; // should not happen, but just in case
            };
            initial_edges.insert((*source_subst, *target_subst));
        }

        let mut current_edges = HashSet::new();
        for (source, target, _) in self.current_state.graph.graph.all_edges() {
            let Some(source_aid) = self.current_state.node_keys_to_aid.get_left(&source) else {
                continue; // should not happen, but just in case
            };
            let Some(target_aid) = self.current_state.node_keys_to_aid.get_left(&target) else {
                continue; // should not happen, but just in case
            };
            if let (
                AbstractNodeId::ParameterMarker(source_subst),
                AbstractNodeId::ParameterMarker(target_subst),
            ) = (source_aid, target_aid)
            {
                current_edges.insert((source_subst.clone(), target_subst.clone()));
            }
        }

        // deleted edges are those that were in the initial substitution but not in the current state
        let deleted_edges: HashSet<_> = initial_edges.difference(&current_edges).cloned().collect();
        signature.output.maybe_deleted_edges = deleted_edges;

        // changed nodes and edges must be kept track of during the interpretation, including calls to child operations.

        for (aid, node_abstract) in &self.current_state.node_may_be_written_to {
            // we care about reporting only subst markers
            let AbstractNodeId::ParameterMarker(subst) = aid else {
                continue;
            };
            signature
                .output
                .maybe_changed_nodes
                .insert(*subst, node_abstract.clone());
        }

        for ((source_aid, target_aid), edge_abstract) in &self.current_state.edge_may_be_written_to
        {
            // we care about reporting only subst markers
            let AbstractNodeId::ParameterMarker(source_subst) = source_aid else {
                continue;
            };
            let AbstractNodeId::ParameterMarker(target_subst) = target_aid else {
                continue;
            };
            signature
                .output
                .maybe_changed_edges
                .insert((*source_subst, *target_subst), edge_abstract.clone());
        }

        Ok((ud_output, signature))
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
                self.interpret_op_like(marker, oplike)
                    .attach_printable_lazy(|| format!("Failed OpLike"))?,
                InterpretedInstruction::OpLike,
            )),
            IntermediateInstruction::BuiltinQuery(query, args, query_instructions) => {
                self.interpret_builtin_query(query, args, query_instructions)
            }
            IntermediateInstruction::GraphShapeQuery {
                is_finished,
                marker,
                graph_instructions,
                query_instructions,
            } => self.interpret_graph_shape_query(
                is_finished,
                marker,
                graph_instructions,
                query_instructions,
            ),
            IntermediateInstruction::RenameNode { aid, new_name } => {
                let new_aid = AbstractNodeId::named(new_name);

                Ok((
                    self.rename_node(aid, new_aid)?,
                    InterpretedInstruction::OpLike,
                ))
            }
        }
    }

    fn rename_node(
        &mut self,
        old_aid: AbstractNodeId,
        new_aid: AbstractNodeId,
    ) -> Result<UDInstruction<S>, OperationBuilderError> {
        // don't allow renaming ParameterMarker nodes
        if let AbstractNodeId::ParameterMarker(_) = old_aid {
            bail!(OperationBuilderError::CannotRenameParameterNode(old_aid));
        }
        self.current_state.rename_aid(old_aid, new_aid)?;
        Ok(UDInstruction::RenameNode {
            old: old_aid,
            new: new_aid,
        })
    }

    fn interpret_op_like(
        &mut self,
        marker: Option<AbstractOperationResultMarker>,
        oplike: IntermediateOpLike<S>,
    ) -> Result<UDInstruction<S>, OperationBuilderError> {
        match oplike {
            IntermediateOpLike::Builtin(builtin_op, args) => {
                let op = Operation::Builtin(&builtin_op);
                let abstract_arg =
                    self.interpret_op(marker, op, args)
                        .attach_printable_lazy(|| {
                            format!("Failed to interpret builtin operation: {builtin_op:?}")
                        })?;

                Ok(UDInstruction::OpLike(
                    OpLikeInstruction::Builtin(builtin_op),
                    abstract_arg,
                ))
            }
            IntermediateOpLike::LibBuiltin(lib_builtin_op, args) => {
                let op = Operation::LibBuiltin(&lib_builtin_op);
                let abstract_arg =
                    self.interpret_op(marker, op, args)
                        .attach_printable_lazy(|| {
                            format!("Failed to interpret lib builtin operation: {lib_builtin_op:?}")
                        })?;

                Ok(UDInstruction::OpLike(
                    OpLikeInstruction::LibBuiltin(lib_builtin_op),
                    abstract_arg,
                ))
            }
            IntermediateOpLike::Operation(id, args) => {
                let op = self
                    .op_ctx
                    .get(id)
                    .ok_or(OperationBuilderError::NotFoundOperationId(id))?;
                let abstract_arg = self
                    .interpret_op(marker, op, args)
                    .attach_printable_lazy(|| format!("Failed to interpret operation: {id:?}"))?;

                Ok(UDInstruction::OpLike(
                    OpLikeInstruction::Operation(id),
                    abstract_arg,
                ))
            }
            IntermediateOpLike::Recurse(args) => {
                // TODO: recursion is actually tricky. because at this point we have not finished interpreting the current operation yet.
                //  So how are we supposed to know the abstract changes?

                // TODO: use approach from `problems-testcases.md`

                // this should probably use some pre-defined (at the beginning) abstract changes to the graph.

                // Hack: pretend the partial user defined operation is the full operation.
                // TODO: remove this hack. Make it so only explicit changes via ExpectRecursionChange... instructions are allowed.
                //  (note: it's also unsound for now, since changes _after_ the recursion call are ignored. -- actually, not quite. see tests)
                let op = Operation::Custom(&self.partial_self_user_defined_op);

                let abstract_arg = self
                    .interpret_op(marker, op, args)
                    .attach_printable_lazy(|| "Failed to interpret recursive call")?;

                Ok(UDInstruction::OpLike(
                    OpLikeInstruction::Operation(self.self_op_id),
                    abstract_arg,
                ))
            }
        }
    }

    fn interpret_op(
        &mut self,
        marker: Option<AbstractOperationResultMarker>,
        op: Operation<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<AbstractOperationArgument, OperationBuilderError> {
        self.current_state
            .interpret_op(
                &self.op_ctx,
                marker,
                AbstractOperation::from_operation(op),
                args,
            )
            .map(|x| x.0)
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
        query.apply_abstract(&mut GraphWithSubstitution::new(
            &mut self.current_state.graph,
            &subst,
        ));

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

    // TESTING
    fn collect_self_state_to_parameter(
        &self,
    ) -> (OperationParameter<S>, AbstractOperationArgument) {
        self.current_state.as_param_for_shape_query()
    }

    // see _old for a version that only collects the bare minimum for the parameter.
    // this version pretends the current abstract state is the parameter.
    // TODO: clean this function up with the above assumptions. right now it's just bare-minimum working.
    fn interpret_graph_shape_query(
        &mut self,
        is_finished: bool,
        gsq_op_marker: AbstractOperationResultMarker,
        gsq_instructions: Vec<GraphShapeQueryInstruction<S>>,
        query_instructions: IntermediateQueryInstructions<S>,
    ) -> Result<(UDInstruction<S>, InterpretedInstruction<S>), OperationBuilderError> {
        let state_before = self.current_state.clone();

        // preparation for false branch
        let false_branch_state = self.current_state.clone();
        let initial_false_branch_state = false_branch_state.clone();

        // first pass: collect the initial graph (the parameter)
        let (param, abstract_args) = self.collect_self_state_to_parameter();

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
                    let aid =
                        AbstractNodeId::DynamicOutputMarker(gsq_op_marker.clone(), marker.clone());
                    self.current_state
                        .node_keys_to_aid
                        .insert(state_key, aid.clone());
                    self.current_state
                        .node_may_originate_from_shape_query
                        .insert(aid.clone());
                }
                GraphShapeQueryInstruction::ExpectShapeNodeChange(aid, av) => {
                    // set the expected av
                    let key = self.get_current_key_from_aid(aid.clone())?;
                    expected_graph.set_node_attr(key, av.clone());

                    // now update the state for the true branch.
                    let state_key = self
                        .get_current_key_from_aid(aid)
                        .change_context(OperationBuilderError::NotFoundAid(aid))?;
                    self.current_state.graph.set_node_attr(state_key, av);
                }
                GraphShapeQueryInstruction::ExpectShapeEdge(src, target, av) => {
                    let src_key = self.get_current_key_from_aid(src.clone())?;
                    let target_key = self.get_current_key_from_aid(target.clone())?;
                    expected_graph.add_edge(src_key, target_key, av.clone());

                    // now update the state for the true branch.
                    let state_src_key = self
                        .get_current_key_from_aid(src)
                        .change_context(OperationBuilderError::ShapeEdgeSourceNotFound)?;
                    let state_target_key = self
                        .get_current_key_from_aid(target)
                        .change_context(OperationBuilderError::ShapeEdgeTargetNotFound)?;
                    self.current_state
                        .graph
                        .add_edge(state_src_key, state_target_key, av);
                    self.current_state
                        .edge_may_originate_from_shape_query
                        .insert((src.clone(), target.clone()));
                }
            }
        }

        let gsq = GraphShapeQuery::new(param, expected_graph, node_keys_to_shape_idents);

        // TODO: need to validate GSQ somewhere.
        //  Most importantly, that there are no free floating shape nodes.
        // TODO: do this with under the is_finished flag.

        let initial_true_branch_state = self.current_state.clone();

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

    fn interpret_graph_shape_query_old(
        &mut self,
        is_finished: bool,
        gsq_op_marker: AbstractOperationResultMarker,
        gsq_instructions: Vec<GraphShapeQueryInstruction<S>>,
        query_instructions: IntermediateQueryInstructions<S>,
    ) -> Result<(UDInstruction<S>, InterpretedInstruction<S>), OperationBuilderError> {
        let state_before = self.current_state.clone();

        // preparation for false branch
        let false_branch_state = self.current_state.clone();
        let initial_false_branch_state = false_branch_state.clone();

        // first pass: collect the initial graph (the parameter)
        let mut param_builder = OperationParameterBuilder::new();

        let mut abstract_args = Vec::new();

        let mut arg_aid_to_param_subst: BiMap<AbstractNodeId, SubstMarker> = BiMap::new();
        let mut arg_aid_to_node_keys: BiMap<AbstractNodeId, NodeKey> = BiMap::new();

        // Collects the AID and adds it to all relevant mappings.
        // The passed AID is a node that is part of the pre-existing graph.
        let mut collect_aid = |aid: AbstractNodeId| -> Result<(), OperationBuilderError> {
            if arg_aid_to_param_subst.contains_left(&aid) {
                // we already processed this
                return Ok(());
            }
            // invent a new subst marker for this AID.
            let subst_marker = param_builder.next_subst_marker();
            let key = self.get_current_key_from_aid(aid)?;
            let abstract_value = self
                .current_state
                .graph
                .get_node_attr(key)
                .expect(
                    "internal error: node key should be in state graph since it is in the mapping",
                )
                .clone();
            // the shape query will expect the same AV
            // context-matching here is against the purpose of shape queries, so every argument is explicit
            param_builder
                .expect_explicit_input_node(subst_marker, abstract_value)
                .change_context(OperationBuilderError::InternalError(
                    "node should not be in param",
                ))?;
            // we need to push in the same sequence as expected in explicit_input_nodes
            abstract_args.push(aid.clone());
            arg_aid_to_param_subst.insert(aid.clone(), subst_marker.clone());
            arg_aid_to_node_keys.insert(aid.clone(), key);
            Ok(())
        };

        // Collects the AID if it is part of the pre-existing graph.
        let mut collect_non_shape_ident =
            |&aid: &AbstractNodeId| -> Result<(), OperationBuilderError> {
                match aid {
                    AbstractNodeId::ParameterMarker(_) => {
                        // we need this.
                        collect_aid(aid)?;
                    }
                    AbstractNodeId::DynamicOutputMarker(orm, node_marker) => {
                        // we need this, but only if it is not from the current graph shape query.
                        if orm != gsq_op_marker {
                            collect_aid(aid)?;
                        }
                    }
                    AbstractNodeId::Named(..) => {
                        // same as above dynamic, except we know that it is not from the current graph shape query since we couldn't have
                        // renamed the matched node yet.
                        collect_aid(aid)?;
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
                    // TODO: really we need to collect the entire connected components associated with src and target here?
                    collect_non_shape_ident(src)?;
                    collect_non_shape_ident(target)?;
                }
                GraphShapeQueryInstruction::ExpectShapeNodeChange(aid, _) => {
                    // we need this node to be in the initial graph.
                    collect_non_shape_ident(aid)?;
                }
            }
        }

        let param = param_builder
            .build()
            .change_context(OperationBuilderError::InternalError(
                "Failed to build operation parameter for graph shape query",
            ))?;

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
                    let aid =
                        AbstractNodeId::DynamicOutputMarker(gsq_op_marker.clone(), marker.clone());
                    self.current_state
                        .node_keys_to_aid
                        .insert(state_key, aid.clone());
                    self.current_state
                        .node_may_originate_from_shape_query
                        .insert(aid.clone());
                }
                GraphShapeQueryInstruction::ExpectShapeNodeChange(aid, av) => {
                    // set the expected av
                    let key = aid_to_node_key_hack!(aid.clone())?;
                    expected_graph.set_node_attr(key, av.clone());

                    // now update the state for the true branch.
                    let state_key = self
                        .get_current_key_from_aid(aid)
                        .change_context(OperationBuilderError::NotFoundAid(aid))?;
                    self.current_state.graph.set_node_attr(state_key, av);
                }
                GraphShapeQueryInstruction::ExpectShapeEdge(src, target, av) => {
                    let src_key = aid_to_node_key_hack!(src.clone())?;
                    let target_key = aid_to_node_key_hack!(target.clone())?;
                    expected_graph.add_edge(src_key, target_key, av.clone());

                    // now update the state for the true branch.
                    let state_src_key = self
                        .get_current_key_from_aid(src)
                        .change_context(OperationBuilderError::ShapeEdgeSourceNotFound)?;
                    let state_target_key = self
                        .get_current_key_from_aid(target)
                        .change_context(OperationBuilderError::ShapeEdgeTargetNotFound)?;
                    self.current_state
                        .graph
                        .add_edge(state_src_key, state_target_key, av);
                    self.current_state
                        .edge_may_originate_from_shape_query
                        .insert((src.clone(), target.clone()));
                }
            }
        }

        let gsq = GraphShapeQuery::new(param, expected_graph, node_keys_to_shape_idents);

        // TODO: need to validate GSQ somewhere.
        //  Most importantly, that there are no free floating shape nodes.
        // TODO: do this with under the is_finished flag.

        let initial_true_branch_state = self.current_state.clone();

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
            AbstractOperationArgument {
                selected_input_nodes: abstract_args,
                subst_to_aid: arg_aid_to_param_subst.into_right_map(),
            },
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
        self.current_state.get_substitution(param, args)
    }

    fn get_new_unnamed_abstract_operation_marker(&mut self) -> AbstractOperationResultMarker {
        let val = self.counter;
        self.counter += 1;
        AbstractOperationResultMarker::Implicit(val)
    }

    fn get_current_key_from_aid(
        &self,
        aid: AbstractNodeId,
    ) -> Result<NodeKey, OperationBuilderError> {
        self.current_state.get_key_from_aid(&aid)
    }

    fn get_current_aid_from_key(
        &self,
        key: NodeKey,
    ) -> Result<AbstractNodeId, OperationBuilderError> {
        self.current_state.get_aid_from_key(&key)
    }
}

fn get_state_for_path<S: Semantics>(
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

fn get_query_path_for_path<S: Semantics>(
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
fn merge_states<S: Semantics>(
    is_true_shape: bool,
    state_true: &IntermediateState<S>,
    state_false: &IntermediateState<S>,
) -> IntermediateState<S> {
    merge_states_result(is_true_shape, state_true, state_false)
        .merged_state
}

struct MergeStatesResult<S: Semantics> {
    merged_state: IntermediateState<S>,
    /// The AIDs that are present in the true state but not in the merged state.
    missing_from_true: HashSet<AbstractNodeId>,
    /// The AIDs that are present in the false state but not in the merged state.
    missing_from_false: HashSet<AbstractNodeId>,
}

fn merge_states_result<S: Semantics>(
    is_true_shape: bool,
    state_true: &IntermediateState<S>,
    state_false: &IntermediateState<S>,
) -> MergeStatesResult<S> {
    // TODO: handle `is_true_shape`.
    //  ^ actually, we're doing that in interpret_graph_shape_query, so we don't need to do it here, I think.

    let mut new_state = IntermediateState::new();

    let mut common_aids = HashSet::new();
    // First, collect all AIDs that are present in both states.
    for aid in state_true.node_keys_to_aid.right_values() {
        if state_false.node_keys_to_aid.contains_right(aid) {
            common_aids.insert(aid.clone());
        }
    }

    // Now, for each common AID, we need to merge the nodes and info from both states.
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
        // Keep track of the node originating from a shape query...
        if state_true
            .node_may_originate_from_shape_query
            .contains(&aid)
            || state_false
            .node_may_originate_from_shape_query
            .contains(&aid)
        {
            new_state.node_may_originate_from_shape_query.insert(aid);
        }
        // ... as well as the written types.
        // We take the join-union of the written types from both states.
        let written_av_true = state_true.node_may_be_written_to.get(&aid).cloned();
        let written_av_false = state_false.node_may_be_written_to.get(&aid).cloned();
        let merged_written_av = match (written_av_true, written_av_false) {
            (Some(av_true), Some(av_false)) => {
                // Note: we need this to be some, since we've already inserted the node in the new graph.
                // for more detail, see the comment in the edges section below.
                Some(S::join_nodes(&av_true, &av_false).expect(
                    "client semantics error: expected to be able to merge written node attributes",
                ))
            }
            (Some(av_true), None) => Some(av_true),
            (None, Some(av_false)) => Some(av_false),
            (None, None) => None,
        };
        if let Some(merged_av) = merged_written_av {
            new_state.node_may_be_written_to.insert(aid, merged_av);
        }
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
        let Some(av_false) = state_false
            .graph
            .get_edge_attr((*from_key_false, *to_key_false))
        else {
            // If the edge does not exist in the false state, we skip it.
            continue;
        };
        let Some(merged_av) = S::join_edges(av_true, av_false) else {
            // If we cannot merge the edges, we skip this edge.
            continue;
        };
        // Add the merged edge to the new state.
        new_state
            .graph
            .add_edge(*from_key_merged, *to_key_merged, merged_av);
        // Keep track of the edge originating from a shape query.
        let edge = (from_aid.clone(), to_aid.clone());
        if state_true
            .edge_may_originate_from_shape_query
            .contains(&edge)
            || state_false
            .edge_may_originate_from_shape_query
            .contains(&edge)
        {
            new_state.edge_may_originate_from_shape_query.insert(edge);
        }

        let written_av_true = state_true
            .edge_may_be_written_to
            .get(&(from_aid.clone(), to_aid.clone()))
            .cloned();
        let written_av_false = state_false
            .edge_may_be_written_to
            .get(&(from_aid.clone(), to_aid.clone()))
            .cloned();
        let merged_written_av = match (written_av_true, written_av_false) {
            (Some(av_true), Some(av_false)) => {
                // Note: this must be Some, because we have the edge in our merged graph for a fact.
                // If we were to ignore it *just for edge_may_be_written_to* if the written values could not be merged,
                // we'd unsoundly skip returning information about potential changes to the edge.
                // I.e., we expect client semantics to support written-av merges if the branch merge succeeded in the first place.
                // this should generally be the case.
                // Update: Below cannot panic for transitive client type systems.
                //  since we were able to join the edge, that means the inferred AVs for both branches
                //  were compatible. We *know* that a written_av *must be* a subtype of the inferred AVs,
                //  since whenever we write we join that write to the current abstract graph AV.
                //  So, we know that some super type of av_true and some super type of av_false
                //  were joinable, and in a transitive type system, that means that av_true and av_false
                //  must be joinable as well. QED.
                Some(S::join_edges(&av_true, &av_false).expect(
                    "client semantics error: expected to be able to merge written edge attributes",
                ))
            }
            (Some(av_true), None) => Some(av_true),
            (None, Some(av_false)) => Some(av_false),
            (None, None) => None,
        };
        if let Some(merged_av) = merged_written_av {
            new_state
                .edge_may_be_written_to
                .insert((from_aid.clone(), to_aid.clone()), merged_av);
        }

        // TODO: edge orders need to be handled here.
    }

    // which AIDs are actually present in the merged state, taking into account everything (names, type join)
    let final_merged_state_aids = new_state
        .node_keys_to_aid
        .right_values()
        .cloned()
        .collect::<HashSet<_>>();

    MergeStatesResult {
        merged_state: new_state,
        missing_from_true: state_true
            .node_keys_to_aid
            .right_values()
            .cloned()
            .filter(|aid| !final_merged_state_aids.contains(aid))
            .collect(),
        missing_from_false: state_false
            .node_keys_to_aid
            .right_values()
            .cloned()
            .filter(|aid| !final_merged_state_aids.contains(aid))
            .collect(),
    }
}
