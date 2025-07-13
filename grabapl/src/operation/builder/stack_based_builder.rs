use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use derive_more::From;
use derive_more::with_trait::TryInto;
use error_stack::{bail, report, Report, ResultExt};
use thiserror::Error;
use crate::{OperationContext, OperationId, Semantics, SubstMarker};
use crate::operation::builder::{merge_states, BuilderInstruction, BuilderOpLike, IntermediateInterpreter, IntermediateState, IntermediateStateBuilder, OperationBuilderInefficient, OperationBuilderError, UDInstructionsWithMarker};
use crate::operation::signature::parameter::{AbstractOutputNodeMarker, OperationParameter};
use crate::operation::signature::parameterbuilder::{OperationParameterBuilder, ParameterBuilderError};
use crate::operation::user_defined::{AbstractNodeId, AbstractOperationResultMarker, Instruction, NamedMarker, UserDefinedOperation};

use error_stack::Result;

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("Unexpected instruction")]
    UnexpectedInstruction,
    #[error("Failed to build operation parameter")]
    ParameterBuildError,
    #[error("Failed with outer error")]
    OutsideError,
    #[error("todo: {0}")]
    NeedsSpecificVariant(&'static str),
}

struct BuildingParameterFrame<S: Semantics> {
    parameter_builder: OperationParameterBuilder<S>,
}

impl<S: Semantics> BuildingParameterFrame<S> {
    fn new() -> Self {
        BuildingParameterFrame {
            parameter_builder: OperationParameterBuilder::new(),
        }
    }

    fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut BuildingParameterFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();

        match instruction {
            BI::ExpectParameterNode(marker, av) => {
                this.parameter_builder.expect_explicit_input_node(marker, av)
                    .change_context(BuilderError::ParameterBuildError)?;
            }
            BI::ExpectContextNode(marker, av) => {
                this.parameter_builder.expect_context_node(marker, av).change_context(BuilderError::ParameterBuildError)?;
            }
            BI::ExpectParameterEdge(src, dst, edge) => {
                this.parameter_builder.expect_edge(src, dst, edge).change_context(BuilderError::ParameterBuildError)?;
            }
            _ => {
                // The user has decided that they're done building the parameter by sending a different instruction
                // restore instruction so we can continue
                let _ = instruction_opt.insert(instruction);

                let this: BuildingParameterFrame<S> = builder.stack.expect_pop();
                let parameter = this.parameter_builder.build().change_context(BuilderError::ParameterBuildError)?;
                let frame = CollectingInstructionsFrame::from_param(&parameter);
                builder.data.built.parameter = Some(parameter);

                builder.push_frame(frame);
            },
        };
        Ok(())
    }
}

struct CollectingInstructionsFrame<S: Semantics> {
    instructions: UDInstructionsWithMarker<S>,
    current_state: IntermediateState<S>,
}

impl<S: Semantics> CollectingInstructionsFrame<S> {
    pub fn from_param(
        parameter: &OperationParameter<S>,
    ) -> Self {
        CollectingInstructionsFrame {
            instructions: vec![],
            current_state: IntermediateState::from_param(parameter),
        }
    }

