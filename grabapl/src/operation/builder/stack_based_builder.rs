use crate::operation::builder::{
    BuilderInstruction, BuilderOpLike, IntermediateState, OperationBuilderError, QueryPath,
    merge_states_result,
};
use crate::operation::signature::parameter::{AbstractOutputNodeMarker, OperationParameter};
use crate::operation::signature::parameterbuilder::OperationParameterBuilder;
use crate::operation::user_defined::{
    AbstractNodeId, AbstractOperationArgument, AbstractOperationResultMarker,
    AbstractUserDefinedOperationOutput, Instruction, InstructionWithResultMarker, NamedMarker,
    QueryInstructions, UserDefinedOperation,
};
use crate::prelude::*;
use crate::{NodeKey, Semantics, SubstMarker};
use derive_more::From;
use derive_more::with_trait::TryInto;
use error_stack::{ResultExt, bail, report};
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::operation::OperationContext;
use crate::operation::marker::{Marker, SkipMarkers};
use crate::operation::query::{GraphShapeQuery, ShapeNodeIdentifier};
use crate::operation::signature::{
    AbstractOutputChanges, AbstractSignatureNodeId, OperationSignature,
};
use crate::semantics::{AbstractJoin, AbstractMatcher};
use crate::util::bimap::BiMap;
use crate::util::log;
use error_stack::Result;

// TODO: give overview of how the stack based builder works.

macro_rules! bail_unexpected_instruction {
    ($i:expr, $i_opt:expr, $frame:literal) => {
        let err = Err(report!(OperationBuilderError::UnexpectedInstruction))
            .attach_printable_lazy(|| format!("Unexpected instruction in {}: {:?}", $frame, $i));
        let _ = $i_opt.insert($i);
        return err;
    };
}

// TODO: turn the error type into a struct with a boolean `recoverable`.
//  That field would indicate whether the error can be recovered from by pushing more instructions or not.
//  For example, an incomplete parameter graph (i.e., disconnected context nodes) would have that flag set.
//  Then, in the builder's push_instruction method, we would only "best-effort-continue" if the error is recoverable.
//  Actually - does that make sense? The frames don't know we're pushing arbitrary "Finalize" instructions
//  just to get a partial operation. But it might work anyway.

struct BuildingParameterFrame<S: Semantics> {
    parameter_builder: OperationParameterBuilder<S>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone
    for BuildingParameterFrame<S>
{
    fn clone(&self) -> Self {
        BuildingParameterFrame {
            parameter_builder: self.parameter_builder.clone(),
        }
    }
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
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let this: &mut BuildingParameterFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();

        match instruction {
            BI::ExpectParameterNode(marker, av) => {
                this.parameter_builder
                    .expect_explicit_input_node(marker, av)
                    .change_context(OperationBuilderError::ParameterBuildError)?;
            }
            BI::ExpectContextNode(marker, av) => {
                this.parameter_builder
                    .expect_context_node(marker, av)
                    .change_context(OperationBuilderError::ParameterBuildError)?;
            }
            BI::ExpectParameterEdge(src, dst, edge) => {
                this.parameter_builder
                    .expect_edge(src, dst, edge)
                    .change_context(OperationBuilderError::ParameterBuildError)?;
            }
            _ => {
                // The user has decided that they're done building the parameter by sending a different instruction
                // restore instruction so we can continue with the appropriate frame
                let _ = instruction_opt.insert(instruction);

                let this: BuildingParameterFrame<S> = builder.stack.expect_pop();
                let parameter = this
                    .parameter_builder
                    .build()
                    .change_context(OperationBuilderError::ParameterBuildError)?;
                parameter
                    .check_validity()
                    .change_context(OperationBuilderError::ParameterBuildError)?;
                let frame = CollectingInstructionsFrame::from_param(&parameter);
                builder.data.built.parameter = Some(parameter.clone());
                builder.data.expected_self_signature.parameter = parameter.clone();

                builder.push_frame(frame);
            }
        };
        Ok(())
    }
}

struct CollectingInstructionsFrame<S: Semantics> {
    instructions: Vec<InstructionWithResultMarker<S>>,
    current_state: IntermediateState<S>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone
    for CollectingInstructionsFrame<S>
{
    fn clone(&self) -> Self {
        CollectingInstructionsFrame {
            instructions: self.instructions.clone(),
            current_state: self.current_state.clone(),
        }
    }
}

impl<S: Semantics> CollectingInstructionsFrame<S> {
    pub fn from_param(parameter: &OperationParameter<S>) -> Self {
        CollectingInstructionsFrame {
            instructions: vec![],
            current_state: IntermediateState::from_param(parameter),
        }
    }

    pub fn from_state(state: IntermediateState<S>) -> Self {
        CollectingInstructionsFrame {
            instructions: vec![],
            current_state: state,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
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
            BI::AddBangOperation(node_name, builder_op_like, args) => {
                let temp_op_marker = this.current_state.get_next_op_result_marker();

                let created_aids = this.handle_operation(
                    &mut builder.data,
                    Some(temp_op_marker),
                    builder_op_like,
                    args,
                )?;

                if created_aids.len() != 1 {
                    bail!(OperationBuilderError::Oneoff(
                        "bang operations must create exactly one node"
                    ));
                }
                // now rename as well
                let old_aid = created_aids[0];
                let _ = instruction_opt.insert(BuilderInstruction::RenameNode(old_aid, node_name));
            }
            BI::StartQuery(..) => {
                let (query_frame, branches_frame) =
                    QueryFrame::new(&this.current_state, instruction)?;

                builder.push_frame(query_frame);
                builder.push_frame(branches_frame);
            }
            // need to handle instructions that change the branch - endquery, entertrue, enterfalse
            instruction if instruction.can_break_body() => {
                let our_frame: CollectingInstructionsFrame<S> = builder.stack.expect_pop();

                // we're done, so push ourselves onto the return stack
                builder.return_stack.push(our_frame);

                // put instruction back, since we want the lower frame to take over
                let _ = instruction_opt.insert(instruction);
            }
            BI::RenameNode(old_aid, new_name) => {
                // don't allow renaming ParameterMarker nodes
                if let AbstractNodeId::ParameterMarker(_) = old_aid {
                    bail!(OperationBuilderError::CannotRenameParameterNode(old_aid));
                }
                let new_aid = AbstractNodeId::named(new_name);
                this.current_state.rename_aid(old_aid, new_aid)?;

                this.instructions.push((
                    None,
                    Instruction::RenameNode {
                        old: old_aid,
                        new: new_aid,
                    },
                ))
            }
            BI::StartShapeQuery(op_result_marker) => {
                // we start a new BuildingShapeQueryFrame
                let initial_state = this.current_state.clone();
                let shape_query_frame =
                    BuildingShapeQueryFrame::new(op_result_marker, initial_state);
                // push it onto the stack
                builder.push_frame(shape_query_frame);
            }
            BI::Diverge(msg) => {
                this.current_state.diverge();
                this.instructions.push((
                    None,
                    Instruction::Diverge {
                        crash_message: msg.to_string(),
                    },
                ));
            }
            BI::Trace => {
                this.instructions.push((None, Instruction::Trace));
            }
            _ => {
                bail_unexpected_instruction!(
                    instruction,
                    instruction_opt,
                    "CollectingInstructionsFrame"
                );
            }
        }

        Ok(())
    }

