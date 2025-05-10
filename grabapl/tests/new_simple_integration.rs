use grabapl::graph::semantics::{AbstractMatcher, AnyMatcher, Semantics};
use grabapl::{DotCollector};

struct SampleSemantics;

enum EdgePattern {
    Wildcard,
    Exact(String),
}

struct EdgeMatcher;
impl AbstractMatcher for EdgeMatcher {
    type Abstract = EdgePattern;
    fn matches(arg: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (arg, parameter) {
            (_, EdgePattern::Wildcard) => true,
            (EdgePattern::Exact(a), EdgePattern::Exact(b)) => a == b,
            (_, _) => false,
        }
    }
}

impl Semantics for SampleSemantics {
    type NodeConcrete = i32;
    type NodeAbstract = ();
    type EdgeConcrete = String;
    type EdgeAbstract = EdgePattern;
    type NodeMatcher = AnyMatcher<()>;
    type EdgeMatcher = EdgeMatcher;
    type BuiltinOperation = ();
}

enum BuiltinOperation {
    AddNode,
    AppendChild,
    /// Labels nodes of a three-cycle with 1,2,3, and requires the edge between 3 and 1 to be labelled "cycle"
    IndexCycle,
}

#[test]
fn test() {
    let mut dot_collector = DotCollector::new();

    let mut g = SampleSemantics::new_concrete_graph();
    dot_collector.collect(&g);
    let a = g.add_node(1);
    let b = g.add_node(2);
    g.add_edge(a, b, "edge".to_string());
}
