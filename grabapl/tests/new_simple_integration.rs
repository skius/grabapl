use grabapl::graph::operation::new_data_graph;
use grabapl::{DotCollector, Semantics, TrueMatcher};

struct SampleSemantics;

enum EdgePattern {
    Wildcard,
    Exact(String),
}

struct EdgeMatcher;
impl grabapl::PatternAttributeMatcher for EdgeMatcher {
    type Attr = String;
    type Pattern = EdgePattern;

    fn matches(attr: &Self::Attr, pattern: &Self::Pattern) -> bool {
        match pattern {
            EdgePattern::Wildcard => true,
            EdgePattern::Exact(p) => attr == p,
        }
    }
}

impl Semantics for SampleSemantics {
    type NodeAttribute = i32;
    type NodePattern = ();
    type EdgeAttribute = String;
    type EdgePattern = EdgePattern;
    type NodeAttributeMatcher = TrueMatcher<Self::NodeAttribute, Self::NodePattern>;
    type EdgeAttributeMatcher = EdgeMatcher;
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

    let mut g = new_data_graph::<SampleSemantics>();
    dot_collector.collect(&g);
    let a = g.add_node(1);
    let b = g.add_node(2);
    g.add_edge(a, b, "edge".to_string());
}
