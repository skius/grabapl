use grabapl::{Semantics, TrueMatcher};

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
    
}