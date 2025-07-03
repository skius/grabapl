use crate::graph::operation::query::{
    BuiltinQuery, GraphShapeQuery, ShapeNodeIdentifier, run_builtin_query, run_shape_query,
};
use crate::graph::operation::signature::{AbstractSignatureNodeId, OperationSignature};
use crate::graph::operation::{
    OperationError, OperationResult, run_builtin_operation, run_operation,
};
use crate::graph::pattern::{
    AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, NodeMarker,
    OperationArgument, OperationOutput, OperationParameter, ParameterSubstitution,
};
use crate::graph::semantics::{AbstractGraph, ConcreteGraph, SemanticsClone};
use crate::util::bimap::BiMap;
use crate::{
    NodeKey, OperationContext, OperationId, Semantics, SubstMarker, interned_string_newtype,
};
use derive_more::with_trait::From;
use internment::Intern;
use std::collections::HashMap;
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

/// Identifies a node in the user defined operation view.
#[derive(Clone, Copy, From, Debug, Eq, PartialEq, Hash)]
pub enum AbstractNodeId {
    /// A node in the parameter graph.
    ParameterMarker(SubstMarker),
    /// A node that was created as a result of another operation.
    DynamicOutputMarker(AbstractOperationResultMarker, AbstractOutputNodeMarker),
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
pub enum Instruction<S: Semantics> {
    // TODO: Split out into Instruction::OperationLike (which includes both Builtin and Operation)
    //  and Instruction::QueryLike (which includes BuiltinQuery and potential future custom queries).
    #[debug("Builtin(???, {_1:#?})")]
    Builtin(S::BuiltinOperation, AbstractOperationArgument),
    #[debug("Operation({_0:#?}, {_1:#?})")]
    Operation(OperationId, AbstractOperationArgument),
    #[debug("BuiltinQuery(???, {_1:#?}, {_2:#?})")]
    BuiltinQuery(
        S::BuiltinQuery,
        AbstractOperationArgument,
        QueryInstructions<S>,
    ),
    #[debug("ShapeQuery(???, {_1:#?}, {_2:#?})")]
    ShapeQuery(
        GraphShapeQuery<S>,
        Vec<AbstractNodeId>,
        QueryInstructions<S>,
    ),
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

impl<S: SemanticsClone> UserDefinedOperation<S> {
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
        let mut output_names = BiMap::new();

        // handle new nodes
        for (aid, (name, av)) in &self.output_changes.new_nodes {
            let nnm = g.new_node_marker();
            g.add_node(nnm.clone(), av.clone());
            output_names.insert(nnm, name.clone());
        }

        let sig_id_to_node_marker = |sig_id: AbstractSignatureNodeId| {
            match sig_id {
                AbstractSignatureNodeId::ExistingNode(subst) => NodeMarker::Subst(subst),
                AbstractSignatureNodeId::NewNode(name) => {
                    // find in output_names
                    let nnm = output_names
                        .get_right(&name)
                        .expect("internal error: signature node not found in output names");
                    NodeMarker::New(*nnm)
                }
            }
        };

        // handle new edges
        for ((src, dst), av) in &self.signature.output.new_edges {
            let src_marker = sig_id_to_node_marker(*src);
            let dst_marker = sig_id_to_node_marker(*dst);
            g.add_edge(src_marker, dst_marker, av.clone());
        }

        // handle changed nodes
        for (subst, av) in &self.signature.output.changed_nodes {
            let node_marker = NodeMarker::Subst(*subst);
            g.set_node_value(node_marker, av.clone()).unwrap();
        }
        // handle changed edges
        for ((src, dst), av) in &self.signature.output.changed_edges {
            let src_marker = NodeMarker::Subst(*src);
            let dst_marker = NodeMarker::Subst(*dst);
            g.set_edge_value(src_marker, dst_marker, av.clone())
                .unwrap();
        }

        // handle removed nodes
        for subst in &self.signature.output.deleted_nodes {
            let node_marker = NodeMarker::Subst(*subst);
            g.delete_node(node_marker);
        }
        // handle removed edges
        for (src, dst) in &self.signature.output.deleted_edges {
            let src_marker = NodeMarker::Subst(*src);
            let dst_marker = NodeMarker::Subst(*dst);
            g.delete_edge(src_marker, dst_marker);
        }

        let (output_names, _) = output_names.into_inner();
        Ok(g.get_abstract_output(output_names))
    }