    pub fn from_state(
        state: IntermediateState<S>,
    ) -> Self {
        CollectingInstructionsFrame {
            instructions: vec![],
            current_state: state,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut CollectingInstructionsFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            // We handle these ourselves
            BI::AddOperation(builder_op_like, args) => {
                this.handle_operation(&mut builder.data, None, builder_op_like, args)?;
            }
            BI::AddNamedOperation(output_name, builder_op_like, args) => {
                this.handle_operation(&mut builder.data, Some(output_name), builder_op_like, args)?;
            }
            // We enter a new context
            BI::StartQuery(..) => {
                let query_frame = QueryFrame::new(&this.current_state, instruction)?;

                // but the new query frame is on top
                builder.push_frame(query_frame);
            }
            // need to handle instructions that change the branch - endquery, entertrue, enterfalse
            BI::EnterFalseBranch | BI::EnterTrueBranch | BI::EndQuery => {
                // our frame needs to somehow be passed to the query frame that should be one below us.
                // I guess this is a "push" (vs pull) model, where we now access the query frame below us and push the data?
                // TODO: make this actually be a result? the expect_...
                let our_frame: CollectingInstructionsFrame<S> = builder.stack.expect_pop();

                // TODO: actually, we need to be able to push to both query and shape query frames.
                //  so need to dynamically dispatch here.

                // hmm. Instead of pushing explicitly, what if we had a data_stack where we could push our frame?
                // then reset the instruction, and the main builder loop would do the dynamic dispatch to the correct frame, which could
                // then consume from the data stack (conditionally if it expects something).

                // TODO: what to do if below returns an error?
                //  we would lose the frame and all the instructions in it!
                //  for this case, solved by having explicit data_stack.
                QueryFrame::push_branch(
                    builder,
                    our_frame,
                )?;

                // put instruction back, since we want QueryFrame to take over
                let _ = instruction_opt.insert(instruction);
            }
            BI::RenameNode(old_aid, new_name) => {
                // don't allow renaming ParameterMarker nodes
                if let AbstractNodeId::ParameterMarker(_) = old_aid {
                    bail!(BuilderError::NeedsSpecificVariant("cannot rename parameter"));
                }
                let new_aid = AbstractNodeId::named(new_name);
                this.current_state.rename_aid(old_aid, new_aid)
                    .change_context(BuilderError::OutsideError)?;

                this.instructions.push(
                    (
                        None,
                            Instruction::RenameNode {
                                old: old_aid,
                                new: new_aid,
                            }
                        )
                )
            }
            BI::ReturnNode(..) => {
                // todo!()
            }
            BI::StartShapeQuery(..) => {
                // todo!()
            }
            _ => {
                // put it back - actually no. should leave it out? since we haven't changed the frame, and thus we'd just get called again and again.
                // actually, it doesn't matter, since we return an error.
                let err = Err(report!(BuilderError::UnexpectedInstruction))
                    .attach_printable_lazy(|| {
                        format!("Unexpected instruction in CollectingInstructionsFrame: {:?}", &instruction)
                    });
                let _ = instruction_opt.insert(instruction);
                return err;
            },
        }

        Ok(())
    }

    pub fn handle_operation(
        &mut self,
        builder: &mut BuilderData<S>,
        output_name: Option<AbstractOperationResultMarker>,
        op_like: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), BuilderError> {
        let output_name_forced_marker = output_name.unwrap_or_else(|| {
            // TODO: better? do we even really need implicit?
            AbstractOperationResultMarker::Implicit(0)
        });

        // TODO: get an actual recursion op
        //  hmm. Maybe we could do this by having a running signature (well, AbstractOutputChanges), and OperationParameter?
        //  that's all that's needed for apply_abstract. And it doesn't require clone on operations!
        let self_op_unfinished = UserDefinedOperation::new_noop();
        let op = op_like.as_operation(builder.op_ctx, &self_op_unfinished)
            .change_context(BuilderError::OutsideError)?;
        let abstract_arg = self.current_state.interpret_op(builder.op_ctx, output_name_forced_marker, op, args)
            .change_context(BuilderError::OutsideError)?;

        // TODO: pass self_op_id
        let op_like_instr = op_like.to_op_like_instruction(0);

        self.instructions.push(
            (
                output_name,
                Instruction::OpLike(op_like_instr, abstract_arg)
            )
        );

        Ok(())
    }
}

struct QueryFrame<S: Semantics> {
    query: S::BuiltinQuery,
    before_branches_state: IntermediateState<S>,
    true_instructions: Option<CollectingInstructionsFrame<S>>,
    false_instructions: Option<CollectingInstructionsFrame<S>>,
    currently_entered_branch: Option<bool>, // true for true branch, false for false branch
}

impl<S: Semantics> QueryFrame<S> {
    pub fn new(outer_state: &IntermediateState<S>, instruction: BuilderInstruction<S>) -> Result<Self, BuilderError> {
        use BuilderInstruction as BI;

        match instruction {
            BI::StartQuery(query, args) => {
                let mut before_branches_state = outer_state.clone();
                // TODO: apply the query's abstract effects to this state



                let frame = QueryFrame {
                    query,
                    before_branches_state,
                    true_instructions: None,
                    false_instructions: None,
                    currently_entered_branch: None,
                };

                Ok(frame)
            },
            _ => {
                Err(report!(BuilderError::UnexpectedInstruction))
                    .attach_printable_lazy(|| {
                        format!("Expected StartQuery, got: {:?}", instruction)
                    })
            },
        }
    }