    /// Returns the new AIDs
    pub fn handle_operation(
        &mut self,
        builder_data: &mut BuilderData<S>,
        output_name: Option<AbstractOperationResultMarker>,
        op_like: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<Vec<AbstractNodeId>, OperationBuilderError> {
        let op = op_like
            .as_abstract_operation(builder_data.op_ctx, &builder_data.expected_self_signature)?;
        let (abstract_arg, output_res) =
            self.current_state
                .interpret_op(builder_data.op_ctx, output_name, op, args)?;

        let op_like_instr = op_like.into_op_like_instruction(builder_data.self_op_id);

        self.instructions.push((
            output_name,
            Instruction::OpLike(op_like_instr, abstract_arg),
        ));
        // forget removed aids
        for aid in output_res.removed_aids {
            self.instructions
                .push((None, Instruction::ForgetAid { aid }));
        }

        Ok(output_res.new_aids)
    }
}

struct BranchesFrame<S: Semantics> {
    initial_true_branch_state: IntermediateState<S>,
    initial_false_branch_state: IntermediateState<S>,
    currently_entered_branch: Option<bool>, // true for true branch, false for false branch
    true_branch: Option<CollectingInstructionsFrame<S>>,
    false_branch: Option<CollectingInstructionsFrame<S>>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for BranchesFrame<S> {
    fn clone(&self) -> Self {
        BranchesFrame {
            initial_true_branch_state: self.initial_true_branch_state.clone(),
            initial_false_branch_state: self.initial_false_branch_state.clone(),
            currently_entered_branch: self.currently_entered_branch,
            true_branch: self.true_branch.clone(),
            false_branch: self.false_branch.clone(),
        }
    }
}

impl<S: Semantics> BranchesFrame<S> {
    pub fn new(
        initial_true_branch_state: IntermediateState<S>,
        initial_false_branch_state: IntermediateState<S>,
    ) -> Self {
        BranchesFrame {
            initial_true_branch_state,
            initial_false_branch_state,
            currently_entered_branch: None,
            true_branch: None,
            false_branch: None,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let this: &mut BranchesFrame<S> = builder.stack.expect_mut();

        if let Some(branch) = this.currently_entered_branch
            && builder
                .return_stack
                .top_is::<CollectingInstructionsFrame<S>>()
        {
            if branch {
                // We are in the true branch
                let branch_frame: CollectingInstructionsFrame<S> =
                    builder.return_stack.expect_pop();
                this.true_branch = Some(branch_frame);
            } else {
                // We are in the false branch
                let branch_frame: CollectingInstructionsFrame<S> =
                    builder.return_stack.expect_pop();
                this.false_branch = Some(branch_frame);
            }
            this.currently_entered_branch = None;
        }

        let instruction = instruction_opt.take().unwrap();

        match instruction {
            BI::EnterTrueBranch => {
                if this.true_branch.is_some() {
                    bail!(OperationBuilderError::AlreadyVisitedBranch(true));
                }
                // We enter the true branch
                let true_frame =
                    CollectingInstructionsFrame::from_state(this.initial_true_branch_state.clone());
                this.currently_entered_branch = Some(true);
                builder.push_frame(true_frame);
            }
            BI::EnterFalseBranch => {
                if this.false_branch.is_some() {
                    bail!(OperationBuilderError::AlreadyVisitedBranch(false));
                }
                // We enter the false branch
                let false_frame = CollectingInstructionsFrame::from_state(
                    this.initial_false_branch_state.clone(),
                );
                this.currently_entered_branch = Some(false);
                builder.push_frame(false_frame);
            }
            BI::EndQuery | BI::Finalize => {
                // outer frame must handle this
                let this: BranchesFrame<S> = builder.stack.expect_pop();
                builder.return_stack.push(this);
                let _ = instruction_opt.insert(instruction);
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "QueryFrame");
            }
        }

        Ok(())
    }

    fn into_merged_state_and_query_instructions(
        self,
        default_true_state: &IntermediateState<S>,
        default_false_state: &IntermediateState<S>,
    ) -> Result<(IntermediateState<S>, QueryInstructions<S>), OperationBuilderError> {
        let true_branch_state_ref = self
            .true_branch
            .as_ref()
            .map(|cif| &cif.current_state)
            .unwrap_or(default_true_state);
        let false_branch_state_ref = self
            .false_branch
            .as_ref()
            .map(|cif| &cif.current_state)
            .unwrap_or(default_false_state);
        let merge_result = merge_states_result(true_branch_state_ref, false_branch_state_ref);

        // take into account the missing AIDs from the branches, and insert ForgetAid instructions
        let mut true_instructions = self
            .true_branch
            .map(|cif| cif.instructions)
            .unwrap_or_default();
        for aid in merge_result.missing_from_true {
            true_instructions.push((None, Instruction::ForgetAid { aid }));
        }

        let mut false_instructions = self
            .false_branch
            .map(|cif| cif.instructions)
            .unwrap_or_default();
        for aid in merge_result.missing_from_false {
            false_instructions.push((None, Instruction::ForgetAid { aid }));
        }

        let query_instructions = QueryInstructions {
            taken: true_instructions,
            not_taken: false_instructions,
        };
        Ok((merge_result.merged_state, query_instructions))
    }
}

struct QueryFrame<S: Semantics> {
    query: S::BuiltinQuery,
    abstract_arg: AbstractOperationArgument,
    before_branches_state: IntermediateState<S>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for QueryFrame<S> {
    fn clone(&self) -> Self {
        QueryFrame {
            query: self.query.clone(),
            abstract_arg: self.abstract_arg.clone(),
            before_branches_state: self.before_branches_state.clone(),
        }
    }
}

impl<S: Semantics> QueryFrame<S> {
    pub fn new(
        outer_state: &IntermediateState<S>,
        instruction: BuilderInstruction<S>,
    ) -> Result<(Self, BranchesFrame<S>), OperationBuilderError> {
        use BuilderInstruction as BI;

        match instruction {
            BI::StartQuery(query, args) => {
                let mut before_branches_state = outer_state.clone();
                // TODO: decide if queries should be allowed to modify the state.
                //  (maybe they should even be allowed to provide different states on true and false?)
                let abstract_arg = before_branches_state.interpret_builtin_query(&query, args)?;

                let frame = QueryFrame {
                    query,
                    abstract_arg,
                    before_branches_state,
                };

                let branches_frame = BranchesFrame::new(
                    frame.before_branches_state.clone(),
                    frame.before_branches_state.clone(),
                );

                Ok((frame, branches_frame))
            }
            _ => Err(report!(OperationBuilderError::UnexpectedInstruction))
                .attach_printable_lazy(|| format!("Expected StartQuery, got: {instruction:?}")),
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::EndQuery | BI::Finalize => {
                // TODO: decide if BI::Finalize should be allowed to end a query in the context of
                // building a final operation, or if explicit endquery operations are required.

                // We finish the query, and give the outer frame all our information.
                let query_frame: QueryFrame<S> = builder.stack.expect_pop();
                query_frame.handle_query_end(builder)?;
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "QueryFrame");
            }
        }

        Ok(())
    }

    fn handle_query_end(self, builder: &mut Builder<S>) -> Result<(), OperationBuilderError> {
        // we need to handle everything that happens at the end of a query frame - i.e., merging states
        let branches_frame: BranchesFrame<S> = builder.return_stack.expect_pop();

        let (merged_branch, query_instructions) = branches_frame
            .into_merged_state_and_query_instructions(
                &self.before_branches_state,
                &self.before_branches_state,
            )?;

        let outer_frame: &mut CollectingInstructionsFrame<S> = builder.stack.expect_mut();
        outer_frame.current_state = merged_branch;
        // push ourselves as instruction
        outer_frame.instructions.push((
            None,
            Instruction::BuiltinQuery(self.query, self.abstract_arg, query_instructions),
        ));

        Ok(())
    }
}

/// This frame's entire purpose is to create the final return frame once the data is available.
struct WrapperReturnFrame<S: Semantics> {
    phantom: PhantomData<S>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for WrapperReturnFrame<S> {
    fn clone(&self) -> Self {
        WrapperReturnFrame {
            phantom: PhantomData,
        }
    }
}

impl<S: Semantics> WrapperReturnFrame<S> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        _instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        let _: WrapperReturnFrame<S> = builder.stack.expect_pop();
        // We need to create the ReturnFrame from the current state and the return stack.
        let instr_frame: CollectingInstructionsFrame<S> = builder.return_stack.expect_pop();
        let return_frame = ReturnFrame::new(&builder.data, instr_frame);
        builder.stack.push(return_frame);

        // we don't consume the instruction, hence it will be passed to the ReturnFrame.

