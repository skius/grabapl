use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use derive_more::From;
use derive_more::with_trait::TryInto;
use error_stack::{bail, report, Report, ResultExt};
use thiserror::Error;
use crate::{NodeKey, OperationContext, OperationId, Semantics, SubstMarker};
use crate::operation::builder::{merge_states, BuilderInstruction, BuilderOpLike, IntermediateInterpreter, IntermediateState, IntermediateStateBuilder, OperationBuilderInefficient, OperationBuilderError, UDInstructionsWithMarker};
use crate::operation::signature::parameter::{AbstractOutputNodeMarker, OperationParameter};
use crate::operation::signature::parameterbuilder::{OperationParameterBuilder, ParameterBuilderError};
use crate::operation::user_defined::{AbstractNodeId, AbstractOperationArgument, AbstractOperationResultMarker, AbstractUserDefinedOperationOutput, Instruction, NamedMarker, QueryInstructions, UserDefinedOperation};

use error_stack::Result;
use crate::operation::query::{GraphShapeQuery, ShapeNodeIdentifier};
use crate::operation::signature::{AbstractOutputChanges, AbstractSignatureNodeId, OperationSignature};
use crate::semantics::{AbstractGraph, AbstractMatcher};
use crate::util::bimap::BiMap;

macro_rules! bail_unexpected_instruction {
    ($i:expr, $i_opt:expr, $frame:literal) => {
        let err = Err(report!(BuilderError::UnexpectedInstruction))
            .attach_printable_lazy(|| {
                format!("Unexpected instruction in {}: {:?}", $frame, $i)
            });
        let _ = $i_opt.insert($i);
        return err;
    };
}

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
            instruction if instruction.can_break_body() => {
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
                // QueryFrame::push_branch(
                //     builder,
                //     our_frame,
                // )?;

                // data_stack is the return_stack
                builder.return_stack.push(our_frame);

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
            BI::StartShapeQuery(op_result_marker) => {
                // we start a new BuildingShapeQueryFrame
                let initial_state = this.current_state.clone();
                let shape_query_frame = BuildingShapeQueryFrame::new(op_result_marker, initial_state);
                // push it onto the stack
                builder.push_frame(shape_query_frame);
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

// TODO: could have a BranchesFrame that is on top of both QueryFrame and ShapeQueryFrame that exclusively handles
//  EnterTrue/False,EndQuery, and then pushes itself onto the return_stack, to be handled by the outer frame.
//  That way we don't have duplicate code for gsq/query frames.
struct QueryFrame<S: Semantics> {
    query: S::BuiltinQuery,
    abstract_arg: AbstractOperationArgument,
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
                let abstract_arg = before_branches_state.interpret_builtin_query(&query, args)
                    .change_context(BuilderError::OutsideError)?;

                let frame = QueryFrame {
                    query,
                    abstract_arg,
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

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut QueryFrame<S> = builder.stack.expect_mut();

        if let Some(branch) = this.currently_entered_branch && builder.return_stack.top_is::<CollectingInstructionsFrame<S>>() {
            // TODO: is there a situation where this.currently_entered_branch is None but we have a branch frame?

            let branch_frame: CollectingInstructionsFrame<S> = builder.return_stack.expect_pop();
            if branch {
                this.true_instructions = Some(branch_frame);
            } else {
                this.false_instructions = Some(branch_frame);
            }

            this.currently_entered_branch = None;
        }

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
        // push ourselves as instruction
        let query_instructions = QueryInstructions {
            taken: self.true_instructions.map(|cif| cif.instructions).unwrap_or_default(),
            not_taken: self.false_instructions.map(|cif| cif.instructions).unwrap_or_default(),
        };
        outer_frame.instructions.push((
            None,
            Instruction::BuiltinQuery(self.query,
                self.abstract_arg,
                query_instructions
            ),
        ));

        Ok(())
    }
}

/// This frame's entire purpose is to create the final return frame once the data is available.
struct WrapperReturnFrame<S: Semantics> {
    phantom: PhantomData<S>,
}

impl<S: Semantics> WrapperReturnFrame<S> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        let _: WrapperReturnFrame<S> = builder.stack.expect_pop();
        // We need to create the ReturnFrame from the current state and the return stack.
        let instr_frame: CollectingInstructionsFrame<S> = builder.return_stack.expect_pop();
        let return_frame = ReturnFrame::new(&builder.data, instr_frame);
        builder.stack.push(return_frame);

        Ok(())
    }
}