    fn push_branch(
        builder: &mut Builder<S>,
        frame: CollectingInstructionsFrame<S>,
    ) -> Result<(), BuilderError> {
        let this: &mut QueryFrame<S> = builder.stack.expect_mut();

        // We just finished a branch and got `frame` as result.

        if let Some(branch) = this.currently_entered_branch {
            if branch {
                if this.true_instructions.is_some() {
                    // should not happen
                    bail!(BuilderError::NeedsSpecificVariant("true branch already entered"));
                }
                this.true_instructions = Some(frame);
            } else {
                if this.false_instructions.is_some() {
                    // should not happen
                    bail!(BuilderError::NeedsSpecificVariant("false branch already entered"));
                }
                this.false_instructions = Some(frame);
            }
        } else {
            // We are not in a branch, this hsould not happen
            bail!(BuilderError::NeedsSpecificVariant("not in a branch, but trying to push branch instructions"));
        }
        this.currently_entered_branch = None; // reset, since we just pushed the branch

        Ok(())
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut QueryFrame<S> = builder.stack.expect_mut();

        // We accept: EnterTrue, EnterFalse, and EndQuery.

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::EnterTrueBranch => {
                if this.true_instructions.is_some() {
                    bail!(BuilderError::NeedsSpecificVariant("true branch already entered"));
                }
                // We enter the true branch
                let true_frame = CollectingInstructionsFrame::from_state(this.before_branches_state.clone());
                this.currently_entered_branch = Some(true);
                builder.push_frame(true_frame);
            }
            BI::EnterFalseBranch => {
                if this.false_instructions.is_some() {
                    bail!(BuilderError::NeedsSpecificVariant("false branch already entered"));
                }
                // We enter the false branch
                let false_frame = CollectingInstructionsFrame::from_state(this.before_branches_state.clone());
                this.currently_entered_branch = Some(false);
                builder.push_frame(false_frame);
            }
            BI::EndQuery => {
                // We finish the query, and give the outer frame all our information.
                let query_frame: QueryFrame<S> = builder.stack.expect_pop();
                query_frame.handle_query_end(builder)?;
            }
            _ => {
                bail!(BuilderError::UnexpectedInstruction);
            }
        }


        Ok(())
    }

    fn handle_query_end(
        self,
        builder: &mut Builder<S>,
    ) -> Result<(), BuilderError> {
        assert!(self.currently_entered_branch.is_none());

        // we need to handle everything that happens at the end of a query frame - i.e., merging states

        let true_branch_state_ref = self.true_instructions.as_ref().map(|cif| &cif.current_state).unwrap_or(&self.before_branches_state);
        let false_branch_state_ref = self.false_instructions.as_ref().map(|cif| &cif.current_state).unwrap_or(&self.before_branches_state);
        let merged_branch = merge_states(false, true_branch_state_ref, false_branch_state_ref);

        let outer_frame: &mut CollectingInstructionsFrame<S> = builder.stack.expect_mut();
        outer_frame.current_state = merged_branch;

        Ok(())
    }
}

#[derive(From, TryInto)]
#[try_into(owned, ref, ref_mut)]
enum Frame<S: Semantics> {
    BuildingParameter(BuildingParameterFrame<S>),
    CollectingInstructions(CollectingInstructionsFrame<S>),
    Query(QueryFrame<S>),
}

struct FrameStack<S: Semantics> {
    frames: Vec<Frame<S>>,
}

impl<S: Semantics> FrameStack<S> {
    pub fn new() -> Self {
        FrameStack {
            frames: vec![Frame::BuildingParameter(BuildingParameterFrame::new())],
        }
    }

    pub fn push(&mut self, frame: impl Into<Frame<S>>) {
        self.frames.push(frame.into());
    }

    pub fn pop(&mut self) -> Option<Frame<S>> {
        self.frames.pop()
    }

    pub fn last(&self) -> Option<&Frame<S>> {
        self.frames.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut Frame<S>> {
        self.frames.last_mut()
    }

    pub fn expect_mut<'a, F>(&'a mut self) -> F
    where S: 'a,
    &'a mut Frame<S>: TryInto<F>
    {
        let last = self.frames.last_mut().unwrap();
        last.try_into().ok().unwrap()
    }

    pub fn expect_ref<'a, F>(&'a self) -> F
    where S: 'a,
          &'a Frame<S>: TryInto<F>
    {
        let last = self.frames.last().unwrap();
        last.try_into().ok().unwrap()
    }

    pub fn expect_pop<F>(&mut self) -> F
    where Frame<S>: TryInto<F>
    {
        let last = self.frames.pop().unwrap();
        last.try_into().ok().unwrap()
    }
}

