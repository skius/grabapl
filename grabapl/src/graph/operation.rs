use std::collections::HashMap;
use std::marker::PhantomData;
use crate::{Graph, PatternAttributeMatcher};

// TODO: move pattern matching around?

pub struct TrueMatcher<A, P> {
    phantom_data: PhantomData<(A, P)>,
}

impl<A, P> TrueMatcher<A, P> {
    pub fn new() -> Self {
        TrueMatcher {
            phantom_data: PhantomData,
        }
    }
}

impl<A, P> PatternAttributeMatcher for TrueMatcher<A, P> {
    type Attr = A;
    type Pattern = P;

    fn matches(_attr: &Self::Attr, _pattern: &Self::Pattern) -> bool {
        true
    }
}

/// Contains available operations
pub struct OperationContext<B> {
    builtins: HashMap<OperationId, B>,
    custom: HashMap<OperationId, UserDefinedOperation>,
}

/// Defines the semantics of a client implementation.
pub trait Semantics {
    /// A data graph's nodes contain values of this type.
    type NodeAttribute;
    /// An operation can define patterns for nodes using this type.
    type NodePattern;
    /// A data graph's edges contain values of this type.
    type EdgeAttribute;
    /// An operation can define patterns for edges using this type.
    type EdgePattern;
    /// The specific matching process for nodes.
    type NodeAttributeMatcher: PatternAttributeMatcher<Attr = Self::NodeAttribute, Pattern = Self::NodePattern>;
    /// The specific matching process for edges.
    type EdgeAttributeMatcher: PatternAttributeMatcher<Attr = Self::EdgeAttribute, Pattern = Self::EdgePattern>;

    /// Builtin operations are of this type.
    type BuiltinOperation;

}

pub fn new_data_graph<S: Semantics>() -> Graph<S::NodeAttribute, S::EdgeAttribute> {
    Graph::new()
}


enum Operation<B> {
    Builtin(B),
    Custom(UserDefinedOperation),
}

// TODO: Builtin operations should be a trait that follows some generic pattern of mutating the graph
// also, 



// A 'custom'/user-defined operation
struct UserDefinedOperation {
    instructions: Vec<Instruction>
}

pub type OperationId = u32;

enum Instruction {
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

