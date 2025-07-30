pub mod builder;
pub mod builtin;
pub mod marker;
pub mod query;
pub mod signature;
pub mod trace;
pub mod user_defined;

use crate::graph::EdgeAttribute;
use crate::operation::builtin::LibBuiltinOperation;
use crate::operation::marker::MarkerSet;
use crate::operation::signature::parameter::ConcreteOperationOutput;
use crate::operation::trace::Trace;
use crate::operation::user_defined::{
    AbstractNodeId, AbstractOperationResultMarker, UserDefinedOperation,
};
use crate::semantics::{
    AbstractGraph, AbstractMatcher, ConcreteGraph, ConcreteToAbstract, Semantics,
};
use crate::util::log;
use crate::{Graph, NodeKey, SubstMarker};
use error_stack::ResultExt;
use petgraph::algo::general_subgraph_monomorphisms_iter;
use petgraph::visit::NodeIndexable;
use serde::{Deserialize, Serialize};
use signature::parameter::{
    AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, OperationArgument,
    OperationOutput, OperationParameter, ParameterSubstitution,
};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use thiserror::Error;

pub trait BuiltinOperation: Debug {
    type S: Semantics;

    /// The pattern to match against the graph.
    // TODO: in theory we could have the apply_abstract function do what parameter is doing, if we wanted
    //  to provide clients with more power to match the abstract graph against their parameter with more freedom.
    fn parameter(&self) -> OperationParameter<Self::S>;

    /// *If the operation argument matches*, what happens to the abstract graph?
    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> AbstractOperationOutput<Self::S>;

    fn apply(
        &self,
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>,
        concrete_data: &mut ConcreteData,
    ) -> OperationOutput;
}

/// Additional, global data that can be used and modified by operations.
#[derive(Debug, Clone)]
pub struct ConcreteData<'a> {
    /// A set of markers that is natively used by shape queries.
    marker_set: &'a RefCell<MarkerSet>,
}

impl ConcreteData<'_> {
    pub fn marker_set_mut(&mut self) -> RefMut<MarkerSet> {
        // note: self could be taken as &self, but there shouldn't be a situation where we don't have a mutable reference to ConcreteData.
        self.marker_set.borrow_mut()
    }

    pub fn marker_set(&self) -> Ref<MarkerSet> {
        self.marker_set.borrow()
    }
}

/// Contains available operations
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound = "S: crate::serde::SemanticsSerde")
)]
pub struct OperationContext<S: Semantics> {
    builtins: HashMap<OperationId, S::BuiltinOperation>,
    libbuiltins: HashMap<OperationId, LibBuiltinOperation<S>>,
    custom: HashMap<OperationId, UserDefinedOperation<S>>,
}