struct ReturnFrame<S: Semantics> {
    instr_frame: CollectingInstructionsFrame<S>,
    signature: OperationSignature<S>,
    abstract_ud_output: AbstractUserDefinedOperationOutput,
    // TODO: remove these?
    return_nodes: HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
    return_edges: HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,

}

impl<S: Semantics> ReturnFrame<S> {
    pub fn new(data: &BuilderData<S>, cif: CollectingInstructionsFrame<S>) -> Self {
        let mut signature = OperationSignature::empty_new("some_name", data.built.parameter.clone().unwrap());
        populate_signature_changes(&mut signature, &cif.current_state);

        ReturnFrame {
            instr_frame: cif,
            signature,
            abstract_ud_output: AbstractUserDefinedOperationOutput::new(),
            return_nodes: HashMap::new(),
            return_edges: HashMap::new(),
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut ReturnFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::ReturnNode(aid, output_marker, node) => {
                this.include_return_node(
                    aid,
                    output_marker,
                    node,
                )?;
            }
            BI::ReturnEdge(src, dst, edge) => {
                this.include_return_edge(
                    src,
                    dst,
                    edge,
                )?;
            }
            BI::Finalize => {
                // nothing for now, just consume.
                // TODO
                // In future: maybe push self onto return stack?
                // maybe switch type of return stack to a different enum.
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "ReturnFrame");
            },
        }

        Ok(())
    }

    fn include_return_node(
        &mut self,
        aid: AbstractNodeId,
        output_marker: AbstractOutputNodeMarker,
        av: S::NodeAbstract,
    ) -> Result<(), BuilderError> {
        if let AbstractNodeId::ParameterMarker(_) = aid {
            bail!(BuilderError::NeedsSpecificVariant("cannot return parameter node"));
        }
        if !self.last_state().contains_aid(&aid) {
            bail!(BuilderError::NeedsSpecificVariant("aid not found"));
        }
        if self.return_nodes.contains_key(&aid) {
            bail!(BuilderError::NeedsSpecificVariant("return node already exists"));
        }
        // if the user wants to return the node as an `av`, `av` must be a supertype of the inferred type
        let inferred_av = self.last_state().node_av_of_aid(&aid).unwrap();
        if !S::NodeMatcher::matches(inferred_av, &av) {
            bail!(BuilderError::NeedsSpecificVariant("cannot return node with incompatible abstract value"));
        }
        if self.last_state().node_may_originate_from_shape_query.contains(&aid) {
            bail!(BuilderError::NeedsSpecificVariant("cannot return node that originates from a shape query"));
        }

        self.abstract_ud_output.new_nodes.insert(aid, output_marker);
        self.signature.output.new_nodes.insert(
            output_marker,
            av.clone(),
        );
        self.return_nodes.insert(aid, (output_marker, av));
        Ok(())
    }

    fn include_return_edge(
        &mut self,
        src: AbstractNodeId,
        dst: AbstractNodeId,
        av: S::EdgeAbstract,
    ) -> Result<(), BuilderError> {
        // TODO: need to check validity here
        if !self.last_state().contains_edge(&src, &dst) {
            bail!(BuilderError::NeedsSpecificVariant("edge not found"));
        }
        if self.return_edges.contains_key(&(src, dst)) {
            bail!(BuilderError::NeedsSpecificVariant("return edge already exists"));
        }
        if !self.last_state().contains_aid(&src) {
            bail!(BuilderError::NeedsSpecificVariant("src aid not found"));
        }
        if !self.last_state().contains_aid(&dst) {
            bail!(BuilderError::NeedsSpecificVariant("dst aid not found"));
        }
        // if the user wants to return the edge as an `av`, `av` must be a supertype of the inferred type
        let inferred_av = self.last_state().edge_av_of_aid(&src, &dst).unwrap();
        if !S::EdgeMatcher::matches(inferred_av, &av) {
            bail!(BuilderError::NeedsSpecificVariant("cannot return edge with incompatible abstract value"));
        }
        if self.last_state().edge_may_originate_from_shape_query.contains(&(src, dst)) {
            bail!(BuilderError::NeedsSpecificVariant("cannot return edge that originates from a shape query"));
        }

        let src_sig_id = self.aid_to_sig_id(&src).attach_printable_lazy(|| "cannot use source node in signature")?;
        let dst_sig_id = self.aid_to_sig_id(&dst).attach_printable_lazy(|| "cannot use destination node in signature")?;
        self.signature.output.new_edges.insert((src_sig_id, dst_sig_id), av.clone());

        self.return_edges.insert((src, dst), av);
        Ok(())
    }

    fn aid_to_sig_id(&self, aid: &AbstractNodeId) -> Result<AbstractSignatureNodeId, BuilderError> {
        match *aid {
            AbstractNodeId::ParameterMarker(s) => Ok(AbstractSignatureNodeId::ExistingNode(s)),
            AbstractNodeId::DynamicOutputMarker(_, _) | AbstractNodeId::Named(..) => {
                // we must be returning this node if we want to return an incident edge.
                let Some((output_marker, _)) = self.return_nodes.get(aid) else {
                    bail!(BuilderError::NeedsSpecificVariant("node not returned"));
                };
                Ok(AbstractSignatureNodeId::NewNode(output_marker.clone()))
            }
        }
    }

    fn last_state(&self) -> &IntermediateState<S> {
        &self.instr_frame.current_state
    }
}