        Ok(())
    }
}

struct ReturnFrame<S: Semantics> {
    instr_frame: CollectingInstructionsFrame<S>,
    signature: OperationSignature<S>,
    abstract_ud_output: AbstractUserDefinedOperationOutput,
    // TODO: only used to check for existence of already returned edges. Do we really need this?
    return_edges: HashSet<(AbstractNodeId, AbstractNodeId)>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for ReturnFrame<S> {
    fn clone(&self) -> Self {
        ReturnFrame {
            instr_frame: self.instr_frame.clone(),
            signature: self.signature.clone(),
            abstract_ud_output: self.abstract_ud_output.clone(),
            return_edges: self.return_edges.clone(),
        }
    }
}

impl<S: Semantics> ReturnFrame<S> {
    pub fn new(data: &BuilderData<S>, cif: CollectingInstructionsFrame<S>) -> Self {
        let mut signature =
            OperationSignature::empty_new("some_name", data.built.parameter.clone().unwrap());
        populate_signature_changes(&mut signature, &cif.current_state);

        ReturnFrame {
            instr_frame: cif,
            signature,
            abstract_ud_output: AbstractUserDefinedOperationOutput::new(),
            return_edges: HashSet::new(),
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let this: &mut ReturnFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::ReturnNode(aid, output_marker, node) => {
                this.include_return_node(aid, output_marker, node, &builder.data)?;
            }
            BI::ReturnEdge(src, dst, edge) => {
                this.include_return_edge(src, dst, edge, &builder.data)?;
            }
            BI::Finalize => {
                // nothing for now, just consume.
                // TODO
                // In future: maybe push self onto return stack?
                // maybe switch type of return stack to a different enum.
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "ReturnFrame");
            }
        }

        Ok(())
    }

    fn get_return_node_marker(&self, aid: &AbstractNodeId) -> Option<AbstractOutputNodeMarker> {
        self.abstract_ud_output.new_nodes.get(aid).copied()
    }

    fn include_return_node(
        &mut self,
        aid: AbstractNodeId,
        output_marker: AbstractOutputNodeMarker,
        av: S::NodeAbstract,
        data: &BuilderData<S>,
    ) -> Result<(), OperationBuilderError> {
        if let AbstractNodeId::ParameterMarker(_) = aid {
            bail!(OperationBuilderError::CannotReturnParameter(aid));
        }
        if !self.last_state().contains_aid(&aid) {
            bail!(OperationBuilderError::NotFoundAid(aid));
        }
        if self.get_return_node_marker(&aid).is_some() {
            bail!(OperationBuilderError::Oneoff(
                "already returned this return node"
            ));
        }
        // if we have already asserted that we return a node with this marker, it must be the same type.
        if let Some(expected_av) = data
            .expected_self_signature
            .output
            .new_nodes
            .get(&output_marker)
        {
            if expected_av != &av {
                bail!(OperationBuilderError::Oneoff(
                    "trying to return node with type different from stated return type"
                ));
            }
        }

        // if the user wants to return the node as an `av`, `av` must be a supertype of the inferred type
        let inferred_av = self.last_state().node_av_of_aid(&aid).unwrap();
        if !S::NodeMatcher::matches(inferred_av, &av) {
            bail!(OperationBuilderError::InvalidReturnNodeType(aid));
        }
        // TODO: I think we can comment this and actually allow returning nodes that originate from a shape query.
        //  Reason: an invariant of the abstract graph is that there is at most one abstract handle to any given node at any point.
        //  In other words, since we were able to match a node in the shape query, that means we have that single handle to the node,
        //  and we can do with it whatever we want. Including returning it.
        // if self
        //     .last_state()
        //     .node_may_originate_from_shape_query
        //     .contains(&aid)
        // {
        //     bail!(BuilderError::NeedsSpecificVariant(
        //         "cannot return node that originates from a shape query"
        //     ));
        // }

        self.abstract_ud_output.new_nodes.insert(aid, output_marker);
        self.signature.output.new_nodes.insert(output_marker, av);
        Ok(())
    }

    fn include_return_edge(
        &mut self,
        src: AbstractNodeId,
        dst: AbstractNodeId,
        av: S::EdgeAbstract,
        data: &BuilderData<S>,
    ) -> Result<(), OperationBuilderError> {
        if !self.last_state().contains_edge(&src, &dst) {
            bail!(OperationBuilderError::NotFoundReturnEdge(src, dst));
        }
        if self.return_edges.contains(&(src, dst)) {
            bail!(OperationBuilderError::Oneoff("already returned this edge"));
        }
        if !self.last_state().contains_aid(&src) {
            bail!(OperationBuilderError::NotFoundReturnEdgeSource(src));
        }
        if !self.last_state().contains_aid(&dst) {
            bail!(OperationBuilderError::NotFoundReturnEdgeTarget(dst));
        }

        let src_sig_id = self
            .aid_to_sig_id(&src)
            .attach_printable_lazy(|| "cannot use source node in signature")?;
        let dst_sig_id = self
            .aid_to_sig_id(&dst)
            .attach_printable_lazy(|| "cannot use destination node in signature")?;

        // if we have already asserted that we return an edge with this src and dst, it must be the same type.
        if let Some(expected_av) = data
            .expected_self_signature
            .output
            .new_edges
            .get(&(src_sig_id, dst_sig_id))
        {
            if expected_av != &av {
                bail!(OperationBuilderError::Oneoff(
                    "trying to return edge with type different from stated return type"
                ));
            }
        }

        // if the user wants to return the edge as an `av`, `av` must be a supertype of the inferred type
        let inferred_av = self.last_state().edge_av_of_aid(&src, &dst).unwrap();
        if !S::EdgeMatcher::matches(inferred_av, &av) {
            bail!(OperationBuilderError::InvalidReturnEdgeType(src, dst));
        }
        // TODO: remove this check. see the comment in `include_return_node`.
        // if self
        //     .last_state()
        //     .edge_may_originate_from_shape_query
        //     .contains(&(src, dst))
        // {
        //     bail!(BuilderError::NeedsSpecificVariant(
        //         "cannot return edge that originates from a shape query"
        //     ));
        // }

        self.signature
            .output
            .new_edges
            .insert((src_sig_id, dst_sig_id), av);

        self.return_edges.insert((src, dst));
        Ok(())
    }

    fn aid_to_sig_id(
        &self,
        aid: &AbstractNodeId,
    ) -> Result<AbstractSignatureNodeId, OperationBuilderError> {
        match *aid {
            AbstractNodeId::ParameterMarker(s) => Ok(AbstractSignatureNodeId::ExistingNode(s)),
            AbstractNodeId::DynamicOutputMarker(_, _) | AbstractNodeId::Named(..) => {
                // we must be returning this node if we want to return an incident edge.
                let Some(output_marker) = self.get_return_node_marker(aid) else {
                    bail!(OperationBuilderError::NotFoundReturnNode(*aid));
                };
                Ok(AbstractSignatureNodeId::NewNode(output_marker))
            }
        }
    }

    fn last_state(&self) -> &IntermediateState<S> {
        &self.instr_frame.current_state
    }
}

/// This frame is used to build a shape query, i.e., everything before the first EnterXBranch/EndQuery instruction.
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
    skip_markers: SkipMarkers,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone
    for BuildingShapeQueryFrame<S>
{
    fn clone(&self) -> Self {
        BuildingShapeQueryFrame {
            parameter: self.parameter.clone(),
            abstract_arg: self.abstract_arg.clone(),
            gsq_node_keys_to_shape_idents: self.gsq_node_keys_to_shape_idents.clone(),
            query_marker: self.query_marker,
            initial_state: self.initial_state.clone(),
            true_branch_state: self.true_branch_state.clone(),
            skip_markers: self.skip_markers.clone(),
        }
    }
}

impl<S: Semantics> BuildingShapeQueryFrame<S> {
    pub fn new(
        query_marker: AbstractOperationResultMarker,
        initial_state: IntermediateState<S>,
    ) -> Self {
        let true_branch_state = initial_state.clone();
        let (parameter, abstract_arg) = initial_state.as_param_for_shape_query();
        BuildingShapeQueryFrame {
            parameter,
            abstract_arg,
            query_marker,
            gsq_node_keys_to_shape_idents: BiMap::new(),
            initial_state,
            true_branch_state,
            skip_markers: SkipMarkers::none(), // we don't skip any markers by default
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let this: &mut BuildingShapeQueryFrame<S> = builder.stack.expect_mut();

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::ExpectShapeNode(marker, av) => {
                let aid = AbstractNodeId::dynamic_output(this.query_marker, marker);
                let sni: ShapeNodeIdentifier = marker.0.into();
                // return error if we already encountered this key before
                if this.gsq_node_keys_to_shape_idents.contains_right(&sni) {
                    bail!(OperationBuilderError::ShapeNodeAlreadyExists(sni));
                }

                this.true_branch_state.add_node(aid, av, true);
                this.gsq_node_keys_to_shape_idents
                    .insert(this.true_branch_state.get_key_from_aid(&aid)?, sni);
            }
            BI::ExpectShapeNodeChange(aid, new_av) => {
                this.true_branch_state.set_node_av(aid, new_av)?;
            }
            BI::ExpectShapeEdge(src, dst, edge) => {
                this.true_branch_state.add_edge(src, dst, edge, true)?;
            }
            BI::SkipMarker(marker) => {
                this.skip_markers.skip(marker);
            }
            BI::SkipAllMarkers => {
                this.skip_markers.skip_all();
            }
            instruction if instruction.can_break_body() => {
                // Advance to BuiltShapeQueryFrame
                // it needs to consume this instruction
                let _ = instruction_opt.insert(instruction);

                let this: BuildingShapeQueryFrame<S> = builder.stack.expect_pop();
                let (built_frame, branches_frame) = this.into_built_shape_query_frame(builder)?;
                builder.push_frame(built_frame);
                builder.push_frame(branches_frame);
            }
            _ => {
                bail_unexpected_instruction!(
                    instruction,
                    instruction_opt,
                    "BuildingShapeQueryFrame"
                );
            }
        }

        Ok(())
    }