impl<S: Semantics> OperationContext<S> {
    pub fn new() -> Self {
        OperationContext {
            builtins: HashMap::new(),
            libbuiltins: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    pub fn from_builtins(builtins: HashMap<OperationId, S::BuiltinOperation>) -> Self {
        OperationContext {
            builtins,
            libbuiltins: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    pub fn add_builtin_operation(&mut self, id: OperationId, op: S::BuiltinOperation) {
        self.builtins.insert(id, op);
    }

    pub fn add_lib_builtin_operation(&mut self, id: OperationId, op: LibBuiltinOperation<S>) {
        self.libbuiltins.insert(id, op);
    }

    pub fn add_custom_operation(&mut self, id: OperationId, op: UserDefinedOperation<S>) {
        self.custom.insert(id, op);
    }

    pub fn get(&self, id: OperationId) -> Option<Operation<S>> {
        if let Some(lib_builtin) = self.libbuiltins.get(&id) {
            return Some(Operation::LibBuiltin(lib_builtin));
        }
        if let Some(builtin) = self.builtins.get(&id) {
            return Some(Operation::Builtin(builtin));
        }
        if let Some(custom) = self.custom.get(&id) {
            return Some(Operation::Custom(custom));
        }
        None
    }
}

impl<S: Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>> Clone for OperationContext<S> {
    fn clone(&self) -> Self {
        OperationContext {
            builtins: self.builtins.clone(),
            libbuiltins: self.libbuiltins.clone(),
            custom: self.custom.clone(),
        }
    }
}

pub enum Operation<'a, S: Semantics> {
    Builtin(&'a S::BuiltinOperation),
    LibBuiltin(&'a LibBuiltinOperation<S>),
    Custom(&'a UserDefinedOperation<S>),
}

impl<'a, S: Semantics> Operation<'a, S> {
    pub fn parameter(&self) -> OperationParameter<S> {
        match self {
            Operation::Builtin(op) => op.parameter(),
            Operation::LibBuiltin(op) => op.parameter(),
            Operation::Custom(op) => op.signature.parameter.clone(),
        }
    }

    pub fn apply_abstract(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> OperationResult<AbstractOperationOutput<S>> {
        match self {
            Operation::Builtin(op) => Ok(op.apply_abstract(g)),
            Operation::LibBuiltin(op) => Ok(op.apply_abstract(g)),
            Operation::Custom(op) => op.apply_abstract(op_ctx, g),
        }
    }

    // TODO: support getting the signature from also a builtin operation?
}

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

    let return_arg_does_not_match_error_with_dbg_info = || {
        let shape_dbg_arg = g.shape_dot();
        let shape_dbg_param = param.parameter_graph.shape_dot();
        log::info!(
            "Failed to find substitution between parameter and argument graph:
shape of argument graph:\n{shape_dbg_arg},
shape of parameter graph:\n{shape_dbg_param},
args: {selected_inputs:?}"
        );
        SubstitutionError::ArgumentDoesNotMatchParameter
    };

    // TODO: this won't work if the user selects the same node multiple times. We cannot have a subgraph where two nodes of the subgraph actually match to just a single one in the input graph.
    //  A fix might be to split the isomorphism finding to per-explicitly-selected node?

    let enforced_param_to_arg_node_key_mapping = param
        .explicit_input_nodes
        .iter()
        .zip(selected_inputs.iter())
        .map(|(param_marker, argument_node_key)| {
            let param_node_key = param
                .node_keys_to_subst
                .get_right(param_marker)
                .expect("internal error: invalid parameter marker");
            (*param_node_key, *argument_node_key)
        })
        .collect::<HashMap<_, _>>();

    let arg_ref = &g.graph;
    let param_ref = &param.parameter_graph.graph;

    let mut nm = |param_node: &NodeKey, arg_node: &NodeKey| {
        if let Some(expected_arg_node) = enforced_param_to_arg_node_key_mapping.get(param_node)
            && expected_arg_node != arg_node
        {
            // early-exit if the node is in the enforced mapping, but does not match the argument node.
            return false;
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

    let isos = general_subgraph_monomorphisms_iter(&param_ref, &arg_ref, &mut nm, &mut em)
        .ok_or_else(return_arg_does_not_match_error_with_dbg_info)?;

    isos.filter_map(|iso| {
        // TODO: handle edge orderedness

        let mapping = iso
            .iter()
            .enumerate()
            .map(|(param_idx, &arg_idx)| {
                let param_node_key = param_ref.from_index(param_idx);
                let arg_node_key = arg_ref.from_index(arg_idx);
                (
                    // unwrap is ok since it was returned by param_ref.from_index
                    param
                        .node_keys_to_subst
                        .get_left(&param_node_key)
                        .unwrap()
                        .clone(),
                    arg_node_key,
                )
            })
            .collect::<HashMap<_, _>>();

        Some(mapping)
    })
    .next()
    .map(ParameterSubstitution::new)
    .ok_or_else(return_arg_does_not_match_error_with_dbg_info)
}

pub fn run_operation<S: Semantics>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op_ctx: &OperationContext<S>,
    op: OperationId,
    arg: OperationArgument<S>,
) -> OperationResult<OperationOutput> {
    match op_ctx.get(op).expect("Invalid operation ID") {
        Operation::LibBuiltin(lib_builtin) => run_lib_builtin_operation::<S>(g, lib_builtin, arg),
        Operation::Builtin(builtin) => run_builtin_operation::<S>(g, builtin, arg),
        Operation::Custom(custom) => run_custom_operation::<S>(g, op_ctx, custom, arg),
    }
}

fn run_lib_builtin_operation<S: Semantics>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op: &LibBuiltinOperation<S>,
    arg: OperationArgument<S>,
) -> OperationResult<OperationOutput> {
    run_builtin_or_lib_builtin_operation(g, op, arg)
}

fn run_builtin_operation<S: Semantics>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op: &S::BuiltinOperation,
    arg: OperationArgument<S>,
) -> OperationResult<OperationOutput> {
    run_builtin_or_lib_builtin_operation(g, op, arg)
}

fn run_builtin_or_lib_builtin_operation<S: Semantics, BO: BuiltinOperation<S = S>>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op: &BO, // LibBuiltin implements BuiltinOperation for any Semantics.
    arg: OperationArgument<S>,
) -> OperationResult<OperationOutput> {
    let mut gws = GraphWithSubstitution::new(g, &arg.subst);
    let mut concrete_data = ConcreteData {
        marker_set: arg.marker_set,
    };
    let output = op.apply(&mut gws, &mut concrete_data);

    Ok(output)
}

fn run_custom_operation<S: Semantics>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op_ctx: &OperationContext<S>,
    op: &UserDefinedOperation<S>,
    arg: OperationArgument<S>,
) -> OperationResult<OperationOutput> {
    let output = op.apply(op_ctx, g, arg)?;

    Ok(output)
}

pub fn run_from_concrete<S: Semantics>(
    g: &mut ConcreteGraph<S>,
    op_ctx: &OperationContext<S>,
    op: OperationId,
    selected_inputs: &[NodeKey],
) -> OperationResult<ConcreteOperationOutput<S>> {
    // first get substitution
    let abstract_g = S::concrete_to_abstract(g);

    let subst = match op_ctx
        .get(op)
        .ok_or(OperationError::InvalidOperationId(op))?
    {
        Operation::LibBuiltin(lib_builtin) => {
            let param = lib_builtin.parameter();
            get_substitution(&abstract_g, &param, selected_inputs)
                .change_context(OperationError::ArgumentDoesNotMatchParameter)?
        }
        Operation::Builtin(builtin) => {
            let param = builtin.parameter();
            get_substitution(&abstract_g, &param, selected_inputs)
                .change_context(OperationError::ArgumentDoesNotMatchParameter)?
        }
        Operation::Custom(custom) => {
            let param = &custom.signature.parameter;
            get_substitution(&abstract_g, param, selected_inputs)
                .change_context(OperationError::ArgumentDoesNotMatchParameter)?
        }
    };
    // then run the operation
    let marker_set = RefCell::new(MarkerSet::new());
    let trace = RefCell::new(Trace::new());
    let arg = OperationArgument {
        subst,
        selected_input_nodes: selected_inputs.into(),
        hidden_nodes: HashSet::new(),
        marker_set: &marker_set,
        trace: &trace,
    };

    let op_output = run_operation(g, op_ctx, op, arg)?;

    Ok(ConcreteOperationOutput {
        output: op_output,
        marker_set: marker_set.into_inner(),
        trace: trace.into_inner(),
    })
}

pub type OperationResult<T> = error_stack::Result<T, OperationError>;

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
    #[error("Unknown AID: {0:?}")]
    UnknownAID(AbstractNodeId),
    #[error("user crash: {0}")]
    UserCrash(String),
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
