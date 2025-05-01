// A 'custom'/user-defined operation
struct Function {
    instructions: Vec<Instruction>
}

type OperationId = u32;

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
    ExpectNode(NodePattern),
    ExpectEdge(EdgePattern),
}

enum NodePattern {
    NewNode,
}