    fn into_built_shape_query_frame(
        self,
        _builder: &mut Builder<S>,
    ) -> Result<(BuiltShapeQueryFrame<S>, BranchesFrame<S>), OperationBuilderError> {
        // We build the parameter and the initial state

        // TODO: check validity, i.e., no free floating shape nodes, etc.

        let query = GraphShapeQuery::new(
            self.parameter,
            self.true_branch_state.graph.clone(),
            self.gsq_node_keys_to_shape_idents,
        )
        .with_skip_markers(self.skip_markers);

        let built_frame = BuiltShapeQueryFrame::new(
            self.query_marker,
            query,
            self.abstract_arg,
            self.initial_state.clone(),
            self.true_branch_state.clone(),
        );

        let branches_frame =
            BranchesFrame::new(self.true_branch_state.clone(), self.initial_state.clone());

        Ok((built_frame, branches_frame))
    }
}

/// This frame is used to handle the branches of a *built* shape query. It is the product of BuildingShapeQueryFrame.
struct BuiltShapeQueryFrame<S: Semantics> {
    query_marker: AbstractOperationResultMarker,
    query: GraphShapeQuery<S>,
    abstract_arg: AbstractOperationArgument,
    initial_false_branch_state: IntermediateState<S>,
    initial_true_branch_state: IntermediateState<S>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for BuiltShapeQueryFrame<S> {
    fn clone(&self) -> Self {
        BuiltShapeQueryFrame {
            query_marker: self.query_marker,
            query: self.query.clone(),
            abstract_arg: self.abstract_arg.clone(),
            initial_false_branch_state: self.initial_false_branch_state.clone(),
            initial_true_branch_state: self.initial_true_branch_state.clone(),
        }
    }
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
        }
    }

    pub fn consume(
        builder: &mut Builder<S>,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::EndQuery | BI::Finalize => {
                // We finish the query, and give the outer frame all our information.
                let query_frame: BuiltShapeQueryFrame<S> = builder.stack.expect_pop();
                query_frame.handle_shape_query_end(builder)?;
            }
            _ => {
                bail_unexpected_instruction!(instruction, instruction_opt, "BuiltShapeQueryFrame");
            }
        }

        Ok(())
    }

    fn handle_shape_query_end(self, builder: &mut Builder<S>) -> Result<(), OperationBuilderError> {
        // we need to handle everything that happens at the end of a query frame - i.e., merging states

        let branches_frame: BranchesFrame<S> = builder.return_stack.expect_pop();

        let (merged_branch, query_instructions) = branches_frame
            .into_merged_state_and_query_instructions(
                &self.initial_true_branch_state,
                &self.initial_false_branch_state,
            )?;

        let outer_frame: &mut CollectingInstructionsFrame<S> = builder.stack.expect_mut();
        outer_frame.current_state = merged_branch;
        outer_frame.instructions.push((
            Some(self.query_marker),
            Instruction::ShapeQuery(self.query, self.abstract_arg, query_instructions),
        ));

        Ok(())
    }
}

#[derive(From, TryInto)]
#[try_into(owned, ref, ref_mut)]
enum Frame<S: Semantics> {
    BuildingParameter(BuildingParameterFrame<S>),
    // TODO: unfortunate name - 'instructions' is also builder instructions. maybe CollectingStatements?
    CollectingInstructions(CollectingInstructionsFrame<S>),
    Query(QueryFrame<S>),
    Branches(BranchesFrame<S>),
    BuildingShapeQuery(BuildingShapeQueryFrame<S>),
    BuiltShapeQuery(BuiltShapeQueryFrame<S>),
    Return(ReturnFrame<S>),
    WrapperReturn(WrapperReturnFrame<S>),
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for Frame<S> {
    fn clone(&self) -> Self {
        match self {
            Frame::BuildingParameter(frame) => Frame::BuildingParameter(frame.clone()),
            Frame::CollectingInstructions(frame) => Frame::CollectingInstructions(frame.clone()),
            Frame::Query(frame) => Frame::Query(frame.clone()),
            Frame::Branches(frame) => Frame::Branches(frame.clone()),
            Frame::BuildingShapeQuery(frame) => Frame::BuildingShapeQuery(frame.clone()),
            Frame::BuiltShapeQuery(frame) => Frame::BuiltShapeQuery(frame.clone()),
            Frame::Return(frame) => Frame::Return(frame.clone()),
            Frame::WrapperReturn(frame) => Frame::WrapperReturn(frame.clone()),
        }
    }
}

struct FrameStack<S: Semantics> {
    frames: Vec<Frame<S>>,
}

impl<S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for FrameStack<S> {
    fn clone(&self) -> Self {
        FrameStack {
            frames: self.frames.clone(),
        }
    }
}

impl<S: Semantics> FrameStack<S> {
    pub fn new_initial() -> Self {
        let mut stack = FrameStack::new_empty();
        stack.push(WrapperReturnFrame::new());
        stack.push(BuildingParameterFrame::new());
        stack
    }

    pub fn new_empty() -> Self {
        FrameStack { frames: vec![] }
    }

    pub fn push<T: Into<Frame<S>>>(&mut self, frame: T) {
        log::trace!("Pushing frame: {:?}", std::any::type_name::<T>());
        self.frames.push(frame.into());
    }

    pub fn last(&self) -> Option<&Frame<S>> {
        self.frames.last()
    }

    pub fn top_is<'a, F>(&'a self) -> bool
    where
        S: 'a,
        F: 'a,
        &'a Frame<S>: TryInto<&'a F>,
    {
        self.frames.last().is_some_and(|f| f.try_into().is_ok())
    }

    #[track_caller]
    pub fn expect_mut<'a, F>(&'a mut self) -> F
    where
        S: 'a,
        &'a mut Frame<S>: TryInto<F>,
    {
        let last = self.frames.last_mut().unwrap();
        last.try_into().ok().unwrap()
    }

    #[track_caller]
    pub fn expect_ref<'a, F>(&'a self) -> F
    where
        S: 'a,
        &'a Frame<S>: TryInto<F>,
    {
        let last = self.frames.last().unwrap();
        last.try_into().ok().unwrap()
    }

    #[track_caller]
    pub fn expect_pop<F>(&mut self) -> F
    where
        Frame<S>: TryInto<F>,
    {
        let last = self.frames.pop().unwrap();
        last.try_into().ok().unwrap()
    }

    fn to_query_path(&self) -> Vec<QueryPath> {
        let mut path = vec![];
        for frame in &self.frames {
            match frame {
                Frame::Query(_query_frame) => {
                    // TODO: require BuiltinQuery::name() function?
                    path.push(QueryPath::Query("<unnamed query>".to_string()));
                }
                Frame::Branches(branches_frame) => {
                    // check which branch we are in
                    if let Some(entered_branch) = branches_frame.currently_entered_branch {
                        let segment = if entered_branch {
                            QueryPath::TrueBranch
                        } else {
                            QueryPath::FalseBranch
                        };
                        path.push(segment);
                    } else {
                        log::info!("branches frame has not entered any branch yet");
                    }
                }
                Frame::BuildingShapeQuery(shape_query_frame) => {
                    path.push(QueryPath::Query(format!(
                        "{:?}",
                        shape_query_frame.query_marker
                    )));
                }
                Frame::BuiltShapeQuery(built_shape_query_frame) => {
                    path.push(QueryPath::Query(format!(
                        "{:?}",
                        built_shape_query_frame.query_marker
                    )));
                }
                _ => {}
            }
        }
        path
    }
}

