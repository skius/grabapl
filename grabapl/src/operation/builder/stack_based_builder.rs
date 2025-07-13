use std::fmt::{Debug, Formatter};
use derive_more::From;
use derive_more::with_trait::TryInto;
use error_stack::{bail, report, Report, ResultExt};
use thiserror::Error;
use crate::{OperationContext, Semantics};
use crate::operation::builder::{merge_states, BuilderInstruction, BuilderOpLike, IntermediateState, OperationBuilderError, UDInstructionsWithMarker};
use crate::operation::signature::parameter::OperationParameter;
use crate::operation::signature::parameterbuilder::{OperationParameterBuilder, ParameterBuilderError};
use crate::operation::user_defined::{AbstractNodeId, AbstractOperationResultMarker, Instruction, UserDefinedOperation};

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
            BI::StartQuery(ref query, ..) => {
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
                todo!()
            }
            BI::StartShapeQuery(..) => {
                todo!()
            }
            _ => {
                // put it back - actually no. should leave it out? since we haven't changed the frame, and thus we'd just get called again and again.
                // actually, it doesn't matter, since we return an error.
                let _ = instruction_opt.insert(instruction);
                bail!(BuilderError::UnexpectedInstruction)
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
    BuildingParameter(OperationParameterBuilder<S>),
    CollectingInstructions(CollectingInstructionsFrame<S>),
    Query(QueryFrame<S>),
}

struct FrameStack<S: Semantics> {
    frames: Vec<Frame<S>>,
}

impl<S: Semantics> FrameStack<S> {
    pub fn new() -> Self {
        FrameStack {
            frames: vec![Frame::BuildingParameter(OperationParameterBuilder::new())],
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
    built: BuiltData<S>,
}

impl<'a, S: Semantics> BuilderData<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        BuilderData {
            op_ctx,
            built: BuiltData::new(),
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
            Some(Frame::BuildingParameter(param_builder)) => {
                BuilderShowData::ParameterBuilder(param_builder)
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

    pub fn consume(&mut self, instruction: BuilderInstruction<S>) -> Result<(), BuilderError> {
        let mut instruction_opt = Some(instruction);

        while instruction_opt.is_some() {
            // TODO: don't pop
            let curr_frame = self.stack.last().unwrap();
            match curr_frame {
                Frame::BuildingParameter(..) => {
                    self.consume_for_building_parameter(&mut instruction_opt)?;
                }
                Frame::CollectingInstructions(..) => {
                    CollectingInstructionsFrame::consume(self, &mut instruction_opt)?;
                    // self.consume_for_collecting_instructions(&mut instruction_opt, frame)?;
                }
                Frame::Query(..) => {
                    QueryFrame::consume(self, &mut instruction_opt)?;
                    // query_frame.consume(self, &mut instruction_opt)?;
                }
            }
        }

        Ok(())
    }

    fn push_frame(&mut self, frame: impl Into<Frame<S>>) {
        self.stack.push(frame.into());
    }

    fn consume_for_building_parameter(
        &mut self,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let mut param_builder: OperationParameterBuilder<S> = self.stack.expect_pop();

        let instruction = instruction_opt.as_ref().unwrap();

        let next_frame = match instruction {
            // TODO: ugly double match.
            //  instead: have BuilderInstruction have eg a .is_for_parameter() method.
            BI::ExpectParameterNode(..) | BI::ExpectContextNode(..) | BI::ExpectParameterEdge(..) => {
                // consume instruction
                let instruction = instruction_opt.take().unwrap();
                match instruction {
                    BI::ExpectParameterNode(marker, av) => {
                        param_builder.expect_explicit_input_node(marker, av)
                            .change_context(BuilderError::ParameterBuildError)?;
                        Frame::BuildingParameter(param_builder)
                    }
                    BI::ExpectContextNode(marker, av) => {
                        param_builder.expect_context_node(marker, av).change_context(BuilderError::ParameterBuildError)?;
                        Frame::BuildingParameter(param_builder)
                    }
                    BI::ExpectParameterEdge(src, dst, edge) => {
                        param_builder.expect_edge(src, dst, edge).change_context(BuilderError::ParameterBuildError)?;
                        Frame::BuildingParameter(param_builder)
                    }
                    _ => unreachable!("we just checked that this matches"),
                }
            }
            _ => {
                // The user has decided that they're done building the parameter by sending a different instruction
                let parameter = param_builder.build().change_context(BuilderError::ParameterBuildError)?;
                let frame = CollectingInstructionsFrame::from_param(&parameter);
                self.data.built.parameter = Some(parameter);

                Frame::CollectingInstructions(frame)
            },
        };
        self.stack.push(next_frame);
        Ok(())
    }

    // fn consume_for_collecting_instructions(
    //     &mut self,
    //     instruction_opt: &mut Option<BuilderInstruction<S>>,
    //     mut frame: CollectingInstructionsFrame<S>,
    // ) -> Result<(), BuilderError> {
    //     frame.consume(self, instruction_opt)
    // }
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