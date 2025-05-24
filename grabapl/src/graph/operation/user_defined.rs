use std::collections::HashMap;
use std::rc::Rc;
use derive_more::with_trait::From;
use crate::graph::pattern::{AbstractOutputNodeMarker, OperationArgument, OperationOutput, OperationParameter, ParameterSubstition};
use crate::{NodeKey, OperationContext, OperationId, Semantics, SubstMarker};
use crate::graph::operation::{run_builtin_operation, run_operation, OperationError, OperationResult};
use crate::graph::operation::query::{run_builtin_query, BuiltinQuery};
use crate::graph::semantics::{ConcreteGraph, SemanticsClone};

/// These represent the _abstract_ (guaranteed) shape changes of an operation, bundled together.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, From)]
pub struct AbstractOperationResultMarker(pub &'static str);


/// Identifies a node in the user defined operation view.
#[derive(Copy, Clone, From)]
pub enum AbstractNodeId {
    /// A node in the parameter graph.
    ParameterMarker(SubstMarker),
    /// A node that was created as a result of another operation.
    DynamicOutputMarker(AbstractOperationResultMarker, AbstractOutputNodeMarker),
}

pub type InstructionWithResultMarker<S> = (AbstractOperationResultMarker, Instruction<S>);

// A 'custom'/user-defined operation
pub struct UserDefinedOperation<S: Semantics> {
    pub parameter: OperationParameter<S>,
    // TODO: add preprocessing (checking) step to see if the instructions make sense and are well formed wrt which nodes they access statically.
    pub instructions: Vec<InstructionWithResultMarker<S>>,
}

impl<S: SemanticsClone> UserDefinedOperation<S> {
    pub(crate) fn apply(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut ConcreteGraph<S>,
        argument: OperationArgument,
        subst: &ParameterSubstition,
    ) -> OperationResult<OperationOutput> {
        let mut our_output_map: HashMap<AbstractOutputNodeMarker, NodeKey> = HashMap::new();

        let mut previous_results: HashMap<AbstractOperationResultMarker, HashMap<AbstractOutputNodeMarker, NodeKey>> = HashMap::new();

        run_instructions(
            g,
            &mut previous_results,
            &mut our_output_map,
            op_ctx,
            &self.instructions,
            subst,
        )?;

        // TODO: How to define a good output here?
        //  probably should be part of the UserDefinedOperation struct. AbstractNodeId should be used, and then we get the actual node key based on what's happening.
        Ok(OperationOutput {
            new_nodes: our_output_map,
        })
    }
}

fn run_instructions<S: SemanticsClone>(
    g: &mut ConcreteGraph<S>,
    previous_results: &mut HashMap<AbstractOperationResultMarker, HashMap<AbstractOutputNodeMarker, NodeKey>>,
    our_output_map: &mut HashMap<AbstractOutputNodeMarker, NodeKey>,
    op_ctx: &OperationContext<S>,
    instructions: &[InstructionWithResultMarker<S>],
    subst: &ParameterSubstition,
) -> OperationResult<()> {
    for (abstract_output_id, instruction) in instructions {
        match instruction {
            oplike@(Instruction::Operation(_, args) | Instruction::Builtin(_, args)) => {
                let concrete_args = get_concrete_args::<S>(args, subst, previous_results)?;
                // TODO: make fallible
                // TODO: How do we support mutually recursive user defined operations?
                //  - I think just specifying the ID directly? this will mainly be a problem for the OperationBuilder
                let output = match oplike {
                    Instruction::Operation(op_id, _) => {
                        run_operation::<S>(
                            g,
                            op_ctx,
                            *op_id,
                            concrete_args,
                        )?
                    }
                    Instruction::Builtin(op, _) => {
                        run_builtin_operation::<S>(
                            g,
                            op,
                            concrete_args,
                        )?
                    }
                    // does not match the outer match arm
                    Instruction::BuiltinQuery(..) => unreachable!()
                };

                previous_results.insert(*abstract_output_id, output.new_nodes);
            }
            Instruction::BuiltinQuery(query, args, query_instr) => {
                let concrete_args = get_concrete_args::<S>(args, subst, previous_results)?;
                let result = run_builtin_query::<S>(g, query, concrete_args)?;
                let next_instr = if result.taken {
                    &query_instr.taken
                } else {
                    &query_instr.not_taken
                };
                // TODO: don't use function stack (ie, dont recurse), instead use explicit stack
                run_instructions(
                    g,
                    previous_results,
                    our_output_map,
                    op_ctx,
                    next_instr,
                    subst,
                )?
            }
        }
    }
    Ok(())
}

fn get_concrete_args<S: Semantics>(
    args: &[AbstractNodeId],
    subst: &ParameterSubstition,
    previous_results: &HashMap<AbstractOperationResultMarker, HashMap<AbstractOutputNodeMarker, NodeKey>>,
) -> OperationResult<Vec<NodeKey>> {
    args.iter().map(|arg| match arg {
        AbstractNodeId::ParameterMarker(subst_marker) => subst.mapping.get(subst_marker).copied().ok_or(OperationError::UnknownParameterMarker(*subst_marker)),
        AbstractNodeId::DynamicOutputMarker(output_id, output_marker) => {
            let output_map = previous_results.get(output_id).ok_or(OperationError::UnknownOperationResultMarker(*output_id))?;
            output_map.get(output_marker)
                .copied()
                .ok_or(OperationError::UnknownOutputNodeMarker(*output_marker))
        }
    }).collect()
}


pub enum Instruction<S: Semantics> {
    // TODO: Split out into Instruction::OperationLike (which includes both Builtin and Operation)
    //  and Instruction::QueryLike (which includes BuiltinQuery and potential future custom queries).
    Builtin(S::BuiltinOperation, Vec<AbstractNodeId>),
    Operation(OperationId, Vec<AbstractNodeId>),
    BuiltinQuery(S::BuiltinQuery, Vec<AbstractNodeId>, QueryInstructions<S>),
}

pub struct QueryInstructions<S: Semantics> {
    pub taken: Vec<InstructionWithResultMarker<S>>,
    pub not_taken: Vec<InstructionWithResultMarker<S>>,
}

// What happens when the query results in true.
//
// Analogy in Rust:
// ```
// if let Pattern(_) = query { block }
// ```
pub struct QueryTaken<S: Semantics> {
    // The pattern changes are applied to the abstract graph in sequence. Analogy: the "let Pattern" part
    // pub pattern_changes: Vec<PatternChange<S>>,
    // With the new abstract graph, run these instructions. Analogy: the "block" part
    pub instructions: Vec<Instruction<S>>,
}

// These may refer to the original query input somehow.
// For example, we may have a "Has child?" query that:
//  1. ExpectNode(Child)
//  2. ExpectEdge(Parent, Child)
// But "Parent" is a free variable here, hence must somehow come from the query input. Unsure how yet.
pub enum PatternChange<S: Semantics> {
    ExpectNode(NodeChangePattern<S>),
    ExpectEdge(EdgeChangePattern<S>),
}

pub enum NodeChangePattern<S: Semantics> {
    NewNode(SubstMarker, S::NodeAbstract),
}

pub enum EdgeChangePattern<S: Semantics> {
    NewEdge {
        from: SubstMarker,
        to: SubstMarker,
        abstract_value: S::EdgeAbstract,
    },
}