struct BuiltData<S: Semantics> {
    // TODO: remove? do we need BuiltData still?
    //  (parameter is now stored in the expected self sig)
    parameter: Option<OperationParameter<S>>,
}

impl<S: Semantics> Clone for BuiltData<S> {
    fn clone(&self) -> Self {
        BuiltData {
            parameter: self.parameter.clone(),
        }
    }
}

impl<S: Semantics> BuiltData<S> {
    pub fn new() -> Self {
        BuiltData { parameter: None }
    }
}

struct BuilderData<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    self_op_id: OperationId,
    built: BuiltData<S>,
    partial_self_op: UserDefinedOperation<S>,
    /// How we expect our signature to look like
    /// Includes changes asserted by the user via e.g. SelfReturnNode
    expected_self_signature: OperationSignature<S>,
}

impl<'a, S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for BuilderData<'a, S> {
    fn clone(&self) -> Self {
        BuilderData {
            op_ctx: self.op_ctx,
            built: self.built.clone(),
            self_op_id: self.self_op_id,
            partial_self_op: self.partial_self_op.clone(),
            expected_self_signature: self.expected_self_signature.clone(),
        }
    }
}

impl<'a, S: Semantics> BuilderData<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>, self_op_id: OperationId) -> Self {
        BuilderData {
            op_ctx,
            built: BuiltData::new(),
            self_op_id,
            partial_self_op: UserDefinedOperation::new_noop(),
            expected_self_signature: OperationSignature::empty_new(
                "some_name",
                OperationParameter::new_empty(),
            ),
        }
    }

    pub fn consume_global(
        &mut self,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
    ) -> Result<(), OperationBuilderError> {
        use BuilderInstruction as BI;

        let instruction = instruction_opt.take().unwrap();
        match instruction {
            BI::SelfReturnNode(output_marker, av) => {
                self.expected_self_signature
                    .output
                    .new_nodes
                    .insert(output_marker, av);
            }
            BI::SelfReturnEdge(src, dst, av) => {
                self.expected_self_signature
                    .output
                    .new_edges
                    .insert((src, dst), av);
            }
            _ => {
                // do nothing
                let _ = instruction_opt.insert(instruction);
            }
        }

        Ok(())
    }
}

pub struct Builder<'a, S: Semantics> {
    data: BuilderData<'a, S>,
    stack: FrameStack<S>,
    return_stack: FrameStack<S>,
}

impl<'a, S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> Clone for Builder<'a, S> {
    fn clone(&self) -> Self {
        Builder {
            data: self.data.clone(),
            stack: self.stack.clone(),
            return_stack: self.return_stack.clone(),
        }
    }
}

