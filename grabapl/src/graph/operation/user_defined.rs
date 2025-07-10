use crate::graph::operation::builtin::LibBuiltinOperation;
use crate::graph::operation::query::{
    BuiltinQuery, GraphShapeQuery, ShapeNodeIdentifier, run_builtin_query, run_shape_query,
};
use crate::graph::operation::signature::{AbstractSignatureNodeId, OperationSignature};
use crate::graph::operation::{
    OperationError, OperationResult, run_builtin_operation, run_lib_builtin_operation,
    run_operation,
};
use crate::graph::pattern::{
    AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, NodeMarker,
    OperationArgument, OperationOutput, OperationParameter, ParameterSubstitution,
};
use crate::graph::semantics::{AbstractGraph, ConcreteGraph};
use crate::util::bimap::BiMap;
use crate::util::log;
use crate::{
    NodeKey, OperationContext, OperationId, Semantics, SubstMarker, interned_string_newtype,
};
use derive_more::with_trait::From;
use internment::Intern;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::str::FromStr;

/// These represent the _abstract_ (guaranteed) shape changes of an operation, bundled together.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, From)]
pub enum AbstractOperationResultMarker {
    Custom(Intern<String>),
    // NOTE: this may not be created by the user! since this is an unstable index, if the user
    // reorders operations, this marker may suddenly point to a different operation result.
    // Custom markers must always be used for arguments!
    #[from(ignore)]
    Implicit(u64),
}
interned_string_newtype!(
    AbstractOperationResultMarker,
    AbstractOperationResultMarker::Custom
);

#[derive(derive_more::Debug, Clone, Copy, Hash, Eq, PartialEq, From)]
#[debug("N({_0})")]
pub struct NamedMarker(Intern<String>);
interned_string_newtype!(NamedMarker);

/// Identifies a node in the user defined operation view.
#[derive(Clone, Copy, From, Debug, Eq, PartialEq, Hash)]
pub enum AbstractNodeId {
    /// A node in the parameter graph.
    ParameterMarker(SubstMarker),
    /// A node that was created as a result of another operation.
    DynamicOutputMarker(AbstractOperationResultMarker, AbstractOutputNodeMarker),
    /// A node that was given an explicit name. Parameters cannot be renamed.
    Named(NamedMarker),
}

impl AbstractNodeId {
    pub fn param(m: impl Into<SubstMarker>) -> Self {
        AbstractNodeId::ParameterMarker(m.into())
    }

    pub fn dynamic_output(
        output_id: impl Into<AbstractOperationResultMarker>,
        output_marker: impl Into<AbstractOutputNodeMarker>,
    ) -> Self {
        let output_id = output_id.into();
        let output_marker = output_marker.into();
        AbstractNodeId::DynamicOutputMarker(output_id, output_marker)
    }

    pub fn named(name: impl Into<NamedMarker>) -> Self {
        AbstractNodeId::Named(name.into())
    }
}

impl FromStr for AbstractNodeId {
    type Err = ();