/// This frame is used to build a shape query, i.e., everything before the first EnterXBranch/EndQuery instruction.
// TODO: dont need param_builder, since we can just pretend the state from initial_state is the parameter graph.
//  this also allows us to directly extend the expected_graph with guarantees on having the same node keys.
struct BuildingShapeQueryFrame<S: Semantics> {
    /// The parameter used to define the input of the GraphShapeQuery
    parameter: OperationParameter<S>,
    /// The arguments for the resulting GraphShapeQuery
    abstract_arg: AbstractOperationArgument,
    gsq_node_keys_to_shape_idents: BiMap<NodeKey, ShapeNodeIdentifier>,
    query_marker: AbstractOperationResultMarker,
    initial_state: IntermediateState<S>,
    /// Holds the state if the shape query matches.
    /// This is simultaneously the expected graph of the shape query.
    true_branch_state: IntermediateState<S>,
}

impl<S: Semantics> BuildingShapeQueryFrame<S> {
    pub fn new(query_marker: AbstractOperationResultMarker, initial_state: IntermediateState<S>) -> Self {
        let true_branch_state = initial_state.clone();
        let (parameter, abstract_arg) = initial_state.as_param_for_shape_query();
        BuildingShapeQueryFrame {
            parameter,
            abstract_arg,
            query_marker,
            gsq_node_keys_to_shape_idents: BiMap::new(),
            initial_state,
            true_branch_state,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut BuildingShapeQueryFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::ExpectShapeNode(marker, av) => {
                let aid = AbstractNodeId::dynamic_output(this.query_marker, marker);
                let sni: ShapeNodeIdentifier = marker.0.into();
                // return error if we already encountered this key before
                if this.gsq_node_keys_to_shape_idents.contains_right(&sni) {
                    bail!(BuilderError::NeedsSpecificVariant("shape node already exists"));
                }

                this.true_branch_state.add_node(aid, av, true);
                this.gsq_node_keys_to_shape_idents.insert(this.true_branch_state.get_key_from_aid(&aid).unwrap(), sni);
            }
            BI::ExpectShapeNodeChange(aid, new_av) => {
                this.true_branch_state.set_node_av(aid, new_av)
                    .change_context(BuilderError::OutsideError)?;
            }
            BI::ExpectShapeEdge(src, dst, edge) => {
                this.true_branch_state.add_edge(
                    src,
                    dst,
                    edge,
                    true,
                ).change_context(BuilderError::OutsideError)?;

            }
            instruction if instruction.can_break_body() => {
                // Advance to BuiltShapeQueryFrame
                // it needs to consume this instruction
                let _ = instruction_opt.insert(instruction);
                let this: BuildingShapeQueryFrame<S> = builder.stack.expect_pop();
                let built_frame = this.into_built_shape_query_frame(builder)?;
                builder.push_frame(built_frame);
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "BuildingShapeQueryFrame");
            },
        }

