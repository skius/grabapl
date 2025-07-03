pub mod builder;
pub mod parameterbuilder;
pub mod query;
pub mod user_defined;
pub mod signature;

use crate::graph::EdgeAttribute;
use crate::graph::operation::user_defined::{AbstractOperationResultMarker, UserDefinedOperation};
use crate::graph::pattern::{AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, OperationArgument, OperationOutput, OperationParameter, ParameterSubstitution};
use crate::graph::semantics::{
    AbstractGraph, AbstractMatcher, ConcreteGraph, ConcreteToAbstract, Semantics, SemanticsClone,
};
use crate::{DotCollector, Graph, NodeKey, SubstMarker};
use petgraph::algo::general_subgraph_monomorphisms_iter;
use petgraph::visit::NodeIndexable;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use thiserror::Error;

// TODO: We might want to be able to supply additional data to builtin operations. For example, a Set Value operation should be 'generic' over its value without
//  needing to store a separate operation in the OpCtx for every value...
pub trait BuiltinOperation: Debug {
    type S: Semantics;

    /// The pattern to match against the graph.
    fn parameter(&self) -> OperationParameter<Self::S>;

    // TODO: needs an apply_abstract operation that applies the changes to the abstract graph.
    // For example, "add node" adds the node.
    // In general, we still need a way to refer to new changes, e.g., how do we refer
    // to a new node added by an operation?
    // In a frontend that's easy, the user 'sees' the node and can just select it.

    /// *If the operation argument matches*, what happens to the abstract graph?
    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> AbstractOperationOutput<Self::S>;