struct BuiltData<S: Semantics> {
    parameter: Option<OperationParameter<S>>,
    intermediate_state: Option<IntermediateState<S>>,
}

impl<S: Semantics> BuiltData<S> {
    pub fn new() -> Self {
        BuiltData {
            parameter: None,
            intermediate_state: None,
        }
    }

    pub fn provide_parameter(&mut self, parameter: OperationParameter<S>) {
        self.intermediate_state = Some(IntermediateState::from_param(&parameter));
        self.parameter = Some(parameter);
    }
}

struct BuilderData<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    self_op_id: OperationId,
    built: BuiltData<S>,
}

impl<'a, S: Semantics> BuilderData<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        BuilderData {
            op_ctx,
            built: BuiltData::new(),
            self_op_id: 0,
        }
    }
}

pub struct Builder<'a, S: Semantics> {
    data: BuilderData<'a, S>,
    stack: FrameStack<S>,
}


impl<'a, S: Semantics> Builder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Builder {
            data: BuilderData::new(op_ctx),
            stack: FrameStack::new(),
        }
    }

    pub fn show(&self) -> BuilderShowData<S> {
        match self.stack.last() {
            Some(Frame::BuildingParameter(frame)) => {
                BuilderShowData::ParameterBuilder(&frame.parameter_builder)
            }
            Some(Frame::CollectingInstructions(frame)) => {
                BuilderShowData::CollectingInstructions(&frame.current_state)
            }
            Some(Frame::Query(_)) => {
                BuilderShowData::QueryFrame()
            }
            None => {
                BuilderShowData::Other("No frame".to_string())
            }
        }
    }

    /// Note: Should only be called before issuing recursion instructions.
    pub fn update_self_op_id(&mut self, self_op_id: OperationId) {
        self.data.self_op_id = self_op_id;
    }

    pub fn consume(&mut self, instruction: BuilderInstruction<S>) -> Result<(), BuilderError> {
        let mut instruction_opt = Some(instruction);

        while instruction_opt.is_some() {
            // TODO: don't pop
            let curr_frame = self.stack.last().unwrap();
            match curr_frame {
                Frame::BuildingParameter(..) => {
                    BuildingParameterFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::CollectingInstructions(..) => {
                    CollectingInstructionsFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::Query(..) => {
                    QueryFrame::consume(self, &mut instruction_opt)?;
                }
            }
        }

        Ok(())
    }

    fn build(&mut self) -> Result<UserDefinedOperation<S>, BuilderError> {
        while self.stack.frames.len() > 1 {
            self.consume(BuilderInstruction::EndQuery)?;
        }

        let frame: CollectingInstructionsFrame<S> = self.stack.expect_pop();

        let mut user_def_op = UserDefinedOperation::new_noop();
        user_def_op.instructions = frame.instructions;
        user_def_op.parameter = self.data.built.parameter.clone().unwrap();
        Ok(user_def_op)
    }

    fn push_frame(&mut self, frame: impl Into<Frame<S>>) {
        self.stack.push(frame.into());
    }

}

pub enum BuilderShowData<'a, S: Semantics> {
    ParameterBuilder(&'a OperationParameterBuilder<S>),
    CollectingInstructions(&'a IntermediateState<S>),
    QueryFrame(),
    Other(String),
}

impl<'a, S: Semantics<NodeAbstract: Debug, EdgeAbstract: Debug>> Debug for BuilderShowData<'a, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BuilderShowData::ParameterBuilder(param_builder) => {
                write!(f, "ParameterBuilder: {:?}", (*param_builder).clone().build().unwrap().parameter_graph.shape_dot())
            }
            BuilderShowData::CollectingInstructions(state) => {
                write!(f, "CollectingInstructions: {}", state.dot_with_aid())
            }
            BuilderShowData::QueryFrame() => {
                write!(f, "QueryFrame")
            }
            BuilderShowData::Other(data) => {
                write!(f, "Other: {}", data)
            }
        }
    }
}


pub struct OperationBuilder2<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    instructions: Vec<BuilderInstruction<S>>,
    active: Builder<'a, S>,
}