        Ok(())
    }

    fn into_built_shape_query_frame(self, builder: &mut Builder<S>) -> Result<BuiltShapeQueryFrame<S>, BuilderError> {
        // We build the parameter and the initial state

        let query = GraphShapeQuery {
            parameter: self.parameter,
            expected_graph: self.true_branch_state.graph.clone(),
            node_keys_to_shape_idents: self.gsq_node_keys_to_shape_idents,
        };

        Ok(BuiltShapeQueryFrame::new(self.query_marker, query, self.abstract_arg, self.initial_state, self.true_branch_state))
    }
}

/// This frame is used to handle the branches of a *built* shape query. It is the product of BuildingShapeQueryFrame.
struct BuiltShapeQueryFrame<S: Semantics> {
    query_marker: AbstractOperationResultMarker,
    query: GraphShapeQuery<S>,
    abstract_arg: AbstractOperationArgument,
    initial_false_branch_state: IntermediateState<S>,
    initial_true_branch_state: IntermediateState<S>,
    true_instructions: Option<CollectingInstructionsFrame<S>>,
    false_instructions: Option<CollectingInstructionsFrame<S>>,
    currently_entered_branch: Option<bool>, // true for true branch, false for false branch
}

impl<S: Semantics> BuiltShapeQueryFrame<S> {
    pub fn new(
        query_marker: AbstractOperationResultMarker,
        query: GraphShapeQuery<S>,
        abstract_arg: AbstractOperationArgument,
        initial_false_branch_state: IntermediateState<S>,
        initial_true_branch_state: IntermediateState<S>,
    ) -> Self {
        BuiltShapeQueryFrame {
            query_marker,
            query,
            abstract_arg,
            initial_false_branch_state,
            initial_true_branch_state,
            true_instructions: None,
            false_instructions: None,
            currently_entered_branch: None,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let this: &mut BuiltShapeQueryFrame<S> = builder.stack.expect_mut();

        if let Some(branch) = this.currently_entered_branch && builder.return_stack.top_is::<CollectingInstructionsFrame<S>>() {
            // TODO: is there a situation where this.currently_entered_branch is None but we have a branch frame?

            let branch_frame: CollectingInstructionsFrame<S> = builder.return_stack.expect_pop();
            if branch {
                this.true_instructions = Some(branch_frame);
            } else {
                this.false_instructions = Some(branch_frame);
            }

            this.currently_entered_branch = None;
        }

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::EnterTrueBranch => {
                if this.true_instructions.is_some() {
                    bail!(BuilderError::NeedsSpecificVariant("true branch already entered"));
                }
                // We enter the true branch
                let true_frame = CollectingInstructionsFrame::from_state(this.initial_true_branch_state.clone());
                this.currently_entered_branch = Some(true);
                builder.push_frame(true_frame);
            }
            BI::EnterFalseBranch => {
                if this.false_instructions.is_some() {
                    bail!(BuilderError::NeedsSpecificVariant("false branch already entered"));
                }
                // We enter the false branch
                let false_frame = CollectingInstructionsFrame::from_state(this.initial_false_branch_state.clone());
                this.currently_entered_branch = Some(false);
                builder.push_frame(false_frame);
            }
            // TODO: handle return node instruction here as well?
            BI::EndQuery | BI::Finalize => {
                // We finish the query, and give the outer frame all our information.
                let query_frame: BuiltShapeQueryFrame<S> = builder.stack.expect_pop();
                query_frame.handle_shape_query_end(builder)?;
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "BuiltShapeQueryFrame");
            },
        }