impl<'a, S: Semantics> Builder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>, self_op_id: OperationId) -> Self {
        Builder {
            data: BuilderData::new(op_ctx, self_op_id),
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
            Some(Frame::Query(frame)) => BuilderShowData::QueryFrame(&frame.before_branches_state),
            Some(Frame::Branches(frame)) => BuilderShowData::BranchesFrame {
                true_state: &frame.initial_true_branch_state,
                false_state: &frame.initial_false_branch_state,
            },
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
            None => BuilderShowData::Other("No frame".to_string()),
        }
    }

    pub fn update_partial_self_op(&mut self, partial_self_op: UserDefinedOperation<S>) {
        self.data.partial_self_op = partial_self_op;
    }

    pub fn update_expected_self_output_changes(
        &mut self,
        expected_self_output_changes: AbstractOutputChanges<S>,
    ) {
        self.data.expected_self_signature.output = expected_self_output_changes;
    }

    pub fn consume(
        &mut self,
        instruction: BuilderInstruction<S>,
    ) -> Result<(), OperationBuilderError> {
        let mut instruction_opt = Some(instruction);

        // first check if we have a global instruction that needs to be consumed
        self.data.consume_global(&mut instruction_opt)?;
        if instruction_opt.is_none() {
            // if we consumed a global instruction, we don't need to continue
            return Ok(());
        }

        while instruction_opt.is_some() {
            let curr_frame = self.stack.last().unwrap();
            match curr_frame {
                Frame::BuildingParameter(..) => {
                    log::trace!("Consuming for BuildingParameterFrame");
                    BuildingParameterFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::CollectingInstructions(..) => {
                    log::trace!("Consuming for CollectingInstructionsFrame");
                    CollectingInstructionsFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::Query(..) => {
                    log::trace!("Consuming for QueryFrame");
                    QueryFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::Branches(..) => {
                    log::trace!("Consuming for BranchesFrame");
                    BranchesFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::Return(..) => {
                    log::trace!("Consuming for ReturnFrame");
                    ReturnFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::WrapperReturn(..) => {
                    log::trace!("Consuming for WrapperReturnFrame");
                    WrapperReturnFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::BuildingShapeQuery(..) => {
                    log::trace!("Consuming for BuildingShapeQueryFrame");
                    BuildingShapeQueryFrame::consume(self, &mut instruction_opt)?;
                }
                Frame::BuiltShapeQuery(..) => {
                    log::trace!("Consuming for BuiltShapeQueryFrame");
                    BuiltShapeQueryFrame::consume(self, &mut instruction_opt)?;
                }
            }
        }

        Ok(())
    }

    /// Builds the current self output changes for purposes of restarting the builder with this new information.
    fn build_partial_op(mut self) -> Result<AbstractOutputChanges<S>, OperationBuilderError> {
        // first, keep the expected changes
        let expected_self_signature = std::mem::replace(
            &mut self.data.expected_self_signature,
            OperationSignature::new_noop("some name"),
        );
        // then, build self as if it was a full op to get the signature
        // we need to build unvalidated, since we loosen the restriction of returning all expected return nodes on purpose.
        let op = self.build_unvalidated()?;
        // then, add the output changes from the signature
        // we merge the two
        let merged_changes =
            merge_abstract_output_changes(&expected_self_signature.output, &op.signature.output)?;

        // TODO: we should have an assert that the built op's signature has the same parameter as our expected signature.

        Ok(merged_changes)
    }

    fn build(mut self) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // first, keep the expected changes. these are not needed in build_unvalidated.
        // (ugly - should split the struct)
        let expected_signature = std::mem::replace(
            &mut self.data.expected_self_signature,
            OperationSignature::new_noop("some name"),
        );
        let op = self.build_unvalidated()?;
        // validate the operation against the expected self signature
        if op.signature.output.new_nodes != expected_signature.output.new_nodes {
            bail!(OperationBuilderError::Oneoff(
                "operation signature does not match stated signature: different returned nodes"
            ));
        }
        if op.signature.output.new_edges != expected_signature.output.new_edges {
            bail!(OperationBuilderError::Oneoff(
                "operation signature does not match stated signature: different returned edges"
            ));
        }

        // TODO: do we need more validity checks?

        Ok(op)
    }

    /// Builds the current operation but does not perform any final validity checks.
    fn build_unvalidated(mut self) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // this is a bit of a hack. it just works because all nested frames right now can be ended with Finalize.
        // we can 'define' the Finalize message to be just that, though.
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

        Ok(UserDefinedOperation {
            // parameter: self.data.built.parameter.unwrap(),
            signature,
            instructions: instr_frame.instructions,
            output_changes,
        })
    }

    fn push_frame(&mut self, frame: impl Into<Frame<S>>) {
        self.stack.push(frame);
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
                Some(*subst)
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
            current_edges.insert((*source_subst, *target_subst));
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

    for ((source_aid, target_aid), edge_abstract) in &last_state.edge_may_be_written_to {
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
    BranchesFrame {
        true_state: &'a IntermediateState<S>,
        false_state: &'a IntermediateState<S>,
    },
    ShapeQueryFrame(&'a IntermediateState<S>),
    ReturnFrame(&'a IntermediateState<S>),
    Other(String),
}

impl<'a, S: Semantics<NodeAbstract: Debug, EdgeAbstract: Debug>> Debug for BuilderShowData<'a, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BuilderShowData::ParameterBuilder(param_builder) => {
                write!(
                    f,
                    "ParameterBuilder: {:?}",
                    (*param_builder)
                        .clone()
                        .build()
                        .unwrap()
                        .parameter_graph
                        .shape_dot()
                )
            }
            BuilderShowData::CollectingInstructions(state) => {
                write!(f, "CollectingInstructions: {}", state.dot_with_aid())
            }
            BuilderShowData::QueryFrame(state) => {
                write!(f, "QueryFrame: {}", state.dot_with_aid())
            }
            BuilderShowData::BranchesFrame {
                true_state,
                false_state,
            } => {
                write!(
                    f,
                    "BranchesFrame: True: {}, False: {}",
                    true_state.dot_with_aid(),
                    false_state.dot_with_aid()
                )
            }
            BuilderShowData::ShapeQueryFrame(state) => {
                write!(f, "ShapeQueryFrame: {}", state.dot_with_aid())
            }
            BuilderShowData::ReturnFrame(state) => {
                write!(f, "ReturnFrame: {}", state.dot_with_aid())
            }
            BuilderShowData::Other(data) => {
                write!(f, "Other: {data}")
            }
        }
    }
}

/// Builds a user defined operation by collecting instructions and compiling them into a user defined operation.
///
/// At any point, the builder supports looking at the current abstract state of the operation being built via
/// [`OperationBuilder::show_state`].
///
/// For more information on the whole building process, see the [module-level documentation](crate::operation::builder).
// TODO: avoid having an explicit reference to the operation context.
//  maybe a refcell? and when building immediately add the operation to it?
//  maybe we could actually on-the-fly store a signature of self in the operation context for other operation builders.
pub struct OperationBuilder2<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    instructions: Vec<BuilderInstruction<S>>,
    active: Builder<'a, S>,
    self_op_id: OperationId,
}

impl<'a, S: Semantics<BuiltinQuery: Clone, BuiltinOperation: Clone>> OperationBuilder2<'a, S> {
    // TODO: for every instruction, specify in which context it is valid. DONE
    //  then maybe make sure the FrameStack frames above are named consistently?

    /// Creates a new operation builder with the given operation context and self operation ID.
    ///
    /// The operation context is used to access other available operations.
    ///
    /// The self operation ID must be used to insert the operation into the operation context after building it.
    pub fn new(op_ctx: &'a OperationContext<S>, self_op_id: OperationId) -> Self {
        Self {
            instructions: Vec::new(),
            op_ctx,
            active: Builder::new(op_ctx, self_op_id),
            self_op_id,
        }
    }

    /// Renames a node with the given abstract node ID to the new name.
    ///
    /// After this instruction, the node can not be accessed by the old name anymore, and instead
    /// must be accessed via `AbstractNodeId::named(new_name)`.
    ///
    /// Because branch merging is based on node names, this instruction can be used if
    /// a node from one branch should be merged with a node of a different name in the other branch.
    ///
    /// Parameter nodes cannot be renamed.
    ///
    /// Valid in:
    /// * statement context
    pub fn rename_node(
        &mut self,
        old_aid: AbstractNodeId,
        new_name: impl Into<NamedMarker>,
    ) -> Result<(), OperationBuilderError> {
        let new_name = new_name.into();
        self.push_instruction(BuilderInstruction::RenameNode(old_aid, new_name))
    }

    /// Adds an explicit parameter node with the given type to the operation.
    ///
    /// Explicit parameter nodes are ordered by the order in which they were added via this instruction.
    ///
    /// After this instruction, the node can be accessed via `AbstractNodeId::param(marker)`.
    ///
    /// Valid in:
    /// * parameter context
    pub fn expect_parameter_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let marker = marker.into();
        self.push_instruction(BuilderInstruction::ExpectParameterNode(marker, node))
    }

    /// Adds an implicit parameter node with the given type to the operation.
    ///
    /// Implicit parameter nodes are not ordered, and are used to represent nodes that are
    /// automatically, statically, and implicitly matched from the abstract graph whenever an
    /// operation is called.
    ///
    /// Implicit parameter nodes must be connected to explicit parameter nodes via
    /// [`OperationBuilder::expect_parameter_edge`].
    ///
    /// After this instruction, the node can be accessed via `AbstractNodeId::param(marker)`.
    ///
    /// Valid in:
    /// * parameter context
    pub fn expect_context_node(
        &mut self,
        marker: impl Into<SubstMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let marker = marker.into();
        self.push_instruction(BuilderInstruction::ExpectContextNode(marker, node))
    }

    /// Adds an edge between two parameter nodes.
    ///
    /// Valid in:
    /// * parameter context
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

    /// Starts the given query with the given arguments.
    ///
    /// This enters query context, which must either be exited with [`OperationBuilder::end_query`]
    /// or the branches must be entered with [`OperationBuilder::enter_true_branch`] or [`OperationBuilder::enter_false_branch`].
    ///
    /// Valid in:
    /// * statement context
    pub fn start_query(
        &mut self,
        query: S::BuiltinQuery,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::StartQuery(query, args))
    }

    // TODO: can we lift the restriction of never entering the same branch more than once?
    /// Enters the true branch of the current query.
    ///
    /// Can be executed immediately after starting a query or in the statement context
    /// after [`OperationBuilder::enter_false_branch`] as well, but never twice for the same query.
    ///
    /// This enters statement context. Statements in that context will be sent to the true branch of the currently active query.
    ///
    /// Valid in:
    /// * query context
    /// * statement context
    pub fn enter_true_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::EnterTrueBranch)
    }

    /// Enters the false branch of the current query.
    ///
    /// Can be executed immediately after starting a query or in the statement context
    /// after [`OperationBuilder::enter_true_branch`] as well, but never twice for the same query.
    ///
    /// This enters statement context. Statements in that context will be sent to the false branch of the currently active query.
    ///
    /// Valid in:
    /// * query context
    /// * statement context
    pub fn enter_false_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.push_instruction(BuilderInstruction::EnterFalseBranch)
    }

    // TODO: get rid of AbstractOperationResultMarker requirement. Either completely or make it optional and autogenerate one.
    //  How to specify which shape node? ==> the shape node markers should be unique per path
    /// Starts a shape query whose newly matched nodes will be bound to the map of the given marker.
    ///
    /// This enters shape query parameter context, in which the shape query can be built.
    ///
    /// Valid in:
    /// * statement context
    pub fn start_shape_query(
        &mut self,
        op_marker: impl Into<AbstractOperationResultMarker>,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::StartShapeQuery(op_marker.into()))
    }

    /// Ends the current query.
    ///
    /// Returns to the outer statement context.
    ///
    /// Valid in:
    /// * query context
    /// * statement context
    pub fn end_query(&mut self) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::EndQuery)
    }

    // TODO: should expect_*_node really expect a marker? maybe it should instead return a marker?
    //  it could also take an Option<Marker> so that it can autogenerate one if it's none so the caller doesn't have to deal with it.
    /// Adds the requirement to match a shape node with the given abstract value in order to enter the true branch.
    ///
    /// If the current shape query was started with `"shape_query_marker"`, in the true branch,
    /// the node will be available as `AbstractNodeId::dynamic_output("shape_query_marker", marker)`.
    ///
    /// Valid in:
    /// * shape query parameter context
    pub fn expect_shape_node(
        &mut self,
        marker: AbstractOutputNodeMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::ExpectShapeNode(marker, node))
    }

    /// Adds the requirement to match an existing node with a new abstract value in order to enter the true branch.
    ///
    /// Inside the true branch, the node will be visible with the changed abstract value.
    ///
    /// Ideally, the new abstract value is a subtype of the old abstract value, giving new information about the node.
    ///
    /// Valid in:
    /// * shape query parameter context
    pub fn expect_shape_node_change(
        &mut self,
        aid: AbstractNodeId,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::ExpectShapeNodeChange(aid, node))
    }

    /// Adds the requirement to match an edge with the given abstract value in order to enter the true branch.
    ///
    /// Inside the true branch, the edge will be visible with the given abstract value.
    ///
    /// Valid in:
    /// * shape query parameter context
    pub fn expect_shape_edge(
        &mut self,
        source: AbstractNodeId,
        target: AbstractNodeId,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::ExpectShapeEdge(source, target, edge))
    }

    /// Adds a node marker that the currently active shape query will skip.
    ///
    /// For example, we may want to mark nodes as "visited", and then skip all visited nodes
    /// in the shape query.
    ///
    /// Valid in:
    /// * shape query parameter context
    pub fn skip_marker(&mut self, marker: impl Into<Marker>) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::SkipMarker(marker.into()))
    }

    /// Tells the currently active shape query to skip all nodes that are marked with any marker.
    ///
    /// Valid in:
    /// * shape query parameter context
    pub fn skip_all_markers(&mut self) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::SkipAllMarkers)
    }

    /// Issues a call to the specified operation with the given arguments and binds the returned nodes to the map of the given name.
    ///
    /// After this instruction, if the called operation returned nodes `"a"`, `"b"`, and `"c"`, those are
    /// accessible via [`AbstractNodeId`]'s
    /// `AbstractNodeId::dynamic_output(name, "a")`, `AbstractNodeId::dynamic_output(name, "b")`,
    /// and `AbstractNodeId::dynamic_output(name, "c")`, respectively.
    pub fn add_named_operation(
        &mut self,
        name: AbstractOperationResultMarker,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::AddNamedOperation(name, op, args))
    }

    /// Issues a call to the specified operation with the given arguments and binds the single returned node
    /// to [`AbstractNodeId`]'s `AbstractNodeId::named(name)`.
    ///
    /// Returns an error if the operation does not return exactly one node.
    pub fn add_bang_operation(
        &mut self,
        name: impl Into<NamedMarker>,
        op: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::AddBangOperation(name.into(), op, args))
    }

    // ODOT: for ergonomics, could take an impl Into<BuilderOpLike<S>> and blanket impl that for all S::BuiltinOperation etc.
    //  ^ cannot do above right now, since From<S::BuiltinOperation> could be a conflict with From<BuilderOpLike>,
    //    since we cannot guarantee that S::BuiltinOperation is never equal BuilderOpLike itself.
    //    need trait negative bounds or specialization.
    /// Issues a call to the specified operation with the given arguments.
    ///
    /// As opposed to [`OperationBuilder::add_named_operation`] and [`OperationBuilder::add_bang_operation`],
    /// this does not bind potential new nodes to a name.
    ///
    /// Any returned nodes by the operation are hence invisible in the abstract state.
    /// So, if you do not bind the result of, e.g., an `add_child` operation, you will not be allowed
    /// to call a succeeding operation that requires the child node statically.
    ///
    /// Other abstract effects are still applied, like removed or changed nodes, and added, removed, or changed edges.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use grabapl::operation::builder::stack_based_builder::OperationBuilder2;
    /// # use grabapl::prelude::{BuilderOpLike, LibBuiltinOperation, OperationContext};
    /// # use grabapl::semantics::example::{ExampleOperation, ExampleSemantics, NodeType, NodeValue};
    /// # let op_ctx = OperationContext::<ExampleSemantics>::new();
    /// # let mut builder = OperationBuilder2::new(&op_ctx, 0);
    /// builder.add_operation(BuilderOpLike::LibBuiltin(LibBuiltinOperation::AddNode {value: NodeValue::Integer(42)}), vec![]).unwrap();
    /// let state = builder.show_state().unwrap();
    /// assert_eq!(state.node_keys_to_aid.len(), 0);
    /// ```
    pub fn add_operation(
        &mut self,
        op: impl Into<BuilderOpLike<S>>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::AddOperation(op.into(), args))
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

    /// Asserts that the operation being built will return a node with the given marker and abstract value.
    ///
    /// [`OperationBuilder::build`] will fail if the operation does not return a node with the given marker
    /// and abstract value.
    pub fn expect_self_return_node(
        &mut self,
        output_marker: impl Into<AbstractOutputNodeMarker>,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::SelfReturnNode(
            output_marker.into(),
            node,
        ))
    }

    /// Asserts that the operation being built will return an edge between the given source and target nodes
    /// with the given abstract value.
    ///
    /// [`OperationBuilder::build`] will fail if the operation does not return an edge with the given source and target nodes
    /// and abstract value.
    pub fn expect_self_return_edge(
        &mut self,
        src: impl Into<AbstractSignatureNodeId>,
        dst: impl Into<AbstractSignatureNodeId>,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::SelfReturnEdge(
            src.into(),
            dst.into(),
            edge,
        ))
    }

    /// Adds a diverge operation at the current point that crashes with the given message.
    ///
    /// This has special support for static analysis: If one of the two branches of a (shape or regular) query diverges,
    /// the other branch is considered to be the only branch that reaches past the query.
    ///
    /// In other words, the usual merge rules for branches do not apply, and the non-diverging branch
    /// will have its abstract state directly propagated to after the [`OperationBuilder::end_query`] instruction.
    ///
    /// # Example
    /// ```rust
    /// # use grabapl::semantics::example::ExampleSemantics;
    /// # syntax::grabapl_parse!(ExampleSemantics,
    /// fn must_return_child(parent: int) -> (child: int) {
    ///     if shape [child: int, parent -> child: *] {
    ///         // `child` node is in scope here
    ///     } else {
    ///         diverge<"no child found">();
    ///         // `child` is not in scope here
    ///     }
    ///     // `child` is in scope here, despite not existing in the abstract state at the end of the `else` branch.
    ///     return (child: child);
    /// }
    /// # );
    /// ```
    pub fn diverge(&mut self, message: impl Into<String>) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::Diverge(message.into()))
    }

    /// Adds a trace operation that will collect the runtime state of the graph
    /// and the current operation's frame.
    pub fn trace(&mut self) -> Result<(), OperationBuilderError> {
        self.push_instruction(BuilderInstruction::Trace)
    }

    /// Builds the user defined operation from the collected instructions.
    pub fn build(&mut self) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // build on a clone
        self.active.clone().build()
    }

    fn push_instruction(
        &mut self,
        instruction: BuilderInstruction<S>,
    ) -> Result<(), OperationBuilderError> {
        self.__push_instruction(instruction.clone())
            .attach_printable_lazy(move || format!("Failed to push instruction: {instruction:?}"))
    }

    fn __push_instruction(
        &mut self,
        instruction: BuilderInstruction<S>,
    ) -> Result<(), OperationBuilderError> {
        let mut new_builder_stage_1 = self.active.clone();
        // We have not modified our state, so we can just early-exit in case of error:
        new_builder_stage_1.consume(instruction.clone())?;
        // now, we know that running the instruction once did not fail. However, in presence of recursion,
        // it may fail only once a prior recursive call 'sees' the new instruction.

        let new_builder_stage_1_before_build = new_builder_stage_1.clone();
        let new_output_changes = match new_builder_stage_1.build_partial_op() {
            Ok(op) => op,
            Err(e) => {
                // we failed to _build_. This does not mean the instruction is invalid, but rather that
                // the instruction is at a partial state that cannot be built yet.
                // (e.g.: we're building the parameter graph and a context node does not have an edge to a parameter node yet)
                // TODO: indicate some *warning* to the user here?

                log::info!(
                    "Failed to build partial operation, continuing in best-effort. instruction: {:?} error: {:?}",
                    instruction,
                    e
                );

                // accept the instruction.
                // note: must take a clone of the builder, since calling build() changes it.
                // (TODO: make .build() consuming...)
                self.active = new_builder_stage_1_before_build;
                self.instructions.push(instruction);
                return Ok(());
            }
        };
        // now that we have the new self op, let's try the instruction again.
        let mut new_builder_stage_2 =
            self.build_builder_from_scratch_with_output_changes(new_output_changes)?;
        new_builder_stage_2.consume(instruction.clone())?;
        // TODO: add test that checks if maybe we change semantics by replaying all instructions with a different self op?
        // at this point we know the building worked, so we can safely update our active builder.
        // TODO: would be nice if we had an Eq constraint on BuiltinOperations, so that we could check that the result of building the stage 2 UDOp
        //  is the same as `new_self_op`. Then we know nothing changed semantically.

        self.active = new_builder_stage_2;
        self.instructions.push(instruction.clone());
        Ok(())
    }

    fn build_builder_from_scratch_with_output_changes(
        &self,
        self_output_changes: AbstractOutputChanges<S>,
    ) -> Result<Builder<'a, S>, OperationBuilderError> {
        let mut builder = Builder::new(self.op_ctx, self.self_op_id);
        builder.update_expected_self_output_changes(self_output_changes);
        for instruction in &self.instructions {
            builder.consume(instruction.clone())?;
        }

        Ok(builder)
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
> OperationBuilder2<'a, S>
{
    pub fn show_state(&self) -> Result<IntermediateState<S>, OperationBuilderError> {
        let inner = self.active.show();
        let mut intermediate_state = match inner {
            BuilderShowData::ParameterBuilder(param_builder) => {
                let param = param_builder.clone().build().unwrap();
                Ok(IntermediateState::from_param(&param))
            }
            BuilderShowData::CollectingInstructions(state) => Ok(state.clone()),
            BuilderShowData::QueryFrame(state) => Ok(state.clone()),
            BuilderShowData::BranchesFrame { true_state, .. } => {
                // we only take the true state, since we only have a branchesframe on top right after a start_query instruction.
                Ok(true_state.clone())
            }
            BuilderShowData::ShapeQueryFrame(state) => Ok(state.clone()),
            BuilderShowData::ReturnFrame(state) => Ok(state.clone()),
            BuilderShowData::Other(_) => Err(report!(OperationBuilderError::Oneoff(
                "error showing state"
            )))
            .attach_printable_lazy(|| {
                format!("Expected to receive data with intermediate state, got: {inner:?}")
            }),
        }?;

        // TODO: we could improve this now, since we actually have a full, current view of the stack.
        let query_path = self.active.stack.to_query_path();
        intermediate_state.query_path = query_path;
        Ok(intermediate_state)
    }

    pub fn format_state(&self) -> String {
        let inner = self.active.show();
        format!("{inner:?}")
    }
}