    // TODO: OperationOutput returned here should only represent Abstract changes. Basically the guaranteed new nodes so that other ops can refer to it.
    //  Maybe we could have something be returned in apply_abstract (just a Vec<SubstMarker>?) to indicate _which_ nodes are guaranteed to be added, and apply then returns a map with those substmarkers as keys?
    fn apply(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> OperationOutput;
}

/// Contains available operations
pub struct OperationContext<S: Semantics> {
    builtins: HashMap<OperationId, S::BuiltinOperation>,
    custom: HashMap<OperationId, UserDefinedOperation<S>>,
}

impl<S: Semantics> OperationContext<S> {
    pub fn new() -> Self {
        OperationContext {
            builtins: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    pub fn from_builtins(builtins: HashMap<OperationId, S::BuiltinOperation>) -> Self {
        OperationContext {
            builtins,
            custom: HashMap::new(),
        }
    }

    pub fn add_builtin_operation(&mut self, id: OperationId, op: S::BuiltinOperation) {
        self.builtins.insert(id, op);
    }

    pub fn add_custom_operation(&mut self, id: OperationId, op: UserDefinedOperation<S>) {
        self.custom.insert(id, op);
    }

    pub fn get(&self, id: OperationId) -> Option<Operation<S>> {
        if let Some(builtin) = self.builtins.get(&id) {
            return Some(Operation::Builtin(builtin));
        }
        if let Some(custom) = self.custom.get(&id) {
            return Some(Operation::Custom(custom));
        }
        None
    }
}

pub enum Operation<'a, S: Semantics> {
    Builtin(&'a S::BuiltinOperation),
    Custom(&'a UserDefinedOperation<S>),
}

impl<'a, S: SemanticsClone> Operation<'a, S> {
    pub fn parameter(&self) -> OperationParameter<S> {
        match self {
            Operation::Builtin(op) => op.parameter(),
            Operation::Custom(op) => op.parameter.clone(),
        }
    }

    pub fn apply_abstract(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> OperationResult<AbstractOperationOutput<S>> {
        match self {
            Operation::Builtin(op) => Ok(op.apply_abstract(g)),
            Operation::Custom(op) => op.apply_abstract(op_ctx, g),
        }
    }

    // TODO: support getting the signature from also a builtin operation?
}

// TODO: Builtin operations should be a trait that follows some generic pattern of mutating the graph
// also,

pub type OperationId = u32;

#[derive(Error, Debug, Clone)]
pub enum SubstitutionError {
    #[error("invalid operation argument count: expected {expected}, got {actual}")]
    InvalidOperationArgumentCount { expected: usize, actual: usize },
    #[error("operation argument does not match parameter")]
    ArgumentDoesNotMatchParameter,
}

/// Returns the pattern subst to input graph node key mapping, if the operation is applicable.
pub fn get_substitution<S: Semantics>(
    g: &AbstractGraph<S>,
    param: &OperationParameter<S>,
    selected_inputs: &[NodeKey],
) -> Result<ParameterSubstitution, SubstitutionError> {
    if param.explicit_input_nodes.len() != selected_inputs.len() {
        // TODO: decide if we want this to be actually reachable? Or if all preprocessing we do should catch this
        return Err(SubstitutionError::InvalidOperationArgumentCount {
            expected: param.explicit_input_nodes.len(),
            actual: selected_inputs.len(),
        });
    }

    // TODO: this won't work if the user selects the same node multiple times. We cannot have a subgraph where two nodes of the subgraph actually match to just a single one in the input graph.
    //  A fix might be to split the isomorphism finding to per-explicitly-selected node?

    // TODO: should we not return an error here?
    let enforced_param_to_arg_node_key_mapping = param
        .explicit_input_nodes
        .iter()
        .zip(selected_inputs.iter())
        .map(|((param_marker, argument_node_key))| {
            let param_node_key = param
                .subst_to_node_keys
                .get(param_marker)
                .expect("Invalid parameter marker");
            (*param_node_key, *argument_node_key)
        })
        .collect::<HashMap<_, _>>();

    let arg_ref = &g.graph;
    let param_ref = &param.parameter_graph.graph;

    let mut nm = |param_node: &NodeKey, arg_node: &NodeKey| {
        if let Some(expected_arg_node) = enforced_param_to_arg_node_key_mapping.get(param_node) {
            return expected_arg_node == arg_node;
        }
        let param_attr = param.parameter_graph.get_node_attr(*param_node).unwrap();
        let arg_attr = g.get_node_attr(*arg_node).unwrap();
        S::NodeMatcher::matches(arg_attr, &param_attr)
    };

    let mut em = |param_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>,
                  arg_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>| {
        let param_attr = &param_attr_wrapper.edge_attr;
        let arg_attr = &arg_attr_wrapper.edge_attr;
        S::EdgeMatcher::matches(arg_attr, &param_attr)
    };

    // TODO: error could indicate that this is not due to edge orderedness, but just general shape.
    let isos = general_subgraph_monomorphisms_iter(&param_ref, &arg_ref, &mut nm, &mut em)
        .ok_or(SubstitutionError::ArgumentDoesNotMatchParameter)?;

    isos.filter_map(|iso| {
        // TODO: handle edge orderedness

        let mapping = iso
            .iter()
            .enumerate()
            .map(|(param_idx, &arg_idx)| {
                let param_node_key = param_ref.from_index(param_idx);
                let arg_node_key = arg_ref.from_index(arg_idx);
                (
                    param.node_keys_to_subst[&param_node_key].clone(),
                    arg_node_key,
                )
            })
            .collect::<HashMap<_, _>>();

        Some(mapping)
    })
    .next()
    .map(ParameterSubstitution::new)
    .ok_or(SubstitutionError::ArgumentDoesNotMatchParameter)
}

pub fn run_operation<S: SemanticsClone>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op_ctx: &OperationContext<S>,
    op: OperationId,
    arg: OperationArgument,
) -> OperationResult<OperationOutput> {
    match op_ctx.get(op).expect("Invalid operation ID") {
        Operation::Builtin(builtin) => run_builtin_operation::<S>(g, builtin, arg),
        Operation::Custom(custom) => run_custom_operation::<S>(g, op_ctx, custom, arg),
    }
}

fn run_builtin_operation<S: SemanticsClone>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op: &S::BuiltinOperation,
    arg: OperationArgument,
) -> OperationResult<OperationOutput> {
    // can we run it?
    // let param = op.parameter();
    // let abstract_g = S::concrete_to_abstract(&g);
    // let subst = get_substitution(&abstract_g, &param, &selected_inputs)?;

    // TODO: we probably dont need to pass the OperationArgument down. Might just cause confusion.
    let mut gws = GraphWithSubstitution::new(g, &arg.subst);
    let output = op.apply(&mut gws);

    Ok(output)
}

fn run_custom_operation<S: SemanticsClone>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op_ctx: &OperationContext<S>,
    op: &UserDefinedOperation<S>,
    arg: OperationArgument,
) -> OperationResult<OperationOutput> {
    // can we run it?
    // let param = &op.parameter;
    // let abstract_g = S::concrete_to_abstract(&g);
    // let subst = get_substitution(&abstract_g, param, &selected_inputs)?;

    let output = op.apply(op_ctx, g, &arg.subst)?;

    Ok(output)
}

pub fn run_from_concrete<S: SemanticsClone>(
    g: &mut ConcreteGraph<S>,
    op_ctx: &OperationContext<S>,
    op: OperationId,
    selected_inputs: &[NodeKey],
) -> OperationResult<OperationOutput> {
    // first get substitution
    let abstract_g = S::concrete_to_abstract(g);

    let subst = match op_ctx
        .get(op)
        .ok_or(OperationError::InvalidOperationId(op))?
    {
        Operation::Builtin(builtin) => {
            let param = builtin.parameter();
            get_substitution(&abstract_g, &param, selected_inputs)?
        }
        Operation::Custom(custom) => {
            let param = &custom.parameter;
            get_substitution(&abstract_g, param, selected_inputs)?
        }
    };
    // then run the operation
    let arg = OperationArgument {
        subst,
        selected_input_nodes: selected_inputs.into(),
    };

    run_operation(g, op_ctx, op, arg)
}

pub type OperationResult<T> = std::result::Result<T, OperationError>;

// TODO: add specific source operation id or similar to the error
#[derive(Error, Debug, Clone)]
pub enum OperationError {
    #[error("operation {0} not found")]
    InvalidOperationId(OperationId),
    #[error("invalid operation argument count: expected {expected}, got {actual}")]
    InvalidOperationArgumentCount { expected: usize, actual: usize },
    #[error("operation argument does not match parameter")]
    ArgumentDoesNotMatchParameter,
    #[error("unknown parameter marker: {0:?}")]
    UnknownParameterMarker(SubstMarker),
    #[error("unknown operation result marker: {0:?}")]
    UnknownOperationResultMarker(AbstractOperationResultMarker),
    #[error("unknown output node marker: {0:?}")]
    UnknownOutputNodeMarker(AbstractOutputNodeMarker),
}

impl From<SubstitutionError> for OperationError {
    fn from(err: SubstitutionError) -> Self {
        match err {
            SubstitutionError::InvalidOperationArgumentCount { expected, actual } => {
                OperationError::InvalidOperationArgumentCount { expected, actual }
            }
            SubstitutionError::ArgumentDoesNotMatchParameter => {
                OperationError::ArgumentDoesNotMatchParameter
            }
        }
    }
}