        Ok(())
    }

    fn handle_shape_query_end(
        self,
        builder: &mut Builder<S>,
    ) -> Result<(), BuilderError> {
        // we need to handle everything that happens at the end of a query frame - i.e., merging states

        // TODO: look at this code

        let true_branch_state_ref = self.true_instructions.as_ref().map(|cif| &cif.current_state).unwrap_or(&self.initial_true_branch_state);
        let false_branch_state_ref = self.false_instructions.as_ref().map(|cif| &cif.current_state).unwrap_or(&self.initial_false_branch_state);
        let merged_branch = merge_states(false, true_branch_state_ref, false_branch_state_ref);

        let outer_frame: &mut CollectingInstructionsFrame<S> = builder.stack.expect_mut();
        outer_frame.current_state = merged_branch;

        let query_instructions = QueryInstructions {
            taken: self.true_instructions.map(|cif| cif.instructions).unwrap_or_default(),
            not_taken: self.false_instructions.map(|cif| cif.instructions).unwrap_or_default(),
        };
        outer_frame.instructions.push((
            Some(self.query_marker),
            Instruction::ShapeQuery(
                self.query,
                self.abstract_arg,
                query_instructions,
            )
        ));


        Ok(())
    }
}

#[derive(From, TryInto)]
#[try_into(owned, ref, ref_mut)]
enum Frame<S: Semantics> {
    BuildingParameter(BuildingParameterFrame<S>),
    CollectingInstructions(CollectingInstructionsFrame<S>),
    Query(QueryFrame<S>),
    BuildingShapeQuery(BuildingShapeQueryFrame<S>),
    BuiltShapeQuery(BuiltShapeQueryFrame<S>),
    Return(ReturnFrame<S>),
    WrapperReturn(WrapperReturnFrame<S>),
}

struct FrameStack<S: Semantics> {
    frames: Vec<Frame<S>>,
}

impl<S: Semantics> FrameStack<S> {
    pub fn new_initial() -> Self {
        let mut stack = FrameStack::new_empty();
        stack.push(WrapperReturnFrame::new());
        stack.push(BuildingParameterFrame::new());
        stack
    }