/// Merges two `AbstractOutputChanges`.
///
/// The result will be as follows:
/// - for nodes unique to one of the two, they will be kept in the result unchanged.
/// - for new nodes or edges in both, they will be kept in the result with the join of the two as the expected AV result.
/// - for nodes or edges that are changed in both, they will be kept in the result with the join of the two as the expected AV result.
/// - for nodes or edges that are deleted in at least one, they will be kept in the result as deleted and not as changed.
// TODO: do the above rules make sense? should one of the two have priority? eg. should we fail if a user expects a return type of Integer but we compute Object?
fn merge_abstract_output_changes<S: Semantics>(
    a: &AbstractOutputChanges<S>,
    b: &AbstractOutputChanges<S>,
) -> Result<AbstractOutputChanges<S>, OperationBuilderError> {
    let mut result = AbstractOutputChanges::new();
    // merge new nodes
    for (marker, av) in &a.new_nodes {
        result.new_nodes.insert(*marker, av.clone());
    }
    for (marker, av) in &b.new_nodes {
        if let Some(existing_av) = result.new_nodes.get(marker) {
            // // if the marker already exists, we join the AVs
            // let joined_av =
            //     S::NodeJoin::join(existing_av, av).ok_or(BuilderError::NeedsSpecificVariant(
            //         "Need to be able to join two different return AVs",
            //     ))?;
            // result.new_nodes.insert(*marker, joined_av);

            // if the marker already eixsts, it must be the same.
            if existing_av != av {
                bail!(OperationBuilderError::Oneoff(
                    "Mismatch in stated return node type and actual returned node type",
                ));
            }
        } else {
            // otherwise, we just insert it
            result.new_nodes.insert(*marker, av.clone());
        }
    }
    // same for new edges
    for ((src, dst), av) in &a.new_edges {
        result.new_edges.insert((*src, *dst), av.clone());
    }
    for ((src, dst), av) in &b.new_edges {
        if let Some(existing_av) = result.new_edges.get(&(*src, *dst)) {
            // // if the edge already exists, we join the AVs
            // let joined_av =
            //     S::EdgeJoin::join(existing_av, av).ok_or(BuilderError::NeedsSpecificVariant(
            //         "Need to be able to join two different return AVs",
            //     ))?;
            // result.new_edges.insert((*src, *dst), joined_av);

            // if the edge already exists, it must be the same.
            if existing_av != av {
                bail!(OperationBuilderError::Oneoff(
                    "Mismatch in expected return edge type and actual returned edge type",
                ));
            }
        } else {
            // otherwise, we just insert it
            result.new_edges.insert((*src, *dst), av.clone());
        }
    }

    // first handle deleted nodes
    for marker in &a.maybe_deleted_nodes {
        result.maybe_deleted_nodes.insert(*marker);
    }
    for marker in &b.maybe_deleted_nodes {
        result.maybe_deleted_nodes.insert(*marker);
    }
    // then handle changed nodes
    for (marker, av) in &a.maybe_changed_nodes {
        // only if it's not deleted
        if result.maybe_deleted_nodes.contains(marker) {
            continue;
        }
        result.maybe_changed_nodes.insert(*marker, av.clone());
    }
    for (marker, av) in &b.maybe_changed_nodes {
        // only if it's not deleted
        if result.maybe_deleted_nodes.contains(marker) {
            continue;
        }
        if let Some(existing_av) = result.maybe_changed_nodes.get(marker) {
            // if the marker already exists, we join the AVs
            let joined_av =
                S::NodeJoin::join(existing_av, av).ok_or(OperationBuilderError::Oneoff(
                    "Need to be able to join two different maybe_changed AVs",
                ))?;
            result.maybe_changed_nodes.insert(*marker, joined_av);
        } else {
            // otherwise, we just insert it
            result.maybe_changed_nodes.insert(*marker, av.clone());
        }
    }

    // first handle deleted edges
    for (src, dst) in &a.maybe_deleted_edges {
        result.maybe_deleted_edges.insert((*src, *dst));
    }
    for (src, dst) in &b.maybe_deleted_edges {
        result.maybe_deleted_edges.insert((*src, *dst));
    }
    // then handle changed edges
    for ((src, dst), av) in &a.maybe_changed_edges {
        // only if it's not deleted
        if result.maybe_deleted_edges.contains(&(*src, *dst)) {
            continue;
        }
        // if one of the two endpoints may be deleted, then the edge may be deleted as well.
        if result.maybe_deleted_nodes.contains(src) || result.maybe_deleted_nodes.contains(dst) {
            result.maybe_deleted_edges.insert((*src, *dst));
            continue;
        }

        // the edge stays, so we can insert it
        result.maybe_changed_edges.insert((*src, *dst), av.clone());
    }
    for ((src, dst), av) in &b.maybe_changed_edges {
        // only if it's not deleted
        if result.maybe_deleted_edges.contains(&(*src, *dst)) {
            continue;
        }
        // if one of the two endpoints may be deleted, then the edge may be deleted as well.
        if result.maybe_deleted_nodes.contains(src) || result.maybe_deleted_nodes.contains(dst) {
            result.maybe_deleted_edges.insert((*src, *dst));
            continue;
        }

        // the edge stays, so we can insert it
        if let Some(existing_av) = result.maybe_changed_edges.get(&(*src, *dst)) {
            // if the edge already exists, we join the AVs
            let joined_av =
                S::EdgeJoin::join(existing_av, av).ok_or(OperationBuilderError::Oneoff(
                    "Need to be able to join two different maybe_changed AVs",
                ))?;
            result.maybe_changed_edges.insert((*src, *dst), joined_av);
        } else {
            // otherwise, we just insert it
            result.maybe_changed_edges.insert((*src, *dst), av.clone());
        }
    }

    Ok(result)
}
