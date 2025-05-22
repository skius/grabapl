use std::collections::HashMap;
use std::rc::Rc;
use derive_more::with_trait::From;
use crate::graph::pattern::{AbstractOutputNodeMarker, OperationArgument, OperationOutput, OperationParameter, ParameterSubstition};
use crate::{NodeKey, OperationContext, OperationId, Semantics, SubstMarker};
use crate::graph::operation::run_operation;
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

// A 'custom'/user-defined operation
pub struct UserDefinedOperation<S: Semantics> {
    pub parameter: OperationParameter<S>,
    // TODO: add preprocessing (checking) step to see if the instructions make sense and are well formed wrt which nodes they access statically.
    pub instructions: Vec<(AbstractOperationResultMarker, Instruction<S>)>,
}

impl<S: SemanticsClone> UserDefinedOperation<S> {
    pub(crate) fn apply(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut ConcreteGraph<S>,
        argument: OperationArgument,
        subst: &ParameterSubstition,
    ) -> OperationOutput {
        let mut our_output_map: HashMap<AbstractOutputNodeMarker, NodeKey> = HashMap::new();

        let mut output_map: HashMap<AbstractOperationResultMarker, HashMap<AbstractOutputNodeMarker, NodeKey>> = HashMap::new();

        for (abstract_output_id, instruction) in &self.instructions {
            match instruction {
                Instruction::Operation(op_id, args) => {
                    let mut new_args = vec![];
                    for arg in args {
                        match arg {
                            AbstractNodeId::ParameterMarker(subst_marker) => {
                                new_args.push(subst.mapping[subst_marker]);
                            }
                            AbstractNodeId::DynamicOutputMarker(output_id, output_marker) => {
                                let output_map = output_map.get_mut(output_id).unwrap();
                                new_args.push(output_map[output_marker]);
                            }
                        }
                    }
                    // TODO: make fallible
                    // TODO: How do we support mutually recursive user defined operations?
                    let output = run_operation::<S>(
                        g,
                        op_ctx,
                        *op_id,
                        new_args,
                    ).unwrap();

                    output_map.insert(*abstract_output_id, output.new_nodes);
                }
                Instruction::Query(query) => {
                    todo!("implement query");
                }
            }
        }

        // TODO: How to define a good output here?
        //  probably should be part of the UserDefinedOperation struct. AbstractNodeId should be used, and then we get the actual node key based on what's happening.
        OperationOutput {
            new_nodes: our_output_map,
        }
    }
}


pub enum Instruction<S: Semantics> {
    Operation(OperationId, Vec<AbstractNodeId>),
    Query(Query<S>),
}

pub struct Query<S: Semantics> {
    taken: QueryTaken<S>,
    not_taken: Vec<Instruction<S>>,
}

// What happens when the query results in true.
//
// Analogy in Rust:
// ```
// if let Pattern(_) = query { block }
// ```
pub struct QueryTaken<S: Semantics> {
    // The pattern changes are applied to the abstract graph in sequence. Analogy: the "let Pattern" part
    pattern_changes: Vec<PatternChange<S>>,
    // With the new abstract graph, run these instructions. Analogy: the "block" part
    instructions: Vec<Instruction<S>>,
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