    pub fn new_empty() -> Self {
        FrameStack {
            frames: vec![],
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

    pub fn top_is<'a, F>(&'a self) -> bool
    where S: 'a,
        F: 'a,
          &'a Frame<S>: TryInto<&'a F>
    {
        self.frames.last().map_or(false, |f| f.try_into().is_ok())
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
    return_stack: FrameStack<S>,
}


impl<'a, S: Semantics> Builder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Builder {
            data: BuilderData::new(op_ctx),
            stack: FrameStack::new_initial(),
            return_stack: FrameStack::new_empty(),
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
            Some(Frame::Query(frame)) => {
                BuilderShowData::QueryFrame(&frame.before_branches_state)
            }
            Some(Frame::Return(frame)) => {
                BuilderShowData::ReturnFrame(&frame.instr_frame.current_state)
            }
            Some(Frame::WrapperReturn(_)) => {
                // take data from return_stack
                // TODO: do we ever enter this path and not immediately consume? I dont think so.
                let instr_frame: &CollectingInstructionsFrame<S> = self.return_stack.expect_ref();
                BuilderShowData::ReturnFrame(&instr_frame.current_state)
            }
            Some(Frame::BuildingShapeQuery(frame)) => {
                BuilderShowData::ShapeQueryFrame(&frame.true_branch_state)
            }
            Some(Frame::BuiltShapeQuery(frame)) => {
                // TODO: do we ever enter this path even? is BuiltShapeQueryFrame ever not immediately consumed/processed?
                BuilderShowData::ShapeQueryFrame(&frame.initial_true_branch_state)
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
                Frame::Return(..) => {
                    ReturnFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::WrapperReturn(..) => {
                    WrapperReturnFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::BuildingShapeQuery(..) => {
                    BuildingShapeQueryFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::BuiltShapeQuery(..) => {
                    BuiltShapeQueryFrame::consume(self, &mut instruction_opt)?;
                }
            }
        }

        Ok(())
    }

    fn build(&mut self) -> Result<UserDefinedOperation<S>, BuilderError> {
        // this is a bit of a hack. it just works because all nested frames right now can be ended with Finalize.
        while self.stack.frames.len() > 1 {
            self.consume(BuilderInstruction::Finalize)?;
        }

        // let instr_frame: CollectingInstructionsFrame<S> = self.return_stack.expect_pop();
        let ret_frame: ReturnFrame<S> = self.stack.expect_pop();

        // let (output_changes, signature) = self.determine_signature(instr_frame.current_state, ret_frame)
        //     .change_context(BuilderError::OutsideError)?;

        let instr_frame = ret_frame.instr_frame;
        let output_changes = ret_frame.abstract_ud_output;
        let signature = ret_frame.signature;

        let mut user_def_op = UserDefinedOperation::new_noop();
        user_def_op.instructions = instr_frame.instructions;
        user_def_op.parameter = self.data.built.parameter.clone().unwrap();
        user_def_op.output_changes = output_changes;
        user_def_op.signature = signature;
        Ok(user_def_op)
    }

    fn push_frame(&mut self, frame: impl Into<Frame<S>>) {
        self.stack.push(frame.into());
    }

}

/// If signature contains the operation's parameter, then this function populates the signature's
/// output changes based on the difference between the parameter and the passed last state.
fn populate_signature_changes<S: Semantics>(
    signature: &mut OperationSignature<S>,
    last_state: &IntermediateState<S>,
) {
    let param = &signature.parameter;
    let initial_subst_nodes = param
        .node_keys_to_subst
        .right_values()
        .cloned()
        .collect::<HashSet<_>>();
    let current_subst_nodes = last_state
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
    for (source, target, _) in param.parameter_graph.graph.all_edges() {
        let Some(source_subst) = param.node_keys_to_subst.get_left(&source) else {
            continue; // should not happen, but just in case
        };
        let Some(target_subst) = param.node_keys_to_subst.get_left(&target) else {
            continue; // should not happen, but just in case
        };
        initial_edges.insert((*source_subst, *target_subst));
    }

    let mut current_edges = HashSet::new();
    for (source, target, _) in last_state.graph.graph.all_edges() {
        let Some(source_aid) = last_state.node_keys_to_aid.get_left(&source) else {
            continue; // should not happen, but just in case
        };
        let Some(target_aid) = last_state.node_keys_to_aid.get_left(&target) else {
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

    for (aid, node_abstract) in &last_state.node_may_be_written_to {
        // we care about reporting only subst markers
        let AbstractNodeId::ParameterMarker(subst) = aid else {
            continue;
        };
        signature
            .output
            .maybe_changed_nodes
            .insert(*subst, node_abstract.clone());
    }

    for ((source_aid, target_aid), edge_abstract) in &last_state.edge_may_be_written_to
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
}


pub enum BuilderShowData<'a, S: Semantics> {
    ParameterBuilder(&'a OperationParameterBuilder<S>),
    CollectingInstructions(&'a IntermediateState<S>),
    QueryFrame(&'a IntermediateState<S>),
    ShapeQueryFrame(&'a IntermediateState<S>),
    ReturnFrame(&'a IntermediateState<S>),
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
            BuilderShowData::QueryFrame(state) => {
                write!(f, "QueryFrame: {}", state.dot_with_aid())
            }
            BuilderShowData::ShapeQueryFrame(state) => {
                write!(f, "ShapeQueryFrame: {}", state.dot_with_aid())
            }
            BuilderShowData::ReturnFrame(state) => {
                write!(f, "ReturnFrame: {}", state.dot_with_aid())
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
        let inner = self.active.show();
        match inner {
            BuilderShowData::ParameterBuilder(param_builder) => {
                let param = param_builder.clone().build().unwrap();
                Ok(IntermediateState::from_param(&param))
            }
            BuilderShowData::CollectingInstructions(state) => {
                Ok(state.clone())
            }
            BuilderShowData::QueryFrame(state) => {
                Ok(state.clone())
            }
            BuilderShowData::ShapeQueryFrame(state) => {
                Ok(state.clone())
            }
            BuilderShowData::ReturnFrame(state) => {
                Ok(state.clone())
            }
            BuilderShowData::Other(_) => {
                Err(report!(OperationBuilderError::NewBuilderError)).attach_printable_lazy(||
                    format!("Expected CollectingInstructions state, got: {:?}", inner)
                )
            }
        }
    }

    pub fn format_state(&self) -> String {
        let mut inner = self.active.show();
        format!("{:?}", inner)
    }
}