    // TODO: add tests for this
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse the two options into the enum variants:
        //  1. "P(<marker:string>)" for ParameterMarker
        //  2. "O(<output_id:string>, <output_marker:string>)" for DynamicOutputMarker
        // Note the inner strings may not contain (, ), or commas to make it easier for us.
        if let Some(stripped) = s.strip_prefix("P(").and_then(|s| s.strip_suffix(')')) {
            Ok(AbstractNodeId::param(stripped))
        } else if let Some(stripped) = s.strip_prefix("O(").and_then(|s| s.strip_suffix(')')) {
            let mut parts = stripped.split(',');
            if let (Some(output_id), Some(output_marker), None) =
                (parts.next(), parts.next(), parts.next())
            {
                let output_id = output_id.trim();
                let output_marker = output_marker.trim();
                Ok(AbstractNodeId::dynamic_output(output_id, output_marker))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}

/// Represents the abstract nodes that will be passed to an operation.
///
/// The mapping for the implicitly matched context graph *needs* to be stored statically,
/// since we define our operation parameters to be matched statically.
#[derive(Debug, Clone)]
pub struct AbstractOperationArgument {
    /// The nodes that were selected explicitly as input to the operation.
    pub selected_input_nodes: Vec<AbstractNodeId>,
    /// A mapping from the parameter's implicitly matched context nodes to the statically matched
    /// nodes from our abstract graph.
    pub subst_to_aid: HashMap<SubstMarker, AbstractNodeId>,
}

impl AbstractOperationArgument {
    pub fn new() -> Self {
        AbstractOperationArgument {
            selected_input_nodes: Vec::new(),
            subst_to_aid: HashMap::new(),
        }
    }

    pub fn new_for_shape_query(explicit_nodes: Vec<AbstractNodeId>) -> Self {
        AbstractOperationArgument {
            selected_input_nodes: explicit_nodes,
            subst_to_aid: HashMap::new(),
        }
    }

    pub fn infer_explicit_for_param(
        selected_nodes: Vec<AbstractNodeId>,
        param: &OperationParameter<impl Semantics>,
    ) -> OperationResult<Self> {
        if param.explicit_input_nodes.len() != selected_nodes.len() {
            return Err(OperationError::InvalidOperationArgumentCount {
                expected: param.explicit_input_nodes.len(),
                actual: selected_nodes.len(),
            });
        }

        let subst = param
            .explicit_input_nodes
            .iter()
            .zip(selected_nodes.iter())
            .map(|(subst_marker, node_key)| (subst_marker.clone(), node_key.clone()))
            .collect();
        Ok(AbstractOperationArgument {
            selected_input_nodes: selected_nodes,
            subst_to_aid: subst,
        })
    }
}

#[derive(derive_more::Debug)]
pub enum OpLikeInstruction<S: Semantics> {
    #[debug("Builtin(???)")]
    Builtin(S::BuiltinOperation),
    #[debug("LibBuiltin({_0:?})")]
    LibBuiltin(LibBuiltinOperation<S>),
    #[debug("Operation({_0:#?})")]
    Operation(OperationId),
}

#[derive(derive_more::Debug)]
pub enum Instruction<S: Semantics> {
    #[debug("OpLike({_0:#?}, {_1:#?})")]
    OpLike(OpLikeInstruction<S>, AbstractOperationArgument),
    // TODO: Split into Instruction::QueryLike (which includes BuiltinQuery and potential future custom queries).
    #[debug("BuiltinQuery(???, {_1:#?}, {_2:#?})")]
    BuiltinQuery(
        S::BuiltinQuery,
        AbstractOperationArgument,
        QueryInstructions<S>,
    ),
    #[debug("ShapeQuery(???, {_1:#?}, {_2:#?})")]
    ShapeQuery(
        GraphShapeQuery<S>,
        // Note: a shape query should have no abstract, implicitly matched argument nodes. Hence the subst mapping in the argument is just for the explicitly selected nodes.
        AbstractOperationArgument,
        QueryInstructions<S>,
    ),
    #[debug("RenameNode({old:#?} ==> {new:#?})")]
    RenameNode {
        old: AbstractNodeId,
        new: AbstractNodeId,
    },
}

#[derive(derive_more::Debug)]
pub struct QueryInstructions<S: Semantics> {
    // TODO: does it make sense to rename these? true_branch and false_branch?
    #[debug("[{}]", taken.iter().map(|(opt, inst)| format!("({opt:#?}, {:#?})", inst)).collect::<Vec<_>>().join(", "))]
    pub taken: Vec<InstructionWithResultMarker<S>>,
    #[debug("[{}]", not_taken.iter().map(|(opt, inst)| format!("({opt:#?}, {:#?})", inst)).collect::<Vec<_>>().join(", "))]
    pub not_taken: Vec<InstructionWithResultMarker<S>>,
}

pub type InstructionWithResultMarker<S> = (Option<AbstractOperationResultMarker>, Instruction<S>);

// TODO: We probably want each instruction to statically know which nodes it uses in a call. We need this because
//  we want the parameter matching to happen statically, so we know for a fact which nodes get modified. And we're not surprised
//  if the concrete graph has more edges and thus the called operation matches differently.
//  This requires thinking about how to keep statically defined mappings in check when running the operation concretely.
//  ==> see big-picture-todos.md for a solution. TL;DR: store implicitly matched context nodes in the form of an explicit mapping from AbstractNodeId to the context nodes.

pub struct AbstractUserDefinedOperationOutput<S: Semantics> {
    // TODO: can probably remove S::NodeAbstract here since it's in the signature.
    pub new_nodes: HashMap<AbstractNodeId, (AbstractOutputNodeMarker, S::NodeAbstract)>,
}

impl<S: Semantics> AbstractUserDefinedOperationOutput<S> {
    pub fn new() -> Self {
        AbstractUserDefinedOperationOutput {
            new_nodes: HashMap::new(),
        }
    }
}

// A 'custom'/user-defined operation
pub struct UserDefinedOperation<S: Semantics> {
    pub parameter: OperationParameter<S>,
    // cached signature. there is definitely some duplicated information here.
    pub signature: OperationSignature<S>,
    // TODO: add preprocessing (checking) step to see if the instructions make sense and are well formed wrt which nodes they access statically.
    pub instructions: Vec<InstructionWithResultMarker<S>>,
    // TODO: need to define output changes.
    pub output_changes: AbstractUserDefinedOperationOutput<S>,
}

// TODO: use a private runner struct that keeps all the necessary mappings on self for easier methods.

impl<S: Semantics> UserDefinedOperation<S> {
    pub fn new_noop() -> Self {
        let parameter = OperationParameter::new_empty();
        let signature = OperationSignature::empty_new("noop", parameter.clone());
        UserDefinedOperation {
            parameter,
            signature,
            instructions: Vec::new(),
            output_changes: AbstractUserDefinedOperationOutput::new(),
        }
    }

    pub fn new(
        parameter: OperationParameter<S>,
        instructions: Vec<InstructionWithResultMarker<S>>,
    ) -> Self {
        let signature = OperationSignature::empty_new("some_name", parameter.clone());
        UserDefinedOperation {
            parameter,
            signature,
            instructions,
            output_changes: AbstractUserDefinedOperationOutput::new(),
        }
    }

    pub(crate) fn apply_abstract(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> OperationResult<AbstractOperationOutput<S>> {
        Ok(self.signature.output.apply_abstract(g))
    }

    pub(crate) fn apply(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut ConcreteGraph<S>,
        arg: &OperationArgument,
    ) -> OperationResult<OperationOutput> {
        let mut runner = Runner::new(op_ctx, g, arg);
        runner.run(&self.instructions)?;

        let our_output_map = self
            .output_changes
            .new_nodes
            .iter()
            .map(|(aid, (name, _))| Ok((*name, runner.aid_to_node_key(*aid)?)))
            .collect::<OperationResult<_>>()?;

        // TODO: How to define a good output here?
        //  probably should be part of the UserDefinedOperation struct. AbstractNodeId should be used, and then we get the actual node key based on what's happening.
        Ok(OperationOutput {
            new_nodes: our_output_map,
            // TODO: populate this
            removed_nodes: vec![],
        })
    }

    pub fn signature(&self) -> OperationSignature<S> {
        // TODO: borrow
        self.signature.clone()
    }
}

/// Runs a user defined operation.
struct Runner<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    g: &'a mut ConcreteGraph<S>,
    /// The argument with which our operation was called.
    arg: &'a OperationArgument<'a>,
    // Note: should not store AID::Parameter nodes, those are in `arg` already.
    abstract_to_concrete: HashMap<AbstractNodeId, NodeKey>,
}

impl<'a, S: Semantics> Runner<'a, S> {
    pub fn new(
        op_ctx: &'a OperationContext<S>,
        g: &'a mut ConcreteGraph<S>,
        arg: &'a OperationArgument<'a>,
    ) -> Self {
        Runner {
            op_ctx,
            g,
            arg,
            abstract_to_concrete: HashMap::new(),
        }
    }

    fn run(&mut self, instructions: &[InstructionWithResultMarker<S>]) -> OperationResult<()> {
        for (abstract_output_id, instruction) in instructions {
            match instruction {
                Instruction::OpLike(oplike, arg) => {
                    let concrete_arg = self.abstract_to_concrete_arg(arg)?;
                    log::trace!("Resulting concrete arg: {concrete_arg:#?}");
                    // TODO: How do we support *mutually* recursive user defined operations?
                    //  - I think just specifying the ID directly? this will mainly be a problem for the OperationBuilder
                    // TODO: we need some ExecutionContext that potentially stores information like fuel (to avoid infinite loops and timing out)
                    let output = match oplike {
                        OpLikeInstruction::Operation(op_id) => {
                            run_operation::<S>(self.g, self.op_ctx, *op_id, concrete_arg)?
                        }
                        OpLikeInstruction::Builtin(op) => {
                            run_builtin_operation::<S>(self.g, op, concrete_arg)?
                        }
                        OpLikeInstruction::LibBuiltin(op) => {
                            run_lib_builtin_operation(self.g, op, concrete_arg)?
                        }
                    };
                    if let Some(abstract_output_id) = abstract_output_id {
                        self.extend_abstract_mapping(abstract_output_id.clone(), output.new_nodes);
                        // TODO: also handle output.removed_nodes.
                    }
                }
                Instruction::BuiltinQuery(query, arg, query_instr) => {
                    let concrete_arg = self.abstract_to_concrete_arg(arg)?;
                    let result = run_builtin_query::<S>(self.g, query, concrete_arg)?;
                    let next_instr = if result.taken {
                        &query_instr.taken
                    } else {
                        &query_instr.not_taken
                    };
                    // TODO: don't use function stack (ie, dont recurse), instead use explicit stack
                    self.run(next_instr)?
                }
                Instruction::ShapeQuery(query, arg, query_instr) => {
                    let concrete_arg = self.abstract_to_concrete_arg(&arg)?;
                    let result = run_shape_query(
                        self.g,
                        query,
                        &concrete_arg.selected_input_nodes,
                        &concrete_arg.hidden_nodes,
                    )?;
                    let next_instr =
                        if let Some(shape_idents_to_node_keys) = result.shape_idents_to_node_keys {
                            // apply the shape idents to node keys mapping

                            let mut query_result_map = HashMap::new();
                            for (ident, node_key) in shape_idents_to_node_keys {
                                // TODO: add helper function, or add new variant to AbstractOutputNodeMarker, or just use that one for the shape query mapping and get rid of ShapeNodeIdentifier.
                                let output_marker = AbstractOutputNodeMarker(ident.into());
                                query_result_map.insert(output_marker, node_key);
                            }
                            if let Some(abstract_output_id) = abstract_output_id {
                                self.extend_abstract_mapping(
                                    abstract_output_id.clone(),
                                    query_result_map,
                                );
                            }

                            &query_instr.taken
                        } else {
                            &query_instr.not_taken
                        };
                    self.run(next_instr)?;
                }
                Instruction::RenameNode { old, new } => {
                    let Some(key) = self.abstract_to_concrete.remove(old) else {
                        return Err(OperationError::UnknownAID(*old));
                    };
                    self.abstract_to_concrete.insert(*old, key);
                }
            }
        }
        Ok(())
    }

    fn extend_abstract_mapping(
        &mut self,
        abstract_output_id: AbstractOperationResultMarker,
        output_map: HashMap<AbstractOutputNodeMarker, NodeKey>,
    ) {
        for (marker, node_key) in output_map {
            self.abstract_to_concrete.insert(
                AbstractNodeId::DynamicOutputMarker(abstract_output_id, marker),
                node_key,
            );
        }
    }

    fn aid_to_node_key(&self, aid: AbstractNodeId) -> OperationResult<NodeKey> {
        // Get a param aid from our argument's substitution, and the rest from the map.
        match aid {
            AbstractNodeId::ParameterMarker(subst_marker) => self
                .arg
                .subst
                .mapping
                .get(&subst_marker)
                .copied()
                .ok_or(OperationError::UnknownParameterMarker(subst_marker)),
            AbstractNodeId::DynamicOutputMarker(..) | AbstractNodeId::Named(..) => {
                let key = self
                    .abstract_to_concrete
                    .get(&aid)
                    .copied()
                    .ok_or(OperationError::UnknownAID(aid))?;
                Ok(key)
            }
        }
    }

    // TODO: decide if we really want to have this be fallible, since we may want to instead have some
    //  invariant that this works. And encode fallibility in a 'builder'
    fn abstract_to_concrete_arg(
        &self,
        arg: &AbstractOperationArgument,
    ) -> OperationResult<OperationArgument<'static>> {
        log::trace!(
            "Getting concrete arg of abstract arg: {arg:#?} previous_results: {:#?}, our operation's argument: {:#?}",
            &self.abstract_to_concrete,
            &self.arg,
        );
        let selected_keys: Vec<NodeKey> = arg
            .selected_input_nodes
            .iter()
            .map(|arg| self.aid_to_node_key(*arg))
            .collect::<OperationResult<_>>()?;

        let new_subst = ParameterSubstitution::new(
            arg.subst_to_aid
                .iter()
                .map(|(subst_marker, abstract_node_id)| {
                    Ok((
                        subst_marker.clone(),
                        self.aid_to_node_key(abstract_node_id.clone())?,
                    ))
                })
                .collect::<OperationResult<_>>()?,
        );

        let hidden_nodes: HashSet<_> = self
            .arg
            .subst
            .mapping
            .values()
            .copied()
            .chain(self.abstract_to_concrete.values().copied())
            .chain(self.arg.hidden_nodes.iter().copied())
            .collect();

        Ok(OperationArgument {
            selected_input_nodes: selected_keys.into(),
            subst: new_subst,
            hidden_nodes,
        })
    }
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
