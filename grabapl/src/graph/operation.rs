use crate::{Graph, NodeKey, SubstMarker};
use std::collections::HashMap;
use std::marker::PhantomData;
use petgraph::algo::general_subgraph_monomorphisms_iter;
use petgraph::visit::NodeIndexable;
use crate::graph::EdgeAttribute;
use crate::graph::pattern::{OperationArgument, OperationParameter, ParameterSubstition};
use crate::graph::semantics::{AbstractGraph, AbstractMatcher, ConcreteGraph, Semantics, SemanticsClone};

pub trait BuiltinOperation {
    type S: Semantics;

    /// The pattern to match against the graph.
    fn parameter(&self) -> OperationParameter<Self::S>;

    // TODO: needs an apply_abstract operation that applies the changes to the abstract graph.
    // For example, "add node" adds the node.
    // In general, we still need a way to refer to new changes, e.g., how do we refer
    // to a new node added by an operation?
    // In a frontend that's easy, the user 'sees' the node and can just select it.

    fn apply(
        &self,
        g: &mut ConcreteGraph<Self::S>,
        argument: OperationArgument,
        substitution: &ParameterSubstition,
    );
}

/// Contains available operations
pub struct OperationContext<B> {
    builtins: HashMap<OperationId, B>,
    custom: HashMap<OperationId, UserDefinedOperation>,
}

impl<B> OperationContext<B> {
    pub fn new() -> Self {
        OperationContext {
            builtins: HashMap::new(),
            custom: HashMap::new(),
        }
    }
    
    pub fn from_builtins(builtins: HashMap<OperationId, B>) -> Self {
        OperationContext {
            builtins,
            custom: HashMap::new(),
        }
    }

    pub fn add_builtin_operation(&mut self, id: OperationId, op: B) {
        self.builtins.insert(id, op);
    }

    pub fn add_custom_operation(&mut self, id: OperationId, op: UserDefinedOperation) {
        self.custom.insert(id, op);
    }

    pub fn get(&self, id: OperationId) -> Option<Operation<B>> {
        if let Some(builtin) = self.builtins.get(&id) {
            return Some(Operation::Builtin(builtin));
        }
        if let Some(custom) = self.custom.get(&id) {
            return Some(Operation::Custom(custom));
        }
        None
    }
}

enum Operation<'a, B> {
    Builtin(&'a B),
    Custom(&'a UserDefinedOperation),
}

// TODO: Builtin operations should be a trait that follows some generic pattern of mutating the graph
// also,

// A 'custom'/user-defined operation
struct UserDefinedOperation {
    instructions: Vec<Instruction>,
}

pub type OperationId = u32;

enum Instruction {
    // TODO: add inputs
    Operation(OperationId),
    Query(Query),
}

struct Query {
    taken: QueryTaken,
    not_taken: Vec<Instruction>,
}

// What happens when the query results in true.
//
// Analogy in Rust:
// ```
// if let Pattern(_) = query { block }
// ```
struct QueryTaken {
    // The pattern changes are applied to the abstract graph in sequence. Analogy: the "let Pattern" part
    pattern_changes: Vec<PatternChange>,
    // With the new abstract graph, run these instructions. Analogy: the "block" part
    instructions: Vec<Instruction>,
}

// These may refer to the original query input somehow.
// For example, we may have a "Has child?" query that:
//  1. ExpectNode(Child)
//  2. ExpectEdge(Parent, Child)
// But "Parent" is a free variable here, hence must somehow come from the query input. Unsure how yet.
enum PatternChange {
    ExpectNode(NodeChangePattern),
    ExpectEdge(EdgeChangePattern),
}

enum NodeChangePattern {
    // TODO: data to name the new node? And do we need a default node attr?
    NewNode,
}

enum EdgeChangePattern {
    // TODO: data to refer to which nodes get connected? And do we need a default edge attr?
    NewEdge,
}


/// Returns the pattern subst to input graph node key mapping, if the operation is applicable.
fn get_substitution<S: Semantics>(
    g: &AbstractGraph<S>,
    param: &OperationParameter<S>,
    selected_inputs: &[NodeKey],
) -> Option<ParameterSubstition> {
    if param.explicit_input_nodes.len() != selected_inputs.len() {
        return None;
    }

    let enforced_param_to_arg_node_key_mapping = param
        .explicit_input_nodes
        .iter()
        .zip(selected_inputs.iter())
        .map(|((param_marker, argument_node_key))| {
            let param_node_key = param.subst_to_node_keys.get(param_marker).expect("Invalid parameter marker");
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
        S::NodeMatcher::matches(arg_attr, &param_attr.value)
    };

    let mut em = |param_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>, arg_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>| {
        let param_attr = &param_attr_wrapper.edge_attr;
        let arg_attr = &arg_attr_wrapper.edge_attr;
        S::EdgeMatcher::matches(arg_attr, &param_attr)
    };

    let isos = general_subgraph_monomorphisms_iter(&param_ref, &arg_ref,
        &mut nm,
        &mut em,
    )?;

    isos.filter_map(|iso| {
        // TODO: handle edge orderedness
        
        let mapping = iso
            .iter()
            .enumerate()
            .map(|(param_idx, &arg_idx)| {
                let param_node_key = param_ref.from_index(param_idx);
                let arg_node_key = arg_ref.from_index(arg_idx);
                let param_subst = param.parameter_graph.get_node_attr(param_node_key).unwrap();
                (param_subst.marker, arg_node_key)
            })
            .collect::<HashMap<_, _>>();
        
        Some(mapping)
    }).next().map(ParameterSubstition::new)
}


pub fn run_operation<S: SemanticsClone>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op_ctx: &OperationContext<S::BuiltinOperation>,
    op: OperationId,
    selected_inputs: Vec<NodeKey>,
) -> Option<()> {
    match op_ctx.get(op).expect("Invalid operation ID") {
        Operation::Builtin(builtin) => {
            run_builtin_operation::<S>(g, builtin, selected_inputs)
        }
        Operation::Custom(custom) => {
            panic!("Custom operations not implemented yet")
        }
    }
}

fn run_builtin_operation<S: SemanticsClone>(
    g: &mut Graph<S::NodeConcrete, S::EdgeConcrete>,
    op: &S::BuiltinOperation,
    selected_inputs: Vec<NodeKey>,
) -> Option<()>
{
    // can we run it?
    let param = op.parameter();
    let abstract_g = S::concrete_to_abstract(&g);
    let subst = get_substitution(&abstract_g, &param, &selected_inputs)?;

    op.apply(g, OperationArgument { selected_input_nodes: selected_inputs }, &subst);

    Some(())
}
























