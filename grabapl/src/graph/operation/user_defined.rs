use crate::graph::pattern::{OperationArgument, OperationParameter, ParameterSubstition};
use crate::{OperationId, Semantics, SubstMarker};
use crate::graph::semantics::ConcreteGraph;

// A 'custom'/user-defined operation
pub struct UserDefinedOperation<S: Semantics> {
    pub parameter: OperationParameter<S>,
    pub instructions: Vec<Instruction<S>>,
}

impl<S: Semantics> UserDefinedOperation<S> {
    pub(crate) fn apply(
        &self,
        g: &mut ConcreteGraph<S>,
        argument: OperationArgument,
        subst: &ParameterSubstition,
    ) {
        // TODO:
    }
}


pub enum Instruction<S: Semantics> {
    // TODO: add inputs
    Operation(OperationId),
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