impl<'a, S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> OperationBuilder2<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Self {
            instructions: Vec::new(),
            op_ctx,
            active: Builder::new(op_ctx),
        }
    }

    pub fn undo_last_instruction(&mut self) {
        if !self.instructions.is_empty() {
            self.instructions.pop();
        }
        self.rebuild_active_from_instructions()
    }

    pub fn rename_node(
        &mut self,
        old_aid: AbstractNodeId,
        new_name: impl Into<NamedMarker>,
    ) -> Result<(), OperationBuilderError> {
        let new_name = new_name.into();
        self.push_instruction(BuilderInstruction::RenameNode(old_aid, new_name))
    }

    pub fn expect_parameter_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let marker = marker.into();
        self.push_instruction(BuilderInstruction::ExpectParameterNode(marker, node))
    }

    pub fn expect_context_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let marker = marker.into();
        self.push_instruction(BuilderInstruction::ExpectContextNode(marker, node))
    }

    pub fn expect_parameter_edge(
        &mut self,
        source_marker: impl Into<SubstMarker>,
        target_marker: impl Into<SubstMarker>,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let source_marker = source_marker.into();
        let target_marker = target_marker.into();
        self.push_instruction(BuilderInstruction::ExpectParameterEdge(
                source_marker,
                target_marker,
                edge,
            ))
    }

    pub fn start_query(
        &mut self,
        query: S::BuiltinQuery,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::StartQuery(query, args))
    }

    pub fn enter_true_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::EnterTrueBranch)
    }

    pub fn enter_false_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::EnterFalseBranch)
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
        self.push_instruction(BuilderInstruction::StartShapeQuery(op_marker.into()))
    }

    pub fn end_query(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::EndQuery)
    }

    // TODO: should expect_*_node really expect a marker? maybe it should instead return a marker?
    //  it could also take an Option<Marker> so that it can autogenerate one if it's none so the caller doesn't have to deal with it.
    pub fn expect_shape_node(
        &mut self,
        marker: AbstractOutputNodeMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO: check that any shape nodes are not free floating. maybe this should be in a GraphShapeQuery validator?
        self.push_instruction(BuilderInstruction::ExpectShapeNode(marker, node))
    }

    pub fn expect_shape_node_change(
        &mut self,
        aid: AbstractNodeId,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::ExpectShapeNodeChange(aid, node))
    }

    pub fn expect_shape_edge(
        &mut self,
        source: AbstractNodeId,
        target: AbstractNodeId,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO:
        self.push_instruction(BuilderInstruction::ExpectShapeEdge(source, target, edge))
    }

    pub fn add_named_operation(
        &mut self,
        name: AbstractOperationResultMarker,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // TODO
        self.push_instruction(BuilderInstruction::AddNamedOperation(name, op, args))
    }

    pub fn add_operation(
        &mut self,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::AddOperation(op, args))
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
        self.push_instruction(BuilderInstruction::ReturnNode(aid, output_marker, node))
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
        self.push_instruction(BuilderInstruction::ReturnEdge(src, dst, edge))
    }

    // TODO: This should run further post processing checks.
    //  Stuff like Context nodes must be connected, etc.
    pub fn build(
        &mut self,
        self_op_id: OperationId,
    ) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        let res = self.active.build();
        if let Ok(op) = res {
            return Ok(op);
        }

        // if we failed, we need to rebuild the active builder from the instructions
        self.rebuild_active_from_instructions();
        res.change_context(OperationBuilderError::NewBuilderError)
    }

    fn push_instruction(&mut self, instruction: BuilderInstruction<S>) -> Result<(), OperationBuilderError> {
        self.instructions.push(instruction.clone());
        let res =self.active.consume(instruction).change_context(OperationBuilderError::NewBuilderError);
        if res.is_err() {
           // rollback
            self.instructions.pop();
            self.rebuild_active_from_instructions();
        }

        res
    }

    fn rebuild_active_from_instructions(&mut self) {
        self.active = Builder::new(self.op_ctx);
        for instruction in &self.instructions {
            self.active.consume(instruction.clone()).expect("internal error: should not fail to consume previously fine instruction");
        }
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
> OperationBuilder2<'a, S> {
    pub fn show_state(&self) -> Result<IntermediateState<S>, OperationBuilderError> {
        let mut inner = self.active.show();
        if let BuilderShowData::CollectingInstructions(state) = &mut inner {
            // we have a state, so we can return it
            Ok(state.clone())
        } else {
            // TODO: improve
            Err(report!(OperationBuilderError::NewBuilderError))
        }
    }

    pub fn format_state(&self) -> String {
        let mut inner = self.active.show();
        format!("{:?}", inner)
    }
}