    pub(crate) fn apply(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut ConcreteGraph<S>,
        subst: &ParameterSubstitution,
    ) -> OperationResult<OperationOutput> {
        let mut our_output_map: HashMap<AbstractOutputNodeMarker, NodeKey> = HashMap::new();

        let mut previous_results: HashMap<
            AbstractOperationResultMarker,
            HashMap<AbstractOutputNodeMarker, NodeKey>,
        > = HashMap::new();

        run_instructions(
            g,
            &mut previous_results,
            &mut our_output_map,
            op_ctx,
            &self.instructions,
            subst,
        )?;

        for (aid, (name, _)) in &self.output_changes.new_nodes {
            let node_key = aid_to_node_key(aid.clone(), subst, &previous_results)?;
            our_output_map.insert(name.clone(), node_key);
        }

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

fn run_instructions<S: SemanticsClone>(
    g: &mut ConcreteGraph<S>,
    previous_results: &mut HashMap<
        AbstractOperationResultMarker,
        HashMap<AbstractOutputNodeMarker, NodeKey>,
    >,
    our_output_map: &mut HashMap<AbstractOutputNodeMarker, NodeKey>,
    op_ctx: &OperationContext<S>,
    instructions: &[InstructionWithResultMarker<S>],
    subst: &ParameterSubstitution,
) -> OperationResult<()> {
    for (abstract_output_id, instruction) in instructions {
        match instruction {
            oplike @ (Instruction::Operation(_, arg) | Instruction::Builtin(_, arg)) => {
                let concrete_arg = get_concrete_arg::<S>(
                    &arg.selected_input_nodes,
                    &arg.subst_to_aid,
                    subst,
                    previous_results,
                )?;
                // TODO: make fallible
                // TODO: How do we support mutually recursive user defined operations?
                //  - I think just specifying the ID directly? this will mainly be a problem for the OperationBuilder
                //  - we need some ExecutionContext that potentially stores information like fuel (to avoid infinite loops and timing out)
                let output = match oplike {
                    Instruction::Operation(op_id, _) => {
                        run_operation::<S>(g, op_ctx, *op_id, concrete_arg)?
                    }
                    Instruction::Builtin(op, _) => run_builtin_operation::<S>(g, op, concrete_arg)?,
                    // does not match the outer match arm
                    Instruction::BuiltinQuery(..) | Instruction::ShapeQuery(..) => unreachable!(),
                };
                if let Some(abstract_output_id) = abstract_output_id {
                    previous_results.insert(abstract_output_id.clone(), output.new_nodes);
                    // TODO: also handle output.removed_nodes.
                }
            }
            Instruction::BuiltinQuery(query, arg, query_instr) => {
                let concrete_arg = get_concrete_arg::<S>(
                    &arg.selected_input_nodes,
                    &arg.subst_to_aid,
                    subst,
                    previous_results,
                )?;
                let result = run_builtin_query::<S>(g, query, concrete_arg)?;
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
            Instruction::ShapeQuery(query, args, query_instr) => {
                // ShapeQueries dont have context mappings, so we can just pass an empty hashmap.
                let concrete_arg =
                    get_concrete_arg::<S>(args, &HashMap::new(), subst, previous_results)?;
                let result = run_shape_query(g, query, &concrete_arg.selected_input_nodes)?;
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
                            previous_results.insert(abstract_output_id.clone(), query_result_map);
                        }

                        &query_instr.taken
                    } else {
                        &query_instr.not_taken
                    };
                run_instructions(
                    g,
                    previous_results,
                    our_output_map,
                    op_ctx,
                    next_instr,
                    subst,
                )?;
            }
        }
    }
    Ok(())
}

// TODO: decide if we really want to have this be fallible, since we may want to instead have some
//  invariant that this works. And encode fallibility in a 'builder'.
fn get_concrete_arg<S: Semantics>(
    explicit_args: &[AbstractNodeId],
    context_mapping: &HashMap<SubstMarker, AbstractNodeId>,
    subst: &ParameterSubstitution,
    previous_results: &HashMap<
        AbstractOperationResultMarker,
        HashMap<AbstractOutputNodeMarker, NodeKey>,
    >,
) -> OperationResult<OperationArgument<'static>> {
    let selected_keys: Vec<NodeKey> = explicit_args
        .iter()
        .map(|arg| aid_to_node_key(arg.clone(), subst, previous_results))
        .collect::<OperationResult<_>>()?;

    let subst = ParameterSubstitution::new(
        context_mapping
            .iter()
            .map(|(subst_marker, abstract_node_id)| {
                Ok((
                    subst_marker.clone(),
                    aid_to_node_key(abstract_node_id.clone(), subst, previous_results)?,
                ))
            })
            .collect::<OperationResult<_>>()?,
    );

    Ok(OperationArgument {
        selected_input_nodes: selected_keys.into(),
        subst,
    })
}

fn aid_to_node_key(
    aid: AbstractNodeId,
    subst: &ParameterSubstitution,
    previous_results: &HashMap<
        AbstractOperationResultMarker,
        HashMap<AbstractOutputNodeMarker, NodeKey>,
    >,
) -> OperationResult<NodeKey> {
    match aid {
        AbstractNodeId::ParameterMarker(subst_marker) => subst
            .mapping
            .get(&subst_marker)
            .copied()
            .ok_or(OperationError::UnknownParameterMarker(subst_marker)),
        AbstractNodeId::DynamicOutputMarker(output_id, output_marker) => {
            let output_map = previous_results
                .get(&output_id)
                .ok_or(OperationError::UnknownOperationResultMarker(output_id))?;
            output_map
                .get(&output_marker)
                .copied()
                .ok_or(OperationError::UnknownOutputNodeMarker(output_marker))
        }
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
